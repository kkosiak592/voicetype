# Technical Research: Sub-500ms Transcription Latency

## Strategic Summary

Your current pipeline spends ~300-800ms on Whisper large-v3-turbo GPU inference, ~30ms on VAD, and ~80ms on text injection — totaling ~1-2s after key release. The dominant bottleneck is Whisper's autoregressive decoder. Three genuinely different approaches exist: (1) swap to a non-autoregressive model like NVIDIA Parakeet TDT via the `parakeet-rs` Rust crate for ~10-50ms inference, (2) swap to distil-large-v3 in whisper.cpp for ~2x speedup with near-zero code changes, or (3) overlap inference with recording so transcription is partially done before the user releases the key. **Recommendation: Parakeet TDT via parakeet-rs for the largest single improvement, combined with injection timing reduction.**

## Requirements

- **Target**: <500ms from key release to text appearing in target app
- **Hardware**: NVIDIA GPU with CUDA
- **Current stack**: Tauri 2.0 + Rust backend, whisper.cpp via whisper-rs, cpal audio
- **Current model**: ggml-large-v3-turbo-q5_0.bin (GPU mode)
- **Audio format**: 16kHz mono f32, typical 1-10s clips
- **Language**: English only
- **Must remain**: Local/offline, no cloud APIs

## Current Latency Breakdown (measured from key release)

| Stage | Duration | Notes |
|-------|----------|-------|
| `flush_and_stop()` + buffer clone | ~1-5ms | Atomic flag + resampler flush |
| VAD gate check (Silero V5) | ~10-30ms | Post-hoc, full buffer scan |
| `ctx.create_state()` | ~5-20ms | Fresh WhisperState per call |
| `state.full(params, audio)` | ~300-800ms | **DOMINANT** — autoregressive decoder |
| Corrections regex | ~1-5ms | Negligible |
| `inject_text()` | ~80ms | 30ms clipboard + 50ms consume sleep |
| Pill UI events | ~5-10ms | Emit only, async |
| **Total** | **~400-950ms** | Best case GPU, short clip |

The variance comes from audio length — a 2s clip is faster than a 10s clip because Whisper processes proportionally.

---

## Approach 1: Switch to NVIDIA Parakeet TDT via `parakeet-rs`

**How it works:** Replace Whisper entirely with NVIDIA's Parakeet TDT 0.6B model, a FastConformer-based transducer that uses non-autoregressive Token-and-Duration Transducer (TDT) decoding. Unlike Whisper's 4-layer autoregressive decoder that generates tokens one at a time, Parakeet processes the entire encoder output and emits all tokens in a single forward pass. The `parakeet-rs` crate provides native Rust bindings via ONNX Runtime with CUDA execution provider.

**Libraries/tools:**
- `parakeet-rs = { version = "0.3", features = ["cuda"] }` — Rust crate on crates.io
- ONNX Runtime with CUDA EP (bundled by parakeet-rs)
- Model: `parakeet-tdt-0.6b-v2` or `v3` (ONNX format, ~600MB)
- Audio: Same 16kHz mono format you already produce

**Estimated inference time:** ~20-80ms for a 5s clip on NVIDIA GPU (RTFx >2,000 = processes 2,000 seconds of audio per second of compute). For your typical 1-10s dictation clips, inference would be near-instantaneous.

**Pros:**
- **Order of magnitude faster** — RTFx >2,000 vs Whisper's ~216. For a 5s clip: ~2.5ms theoretical, ~20-80ms practical (including data transfer, overhead)
- **Native Rust crate** — `parakeet-rs` on crates.io, same language as your Tauri backend
- **CUDA support built-in** — `features = ["cuda"]`, automatic fallback to CPU
- **Non-autoregressive** — inference time barely scales with audio length (encoder-dominant)
- **Token-level timestamps** — get word boundaries if needed later
- **600M parameters** — smaller than Whisper large-v3-turbo (809M), less VRAM

