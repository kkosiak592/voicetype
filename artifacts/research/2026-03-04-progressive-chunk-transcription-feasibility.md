# Feasibility Assessment: Progressive Chunk Transcription During Recording

## Strategic Summary

Progressive chunk transcription is **feasible and architecturally sound** for all three engines (Whisper, Parakeet, Moonshine batch). The codebase already contains 80% of the required infrastructure (real-time VAD worker, VAD-based chunking, per-call state creation). The main concern is pipeline state machine complexity — the current 3-state AtomicU8 needs enrichment to represent "recording + partially transcribed." The key condition for success is that the latency gain only materializes for recordings > ~20 seconds with natural speech pauses; for short hold-to-talk clips, the overhead must be zero.

## What We're Assessing

**Proposed change:** Instead of the current batch pipeline (record -> stop -> transcribe entire buffer -> inject), progressively transcribe completed speech chunks during recording. When the VAD worker detects a silence gap >= 320ms during recording, extract the completed speech segment and dispatch it to a background transcription queue. By the time the user stops recording, all chunks except the last are already transcribed. Post-release latency drops from "full buffer inference time" to "last chunk inference time only."

**Scope:** All three engines (Whisper, Parakeet, Moonshine batch). Not streaming Moonshine models.

## Technical Feasibility

**Can we build it?**

### Known approaches: **Yes** — well-established

The approach (VAD-first segmentation + sequential dispatch overlapping with recording) is validated by multiple published systems:
- **WhisperX** (Interspeech 2023): VAD-chunked batched inference achieves 12x speedup with **no WER degradation** on TED-LIUM 3
- **whisper_streaming** (UFAL, EACL 2023): Buffer-and-confirm approach achieves 3.3s average latency on long-form speech
- **WhisperFlow** (MobiSys 2025): Per-word latency as low as 0.5s with negligible accuracy loss

### Technology maturity: **Proven**

Every component is well-understood:
- VAD-based silence detection: Already implemented in `vad.rs` (`spawn_vad_worker` + `vad_chunk_for_moonshine`)
- Sequential inference queue: Standard `tokio::sync::mpsc` channel pattern
- Chunk boundary handling: VAD split at silence gaps eliminates mid-word cuts (proven by WhisperX)
- Context chaining: Whisper's `initial_prompt` can carry previous chunk's output to maintain coherence

### Existing codebase alignment: **Strong**

| Component | Status | Gap |
|-----------|--------|-----|
| Real-time VAD during recording | `spawn_vad_worker()` in `vad.rs:308` | Needs chunk boundary emission |
| VAD silence-based chunking | `vad_chunk_for_moonshine()` in `vad.rs:166` | Algorithm exists; needs real-time variant |
| Shared audio buffer | `AudioCaptureMutex` with cursor-based reads | Needs chunk extraction without stopping recording |
| Whisper per-call state | `ctx.create_state()` in `transcribe.rs:274` | Ready — shares model, fresh state per chunk |
| Parakeet sequential inference | `Arc<Mutex<ParakeetTDT>>` | Ready — mutex serializes naturally |
| Moonshine sequential inference | `Arc<Mutex<MoonshineEngine>>` | Ready — already does VAD chunking for >30s |
| Pipeline state machine | `AtomicU8` with 3 states | **Needs enrichment** — new "recording + processing" state |

### Technical risks