**Cons:**
- **Higher WER** — ~8% vs Whisper's ~7.4% on standard benchmarks (rank 23 on Open ASR leaderboard)
- **No initial_prompt mechanism** — Parakeet doesn't support vocabulary biasing via prompt (your corrections engine would need to compensate more)
- **ONNX model download** — need to convert or download pre-exported ONNX weights (~600MB)
- **Less battle-tested** — parakeet-rs is newer than whisper-rs, smaller community
- **Audio length limit** — CTC/TDT models cap at ~4-5 minutes (fine for your use case, max 60s)
- **No quantized models** — ONNX format, not ggml quantized; VRAM usage may be higher than q5_0

**Best when:** You want the absolute fastest inference and can tolerate slightly lower accuracy. The corrections engine can compensate for the ~0.6% WER gap.

**Complexity:** M — Replace transcribe.rs logic, add parakeet-rs dependency, handle ONNX model download, adapt pipeline. Audio capture and injection remain unchanged.

**Integration sketch:**
```rust
// Cargo.toml
parakeet-rs = { version = "0.3", features = ["cuda"] }

// transcribe.rs replacement
use parakeet_rs::{Transcriber, TranscriberConfig, ExecutionProvider};

let config = TranscriberConfig::new(model_path)
    .execution_provider(ExecutionProvider::Cuda);
let transcriber = Transcriber::new(config)?;
let result = transcriber.transcribe(&audio_samples)?; // non-blocking, fast
```

---

## Approach 2: Switch to `distil-large-v3` in whisper.cpp

**How it works:** Swap the model file from `ggml-large-v3-turbo-q5_0.bin` to `ggml-distil-large-v3.bin`. Distil-Whisper uses knowledge distillation to reduce the decoder from 32 layers (large-v3) or 4 layers (turbo) to just 2 layers, while keeping the full 32-layer encoder. This is a drop-in model swap — same whisper-rs API, same FullParams, same pipeline code.

**Libraries/tools:**
- Same `whisper-rs = "0.15"` — no dependency changes
- Model: `ggml-distil-large-v3.bin` from HuggingFace (distil-whisper/distil-large-v3-ggml)
- Quantized variants available (q5_0, q5_1, etc.)

**Estimated inference time:** ~150-400ms for a 5s clip (vs current ~300-800ms). The 2-layer decoder is ~50% faster than turbo's 4-layer decoder. Combined with quantization, expect ~1.5-2x speedup.

**Pros:**
- **Zero code changes** — literally swap the model file path in `resolve_model_path()`
- **Same API** — whisper-rs FullParams, WhisperContext, WhisperState all identical
- **Well-tested** — distil-large-v3 is widely used, battle-hardened
- **English accuracy** — within 1% WER of large-v3 (your app is English-only)
- **Smaller VRAM** — ~5GB vs ~8-10GB for large-v3-turbo
- **GGML quantized** — q5_0 available, further reduces memory and may speed up
- **Same corrections/prompt system** — initial_prompt still works

**Cons:**
- **Moderate speedup** — ~1.5-2x, may not be enough alone to hit <500ms consistently
- **English-only** — no multilingual (not a problem for your app)
- **Still autoregressive** — inference time still scales with audio length, just with fewer layers
- **Theoretical floor** — even with 2 decoder layers, Whisper's architecture has inherent per-token overhead

**Best when:** You want minimum risk, minimum code changes, and a meaningful speed improvement. Combine with Approach 4 (injection optimization) to stack gains.

**Complexity:** S — Change one filename string, download one model file, done.

---

## Approach 3: Overlapping Inference During Recording (Streaming Chunks)

**How it works:** Instead of waiting until the user releases the key to start transcription, begin processing audio chunks while the user is still speaking. The pipeline would:
1. Every ~1-2 seconds during recording, snapshot the current buffer
2. Run inference on the accumulated audio so far (speculatively)
3. When the user releases the key, only the final ~1-2s delta needs processing
4. Stitch the speculative result with the final chunk

This effectively hides inference latency behind recording time. For a 5s recording, 3-4s of audio is already transcribed before the key is released.

**Libraries/tools:**
- Same whisper-rs (or parakeet-rs) — model doesn't matter
- Custom async pipeline with buffer snapshotting
- Levenshtein-based stitching for overlapping segments (optional)

**Estimated perceived latency:** ~100-300ms after key release (only the final chunk + stitching). Actual total compute is higher, but hidden behind recording time.