| Risk | Severity | Details |
|------|----------|---------|
| Pipeline state machine complexity | **Medium** | Current 3-state (Idle/Recording/Processing) AtomicU8 needs a 4th state or richer representation to handle "recording while chunks are being transcribed." Double-trigger prevention and error recovery paths all need auditing. |
| Shared buffer extraction during recording | **Medium** | Currently `flush_and_stop()` takes the entire buffer at once. Progressive dispatch needs to extract completed chunks from the live buffer without stopping recording. The buffer is `Arc<Mutex<Vec<f32>>>` — extraction requires careful cursor management to avoid copying active audio. |
| Chunk boundary quality for Parakeet | **Low** | Parakeet currently processes the entire buffer in one call. It has no chunking logic. VAD-based splitting should work (validated for Whisper), but Parakeet's Token-and-Duration Transducer architecture may behave differently at chunk boundaries. Needs validation. |
| Context loss between chunks (Whisper) | **Low** | Whisper uses `single_segment(true)` currently. For progressive chunks that are complete VAD-bounded utterances, this is fine. Context chaining via `initial_prompt` (previous chunk's output) mitigates cross-chunk coherence issues. |
| Hold-to-talk overhead | **Low** | Progressive dispatch must add zero overhead for short hold-to-talk clips. This is achievable with a simple duration gate (only enable progressive mode for recordings > N seconds or when running in toggle mode). |
| GPU serialization misconception | **Low-Info** | Parallel chunk transcription on a single GPU provides zero throughput gain (confirmed by faster-whisper benchmarks: two concurrent jobs on a T4 each take 2x as long). The entire gain comes from overlapping inference with recording time, not from parallel inference. This is architecturally correct but may confuse future maintainers. |

### Technical verdict: **Feasible**

The architecture is sound, the approach is proven, and the codebase has most building blocks. Complexity is concentrated in the state machine refactor and buffer extraction — both are tractable engineering problems, not research problems.

## Resource Feasibility

**Do we have what we need?**

### Skills: **Have**

- Rust async + tokio channels: already used throughout the codebase (VAD worker, pipeline)
- CPAL audio buffer management: already implemented in `audio.rs`
- whisper-rs / parakeet-rs / transcribe-rs APIs: already integrated
- AtomicU8 CAS state machines: already the core pipeline pattern

### Development effort: **Medium**

Estimated scope of changes:

| File | Change | Size |
|------|--------|------|
| `vad.rs` | Add `spawn_progressive_vad_worker()` that emits chunk boundary events via `tokio::sync::mpsc` | ~80 lines new |
| `pipeline.rs` | Add progressive pipeline variant alongside existing batch pipeline. New state management for recording+processing overlap. | ~120 lines new, ~30 lines modified |
| `transcribe.rs` | Add context-chaining support (accept optional `initial_prompt` from previous chunk output) | ~10 lines modified |
| `transcribe_parakeet.rs` | Add VAD chunking support (similar to Moonshine's existing pattern) | ~30 lines new |
| `transcribe_moonshine.rs` | Minor — already does chunking; adapt to receive pre-split chunks | ~10 lines modified |
| `audio.rs` | Add `extract_chunk(start, end)` method that copies a range without stopping recording | ~15 lines new |

Total: ~250-300 lines of new/modified code.

### Testing effort: **Medium-High**

This is the primary resource concern:
- **Benchmark validation**: Must prove no WER regression at chunk boundaries for all three engines across 5s/30s/60s/90s clips
- **Latency benchmarks**: Must measure actual post-release latency improvement to validate the theoretical gains
- **Edge cases**: Short recordings with no silence gaps, very long silence gaps, single-chunk recordings, rapid start/stop
- **State machine correctness**: CAS transitions with 4+ states need thorough testing for race conditions

### Resource verdict: **Feasible**

Development effort is moderate. Testing is the main investment — proof-of-concept benchmarks are required per user's high-confidence requirement.

## External Dependency Feasibility

**Are external factors reliable?**

### APIs/services: **N/A** — fully local, no external dependencies

### Third-party libraries: **Stable**

| Library | Version | Concern |
|---------|---------|---------|
| whisper-rs | 0.15 | `create_state()` is public and documented — safe to call per-chunk |
| parakeet-rs | 0.1.9 | `transcribe_samples()` takes `&mut self` — Mutex serialization handles this |
| transcribe-rs | 0.2.8 | `transcribe_samples()` takes `&mut self` — same pattern |
| voice_activity_detector | (Silero V5) | Stable, already used for three different VAD operations |
| tokio | (async runtime) | `mpsc` channels are battle-tested |

### External verdict: **Feasible** — no external dependency risk

## Blockers

| Blocker | Severity | Mitigation |
|---------|----------|------------|
| No proof-of-concept latency benchmarks exist | **High** | Build a minimal PoC that VAD-chunks a 60s WAV during simulated recording and measures actual per-chunk inference times. Can reuse existing benchmark binary infrastructure. |
| Parakeet has no chunking support | **Medium** | Implement `vad_chunk_for_parakeet()` following the Moonshine pattern. Parakeet has no known context window limit, but VAD-based chunking should work for progressive dispatch. Validate WER against batch baseline. |
| Pipeline state machine needs redesign | **Medium** | Current AtomicU8 works well for 3 states. Adding a 4th "RecordingAndProcessing" state or switching to a richer representation (e.g., bitfield or struct with separate recording/processing flags) requires auditing all state transitions in `pipeline.rs`, `vad.rs`, and `lib.rs` hotkey handlers. |
| 60s recording safety cap | **Low** | Current `MAX_RECORDING_FRAMES = 1875` (60s) limits toggle mode. Progressive dispatch reduces back-pressure, so this cap could be raised. But raising it requires validating memory usage for longer recordings. |

## De-risking Options

1. **PoC benchmark first (recommended)**: Before touching the main pipeline, build a standalone benchmark that:
   - Takes a 60s/90s WAV
   - Simulates progressive VAD chunking (run VAD, split at silence boundaries as they're discovered)
   - Times each chunk's inference independently for all three engines
   - Compares total post-last-chunk latency vs batch-all latency
   - Validates WER per engine against batch baseline
   - **Cost**: ~1-2 hours of development using existing `benchmark.rs` infrastructure
   - **Risk reduced**: Eliminates the "does it actually improve latency?" and "does WER degrade?" unknowns

2. **Toggle-mode only first**: Implement progressive dispatch only for toggle mode (which already has a VAD worker). Hold-to-talk remains unchanged (batch pipeline). This halves the state machine complexity since hold-to-talk's simple release → process flow doesn't change.
   - **Cost**: Reduces scope by ~30%
   - **Risk reduced**: No risk of regressing the more latency-sensitive hold-to-talk path

3. **Duration gate**: Only activate progressive dispatch for recordings > 20 seconds. Below that threshold, use the existing batch pipeline. This ensures zero overhead for typical hold-to-talk usage.
   - **Cost**: Trivial — one `if` statement
   - **Risk reduced**: Eliminates short-clip regression risk entirely

## Overall Verdict

**Go with conditions**

The optimization is technically sound, architecturally well-aligned with the existing codebase, and addresses a real user pain point (5-10+ second wait after long toggle-mode recordings). The codebase already has most of the required infrastructure.

**Conditions:**

1. **PoC benchmark must validate** that progressive dispatch reduces post-release latency by >= 2x for 60s recordings AND introduces no measurable WER regression for any of the three engines
2. **Toggle-mode first** — hold-to-talk remains batch until progressive dispatch is proven stable
3. **Duration gate** — progressive dispatch only activates for recordings > 20 seconds

If the PoC benchmark shows < 1.5x improvement or measurable WER regression, **no-go** — the complexity isn't worth it.

## Implementation Context

### If go:
- **Approach**: VAD-driven progressive dispatch with sequential transcription queue
- **Start with**: Standalone PoC benchmark validating latency gain and WER stability for all three engines
- **Critical path**: The VAD worker's ability to accurately detect chunk boundaries during live recording and the buffer extraction mechanism that copies completed chunks without stopping the audio stream

### Risks:
- **Technical**: State machine refactor introduces regression risk in the pipeline's most critical code path. Mitigate with exhaustive state transition tests.
- **External**: None — fully local.
- **Mitigation**: PoC benchmark first, toggle-mode only, duration gate

### Alternatives:
- **If blocked** (state machine too complex): Use `whisper_full_parallel` post-recording for Whisper-only speedup (2-3x on CPU with N=2 processors). This avoids all pipeline changes but only helps Whisper on CPU.
- **Simpler version**: Only implement for Moonshine (which already has VAD chunking). Extend Moonshine's existing `vad_chunk_for_moonshine` to run during recording instead of after. This is the minimal viable progressive pipeline with the least risk.

## Expected Latency Improvement (Theoretical)

| Recording | Engine | Batch (current) | Progressive (expected) | Gain |
|-----------|--------|-----------------|----------------------|------|
| 30s, 2 chunks | Whisper large-v3-turbo (GPU) | ~3s | ~1.5s | 2x |
| 60s, 4 chunks | Whisper large-v3-turbo (GPU) | ~6s | ~1.5s | 4x |
| 90s, 6 chunks | Whisper large-v3-turbo (GPU) | ~8s | ~1.5s | 5x+ |
| 60s, 4 chunks | Whisper small.en (CPU) | ~6s | ~2s | 3x |
| 90s, 6 chunks | Whisper small.en (CPU) | ~10s | ~2s | 5x |
| 60s, 4 chunks | Parakeet (GPU) | ~4.6s | ~1.2s | 4x |
| 60s, 4 chunks | Moonshine Tiny (GPU) | ~0.8s | ~0.2s | 4x* |

*Moonshine is already fast enough that absolute gain is minimal (~600ms savings). Progressive dispatch matters most for Whisper.

## When It's NOT Worth It

- **Hold-to-talk < 10s**: No time for chunk boundaries. Zero gain, nonzero overhead.
- **Moonshine on any length**: Already sub-second inference for 60s clips. Progressive dispatch saves < 1s.
- **Any engine on < 20s clips**: Batch inference is already fast enough that progressive overhead isn't justified.

## Sources

- [WhisperX paper (Interspeech 2023)](https://www.isca-archive.org/interspeech_2023/bain23_interspeech.pdf) — VAD-chunked batched inference, no WER loss
- [whisper_streaming (UFAL)](https://github.com/ufal/whisper_streaming) — Buffer-and-confirm streaming approach
- [WhisperFlow (arXiv 2412.11272)](https://arxiv.org/abs/2412.11272) — Hush word + beam pruning for 0.5s per-word latency
- [Simul-Whisper (INTERSPEECH 2024)](https://arxiv.org/abs/2406.10052) — Cross-attention alignment for chunk-based streaming
- [whisper.cpp parallel processing](https://github.com/ggml-org/whisper.cpp/issues/1408) — CPU parallel scaling limits
- [faster-whisper GPU parallel degradation](https://github.com/guillaumekln/faster-whisper/issues/441) — Single GPU serialization proof
- [HuggingFace chunking discussion](https://huggingface.co/openai/whisper-large-v2/discussions/67) — Chunk size + overlap recommendations
- [Northflank STT benchmarks 2026](https://northflank.com/blog/best-open-source-speech-to-text-stt-model-in-2026-benchmarks) — Current model performance comparisons
- Codebase files: `vad.rs`, `pipeline.rs`, `transcribe.rs`, `transcribe_parakeet.rs`, `transcribe_moonshine.rs`, `audio.rs`

---

*Assessment date: 2026-03-04*
*Next action: Build PoC benchmark to validate latency gain and WER stability across all three engines*