**Pros:**
- **Dramatically reduces perceived latency** — most inference happens during recording
- **Works with any model** — Whisper, Parakeet, SenseVoice, anything
- **For longer recordings** — benefit increases with recording length (more is pre-transcribed)
- **Can produce live preview** — show partial transcription in pill overlay while recording

**Cons:**
- **Significant complexity** — buffer management, chunk boundaries, stitching, rollback on error
- **Wasted compute** — speculative chunks may be discarded if audio changes meaning
- **Whisper not designed for streaming** — chunk boundaries can cause word splitting, hallucination at edges
- **Doesn't help short recordings** — for 1-2s clips, there's no time to do speculative inference
- **Context management** — each chunk needs proper initial_prompt/context to avoid accuracy loss
- **Memory** — concurrent inference + recording increases GPU memory usage
- **Corrections timing** — corrections can only apply to final stitched result

**Best when:** Your typical recordings are 5+ seconds and you're willing to invest significant engineering effort. Pairs well with a fast model (Parakeet) for the final-chunk inference.

**Complexity:** L — New async pipeline architecture, buffer snapshotting, chunk stitching, error handling for partial results, testing edge cases.

---

## Approach 4: Pipeline Micro-Optimizations (Complementary)

**How it works:** Stack multiple small optimizations across the non-inference parts of the pipeline to shave 50-100ms. These are additive with any model swap.

**Optimizations:**

### 4a. Skip Post-Hoc VAD Gate (~20-30ms saved)
Currently `vad_gate_check()` runs Silero V5 over the entire buffer after recording stops. This is redundant when using hold-to-talk mode (user intentionally pressed and held the key). Replace with a minimum-length check (e.g., `samples.len() >= 4800` = 300ms at 16kHz) or trust the user's intent entirely.

```rust
// Before: full VAD scan (~20-30ms)
if !vad::vad_gate_check(&samples) { ... }

// After: simple length check (~0ms)
if samples.len() < 4800 { ... } // 300ms minimum
```

### 4b. Reduce Injection Sleeps (~35ms saved)
Current: 30ms clipboard propagation + 50ms paste consume = 80ms.
Aggressive: 15ms + 25ms = 40ms. Test empirically — Windows clipboard is fast on modern systems.

```rust
thread::sleep(Duration::from_millis(15)); // was 30
// ... Ctrl+V ...
thread::sleep(Duration::from_millis(25)); // was 50
```

### 4c. Reuse WhisperState (~5-15ms saved)
Currently creates a fresh `WhisperState` per transcription call. For sequential recordings with no concurrency, reusing the state avoids allocation overhead. Requires resetting LSTM state manually.

### 4d. Parallel VAD + Profile Read (~5ms saved)
Move the profile initial_prompt read to happen concurrently with audio flush, not sequentially after it.

### 4e. Pre-warm Clipboard (~5ms saved)
Create the Clipboard instance at app startup and keep it alive, avoiding per-call initialization in `inject_text()`.

**Total savings: ~50-85ms** when stacked.

**Pros:**
- Low risk, incremental changes
- Additive with any model swap
- Some are one-line changes

**Cons:**
- Won't solve the problem alone (inference is the bottleneck)
- Aggressive injection timing may cause paste drops in some apps
- Removing VAD gate increases hallucination risk (though hold-to-talk intent reduces this)

**Complexity:** S — Individual changes are small and independently testable.

---

## Approach 5: SenseVoice via sherpa-onnx

**How it works:** Replace Whisper with Alibaba's SenseVoice, a non-autoregressive speech recognition model that processes 10 seconds of audio in ~70ms (15x faster than Whisper Large). SenseVoice uses a single forward pass through a CTC-like architecture. Integration would be through sherpa-onnx (C/C++ library with ONNX Runtime).

**Libraries/tools:**
- `sherpa-onnx` — C/C++ library, would need Rust FFI bindings
- No existing Rust crate (unlike parakeet-rs)
- Model: SenseVoiceSmall (int8 quantized, ~100MB)

**Estimated inference time:** ~30-70ms for a 10s clip on GPU.

**Pros:**
- **Extremely fast** — 70ms for 10s audio, 15x faster than Whisper Large
- **Non-autoregressive** — constant-time regardless of audio length
- **Small model** — ~100MB quantized
- **Emotion detection** — bonus feature (SER)

**Cons:**
- **No Rust crate** — would need to write or generate FFI bindings to sherpa-onnx C API
- **Less accurate for English** — trained primarily on Chinese/multilingual data
- **No initial_prompt** — no vocabulary biasing mechanism
- **Smaller community** — fewer production deployments for English-only use
- **FFI complexity** — C interop in Tauri is doable but adds build complexity

**Best when:** You want maximum speed and are willing to build FFI bindings. Parakeet TDT is a better fit since it has native Rust bindings.

**Complexity:** L — FFI bindings, build system integration, model management.

---

## Comparison

| Aspect | Parakeet TDT | distil-large-v3 | Overlapping Inference | Micro-Optimizations | SenseVoice |
|--------|-------------|-----------------|----------------------|--------------------| -----------|
| Inference time (5s clip) | ~20-80ms | ~150-400ms | ~100-300ms perceived | Same as current | ~30-70ms |
| Total post-release latency | ~100-200ms | ~250-500ms | ~150-400ms | Current - 50-85ms | ~110-190ms |
| Code changes | M | S | L | S | L |
| Accuracy (WER) | ~8% | ~7.4% | Same as base | Same as current | ~8-10% |
| Risk | M | Low | High | Low | High |
| Rust integration | Native crate | Same whisper-rs | Same as base | Same | FFI needed |
| Vocabulary biasing | No prompt | initial_prompt | Same as base | Same | No |
| Model size | ~600MB | ~1.5GB (q5: ~500MB) | Same as base | Same | ~100MB |

## Recommendation

**Primary: Parakeet TDT via parakeet-rs (Approach 1) + Micro-Optimizations (Approach 4)**

This combination should deliver **~100-200ms total post-release latency**, well under your 500ms target:
- Inference: ~20-80ms (vs current ~300-800ms)
- Injection: ~40ms (vs current ~80ms, with reduced sleeps)
- VAD replaced with simple length check: ~0ms (vs current ~20-30ms)
- Buffer flush + overhead: ~5-10ms

The parakeet-rs crate gives you native Rust integration with CUDA support — same language as your Tauri backend, no FFI complexity. The ~0.6% WER gap vs Whisper is compensable by your corrections engine, especially for domain-specific structural engineering terminology.

**Fallback: distil-large-v3 (Approach 2) + Micro-Optimizations (Approach 4)**

If Parakeet integration proves problematic, swapping to distil-large-v3 is a one-line change that gets you ~1.5-2x faster inference within the existing whisper.cpp stack. Combined with micro-optimizations, this should hit ~250-450ms, borderline but close to your target.

**Future enhancement: Overlapping inference (Approach 3)** is the nuclear option if neither model swap achieves the target — but it's significantly more complex and only helps for longer recordings.

## Implementation Context

<claude_context>
<chosen_approach>
- name: NVIDIA Parakeet TDT via parakeet-rs + Pipeline Micro-Optimizations
- libraries:
  - parakeet-rs = { version = "0.3", features = ["cuda"] }
  - ort (ONNX Runtime, bundled by parakeet-rs)
- install:
  - Add to Cargo.toml: `parakeet-rs = { version = "0.3", features = ["cuda"] }`
  - Download model: parakeet-tdt-0.6b-v2 ONNX files from HuggingFace
  - Place in %APPDATA%/VoiceType/models/
</chosen_approach>
<architecture>
- pattern: Drop-in replacement of transcription engine
- components:
  1. New transcribe_parakeet.rs (replaces whisper inference call)
  2. Modified pipeline.rs (skip post-hoc VAD, use length check)
  3. Modified inject.rs (reduced sleep timings)
  4. Modified download.rs (add Parakeet model download)
  5. Modified lib.rs (init Parakeet instead of Whisper)
- data_flow:
  Audio buffer (16kHz mono f32)
    → Length check (>= 4800 samples)
    → parakeet_rs::Transcriber::transcribe(&samples)
    → Corrections engine (same as current)
    → inject_text() with reduced sleeps
</architecture>
<files>
- create:
  - src-tauri/src/transcribe_parakeet.rs (Parakeet inference wrapper)
- modify:
  - src-tauri/Cargo.toml (add parakeet-rs dependency)
  - src-tauri/src/pipeline.rs (replace VAD gate with length check, call Parakeet)
  - src-tauri/src/inject.rs (reduce sleep timings)
  - src-tauri/src/lib.rs (init Parakeet on startup, update managed state)
  - src-tauri/src/download.rs (add Parakeet ONNX model download)
- reference:
  - src-tauri/src/transcribe.rs (current Whisper wrapper — pattern to follow)
  - src-tauri/src/pipeline.rs (current pipeline orchestration)
</files>
<implementation>
- start_with: Benchmark current latency (add timing logs to pipeline.rs)
- order:
  1. Add timing instrumentation to pipeline.rs (measure each stage)
  2. Implement micro-optimizations (inject sleep reduction, VAD → length check)
  3. Add parakeet-rs dependency to Cargo.toml
  4. Create transcribe_parakeet.rs with Parakeet inference
  5. Wire Parakeet into pipeline.rs (behind feature flag initially)
  6. Add model download support
  7. Benchmark and compare A/B (Whisper vs Parakeet)
  8. If Parakeet wins: remove Whisper code path (or keep as fallback)
- gotchas:
  - parakeet-rs ONNX model format is different from ggml — separate download
  - ONNX Runtime CUDA EP requires matching CUDA toolkit version
  - parakeet-rs has no initial_prompt equivalent — corrections engine must compensate
  - First inference may be slow (ONNX Runtime graph optimization) — warm up at startup
  - Audio length limit ~4-5 min for TDT models (fine for your 60s max)
  - Test injection timing reduction empirically on target apps
- testing:
  - Add `Instant::now()` / `elapsed()` logging to each pipeline stage
  - Compare WER qualitatively on your typical dictation phrases
  - Test injection with reduced sleeps on: VS Code, Word, Notepad, Chrome
  - Benchmark cold start vs warm inference (first call vs subsequent)
</implementation>
</claude_context>

**Next Action:** Add timing instrumentation to the current pipeline, then prototype parakeet-rs integration in transcribe_parakeet.rs.

## Sources

- [whisper.cpp GitHub](https://github.com/ggml-org/whisper.cpp) — whisper.cpp repository
- [Faster-Whisper (CTranslate2)](https://github.com/SYSTRAN/faster-whisper) — 4x faster Whisper implementation
- [NVIDIA whisper_trt](https://github.com/NVIDIA-AI-IOT/whisper_trt) — TensorRT-optimized Whisper
- [Distil-Whisper](https://github.com/huggingface/distil-whisper) — 6x faster, 50% smaller, within 1% WER
- [distil-large-v3-ggml](https://huggingface.co/distil-whisper/distil-large-v3-ggml) — GGML format for whisper.cpp
- [parakeet-rs crate](https://crates.io/crates/parakeet-rs) — Rust bindings for NVIDIA Parakeet
- [parakeet-rs GitHub](https://github.com/altunenes/parakeet-rs) — Source code and docs
- [NVIDIA Parakeet TDT 0.6B v2](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v2) — Model weights
- [NVIDIA Parakeet TDT 0.6B v3](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3) — Multilingual model
- [SenseVoice](https://github.com/FunAudioLLM/SenseVoice) — 15x faster than Whisper Large
- [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) — Multi-model ONNX speech recognition
- [Whisper Large V3 Turbo](https://huggingface.co/openai/whisper-large-v3-turbo) — Current model info
- [Best STT Models 2026 Benchmarks](https://northflank.com/blog/best-open-source-speech-to-text-stt-model-in-2026-benchmarks) — Comparative benchmarks
- [Choosing Whisper Variants](https://modal.com/blog/choosing-whisper-variants) — Variant comparison
- [Whisper Speculative Decoding](https://huggingface.co/blog/whisper-speculative-decoding) — 2x speedup technique
- [Whisper Streaming](https://github.com/ufal/whisper_streaming) — Real-time streaming implementation
- [NVIDIA Speech AI Blog](https://developer.nvidia.com/blog/nvidia-speech-and-translation-ai-models-set-records-for-speed-and-accuracy/) — Parakeet performance data
