# Technical Research: Pipeline Latency Optimization

## Strategic Summary

Your 1-2 second latency breaks down to ~600-1000ms whisper inference (large-v3-turbo with beam_size=5) plus ~200ms fixed injection delays. Three independent optimizations can stack to cut total latency by 50-70%: (1) switch beam search to greedy decoding (~30-50% inference speedup, one line change), (2) enable flash attention + tune whisper parameters (~10-20% additional inference speedup), and (3) cut injection sleep times (~115ms savings). Combined, these should bring you from 1-2s down to 400-700ms without changing models. If you want to go further, switching to distil-large-v3 could cut inference time by another 3-5x.

---

## Current Latency Breakdown

| Pipeline Step | Estimated Time | Code Location |
|---|---|---|
| `flush_and_stop()` + `get_buffer()` | ~2-5ms | `pipeline.rs:50-53` |
| Audio gate (< 1600 samples check) | ~0ms | `pipeline.rs:55` |
| **Whisper inference** (large-v3-turbo, beam=5) | **~600-1000ms** | `transcribe.rs:140-178` |
| Text formatting | ~0ms | `pipeline.rs:127-135` |
| **inject_text** (75ms + Ctrl+V + 120ms) | **~200ms** | `inject.rs:29,39` |
| Pipeline overhead (events, tray) | ~5ms | `pipeline.rs:152-170` |
| **Total** | **~800-1200ms** | |

The two dominant costs are whisper inference and injection delays. VAD gate is NOT called in the current hold-to-talk pipeline, so there's no overhead there.

---

## Approach 1: Whisper Parameter Tuning (No Model Change)

**How it works:** Change inference parameters in `transcribe.rs` to trade unnecessary computation for speed. The current config uses beam search with 5 beams, which is overkill for short English dictation.

**Changes:**

### 1a. Switch to greedy decoding
```rust
// BEFORE (transcribe.rs:145-148)
let mut params = FullParams::new(SamplingStrategy::BeamSearch {
    beam_size: 5,
    patience: -1.0,
});

// AFTER
let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
```

Beam search with beam_size=5 evaluates 5 hypotheses at each decoder step. For short dictation phrases (1-15 words), greedy search produces virtually identical output. whisper.cpp itself defaults to greedy. A beam_size of 2 is 2.35x faster than beam_size=5; greedy is faster still.

**Expected speedup:** 30-50% of inference time (~200-400ms saved)

### 1b. Enable flash attention
```rust
// BEFORE (transcribe.rs:118-119)
let mut ctx_params = WhisperContextParameters::default();
ctx_params.use_gpu(use_gpu);

// AFTER
let mut ctx_params = WhisperContextParameters::default();
ctx_params.use_gpu(use_gpu);
ctx_params.flash_attn(true);
```

Flash attention reduces memory operations during self-attention computation. Available in whisper-rs via `WhisperContextParameters::flash_attn(true)`. Note: incompatible with DTW timestamps (which you don't use).

**Expected speedup:** 10-20% additional inference speedup

### 1c. Disable temperature fallback
```rust
// Add after existing params (transcribe.rs, after line 154)
params.set_temperature_inc(0.0); // disable fallback decoding with higher temps
```

By default, whisper retries with progressively higher temperatures if the initial decode has low confidence. Setting `temperature_inc` to 0.0 prevents these retries. Since you're already at temp=0.0 and doing short dictation, fallback almost never improves results but can add latency.

### 1d. Force single segment
```rust
params.set_single_segment(true); // short dictation = one segment
params.set_no_context(true);     // no prior context needed
```

For hold-to-talk dictation (typically 1-10 seconds), there's almost always a single segment. This tells whisper to not search for segment boundaries.

**Pros:**
- Zero risk of accuracy loss for typical dictation
- All changes are in `transcribe.rs` only
- Beam search removal alone is the single highest-impact change
- Fully reversible (just revert the parameters)

**Cons:**
- Beam search removal could theoretically hurt accuracy on very noisy audio
- Flash attention behavior may vary by GPU architecture (P2000 is Pascal/compute 6.1)

**Best when:** You want maximum speed gain with minimum code change

**Complexity:** S

---

## Approach 2: Switch to distil-large-v3 Model

**How it works:** Replace `ggml-large-v3-turbo-q5_0.bin` with `ggml-distil-large-v3.bin`. The distilled model has only 2 decoder layers (vs 32 in the original), maintaining ~99.2% of large-v3's accuracy while being ~5x faster on inference.

**Changes:**
- Download `ggml-distil-large-v3.bin` from [HuggingFace](https://huggingface.co/distil-whisper/distil-large-v3-ggml)
- Update `resolve_model_path()` in `transcribe.rs` to reference the new filename

```rust
ModelMode::Gpu => (
    "ggml-distil-large-v3.bin",
    "https://huggingface.co/distil-whisper/distil-large-v3-ggml/resolve/main/ggml-distil-large-v3.bin",
),
```

**Performance:**
- distil-large-v3 is ~5.8x faster than large-v3 on long-form audio
- For short clips (1-10s), the speedup is still significant (3-5x) due to fewer decoder layers
- WER difference: within 0.8% of large-v3 on standard benchmarks
- Combined with greedy decoding: inference could drop to ~100-300ms

**Quantized variants also available:**
- `ggml-distil-large-v3-q5_0.bin` — smaller download, similar speed
- Pre-quantized models available on HuggingFace

**Pros:**
- Dramatic inference speedup (3-5x)
- Negligible accuracy loss for English dictation
- Same whisper.cpp/whisper-rs API — just a different model file
- Can stack with Approach 1 parameter tuning

**Cons:**
- Requires downloading a new model file (~1.5GB unquantized, ~500MB quantized)
- Distil models are English-optimized (fine for your use case)
- Less well-tested than the standard large-v3-turbo in whisper.cpp
- Your current large-v3-turbo-q5_0 was specifically chosen; need to verify quality

**Best when:** Greedy decoding alone isn't fast enough and you want sub-500ms inference

**Complexity:** S

---

## Approach 3: Reduce Injection Delays

**How it works:** The `inject_text()` function in `inject.rs` has 195ms of deliberate `thread::sleep()` calls that can be reduced.

**Current delays:**
```
75ms — clipboard propagation (line 29)
120ms — target app paste consumption (line 39)
Total: 195ms fixed overhead
```

**Aggressive reduction:**
```rust
// inject.rs
thread::sleep(Duration::from_millis(30));  // was 75ms — clipboard propagation
// ... Ctrl+V simulation ...
thread::sleep(Duration::from_millis(50));  // was 120ms — app consumption
// Total: 80ms (saves ~115ms)
```

**Very aggressive reduction:**
```rust
thread::sleep(Duration::from_millis(15));  // clipboard propagation
// ... Ctrl+V simulation ...
thread::sleep(Duration::from_millis(30));  // app consumption
// Total: 45ms (saves ~150ms)
```

**Pros:**
- Saves 100-150ms with no change to inference quality
- Simple to test — if paste fails in any app, increase the delays back

**Cons:**
- Risk: some apps (Chrome, VS Code, Outlook) may not receive the paste if delays are too short
- Windows clipboard propagation has documented variability
- The 75ms was chosen as midpoint of a 50-100ms "safe range"

**Recommended:** Start with 30ms + 50ms, test in your daily apps (VS Code, Chrome, Outlook, Teams, AutoCAD). If any app drops pastes, bump the first delay back up to 50ms.

**Best when:** You want easy wins without touching the inference engine

**Complexity:** S

---

## Approach 4: Combined (All of the Above)

**How it works:** Stack all three approaches for maximum impact.

**Expected result:**

| Step | Before | After (Combined) |
|---|---|---|
| Whisper inference | 600-1000ms | 100-300ms (greedy + flash_attn + distil model) |
| Injection delays | 195ms | 80ms (30ms + 50ms) |
| Other overhead | ~10ms | ~10ms |
| **Total** | **800-1200ms** | **190-390ms** |

**Implementation order (progressive — test after each):**
1. Switch to greedy decoding (1 line change, test immediately)
2. Enable flash attention (1 line change, test)
3. Add single_segment + no_context + temperature_inc=0.0 (3 lines)
4. Reduce injection delays to 30ms + 50ms (2 line changes, test in target apps)
5. If still not fast enough: swap model to distil-large-v3

**Pros:**
- Each step is independently testable and reversible
- Progressive approach lets you find the sweet spot
- Could achieve sub-400ms total latency

**Cons:**
- Model swap (step 5) requires downloading a new file
- Need to test injection delays in all your target apps

**Best when:** You want the absolute fastest possible response time

**Complexity:** S-M (individual changes are S, combined testing is M)

---

## Comparison

| Aspect | Approach 1: Param Tuning | Approach 2: Distil Model | Approach 3: Injection Delays | Approach 4: Combined |
|--------|--------------------------|--------------------------|------------------------------|---------------------|
| Complexity | S | S | S | S-M |
| Inference speedup | 30-50% | 300-500% | 0% | 300-500% |
| Injection speedup | 0ms | 0ms | 115-150ms | 115-150ms |
| Accuracy risk | Negligible | ~0.8% WER increase | None | ~0.8% WER increase |
| Code changes | 3-5 lines in transcribe.rs | 2 lines + model download | 2 lines in inject.rs | All of the above |
| Reversibility | Instant | Swap model file back | Instant | Instant |

---

## Recommendation

**Start with Approach 1 (parameter tuning) — it's the best bang for buck.**

The greedy decoding switch alone should cut your 1-2s down to ~600-900ms with zero accuracy risk. Add flash attention and the other parameter tweaks to squeeze out another 10-20%.

Then reduce injection delays (Approach 3) for another ~115ms savings.

If you're still not satisfied after that, the distil-large-v3 model swap (Approach 2) will get you to sub-400ms territory.

The key insight: **beam_size=5 is the single largest contributor to unnecessary latency**. whisper.cpp itself defaults to greedy for a reason — beam search on short dictation clips provides almost zero accuracy benefit at 2-3x the compute cost.

---

## Implementation Context

<claude_context>
<chosen_approach>
- name: Progressive parameter tuning + injection delay reduction
- libraries: whisper-rs 0.15 (existing — no new dependencies)
- install: no new dependencies needed
</chosen_approach>
<architecture>
- pattern: Tuning existing pipeline parameters, no architectural changes
- components: transcribe.rs (inference params), inject.rs (delay reduction), optionally model swap
- data_flow: Same pipeline, faster at each step
</architecture>
<files>
- modify:
  - src-tauri/src/transcribe.rs — switch to greedy, enable flash_attn, add single_segment/no_context/temp_inc
  - src-tauri/src/inject.rs — reduce sleep durations
  - (optional) src-tauri/src/transcribe.rs — change model filename for distil-large-v3
- structure: No new files needed
- reference: Current transcribe.rs and inject.rs
</files>
<implementation>
- start_with: Change SamplingStrategy::BeamSearch to SamplingStrategy::Greedy in transcribe.rs
- order:
  1. Switch to greedy decoding (transcribe.rs:145-148)
  2. Enable flash attention (transcribe.rs:118-119)
  3. Add single_segment, no_context, temperature_inc=0.0 (transcribe.rs after line 154)
  4. Reduce injection delays to 30ms + 50ms (inject.rs:29,39)
  5. Test in target apps (VS Code, Chrome, Outlook, Teams)
  6. If still too slow: download distil-large-v3 model, update resolve_model_path()
- gotchas:
  - Flash attention may not work or may be slower on Pascal GPUs (P2000, compute 6.1) — test and revert if needed
  - Injection delay reduction needs app-by-app testing
  - distil-large-v3 GGML model may not be quantized by default — look for q5_0 variant
  - Monitor the "Transcription completed in Xms" log line to measure actual improvement
- testing:
  - Before any changes: note current "Transcription completed in Xms" log output for baseline
  - After each change: compare inference time in logs
  - Test injection in: VS Code, Chrome, Outlook, Teams, Notepad, AutoCAD
  - Use the same ~3-5 second test phrase each time for consistent comparison
</implementation>
</claude_context>

---

**Next Action:** Apply Approach 1 (greedy + flash_attn + parameter tuning) as a single commit, measure before/after in logs.

---

## Sources

- [whisper.cpp GitHub](https://github.com/ggml-org/whisper.cpp) — reference implementation
- [whisper-rs docs (FullParams)](https://docs.rs/whisper-rs/latest/whisper_rs/struct.FullParams.html) — parameter API
- [whisper-rs docs (WhisperContextParameters)](https://docs.rs/whisper-rs/latest/whisper_rs/struct.WhisperContextParameters.html) — flash_attn API
- [distil-whisper/distil-large-v3-ggml on HuggingFace](https://huggingface.co/distil-whisper/distil-large-v3-ggml) — distilled GGML model
- [Greedy vs beam search in whisper.cpp](https://medium.com/axinc-ai/whisper-speech-recognition-model-capable-of-recognizing-99-languages-5b5cf0197c16) — beam_size=2 is 2.35x faster than beam_size=5
- [Whisper.cpp benchmark results](https://github.com/ggml-org/whisper.cpp/issues/89) — GPU performance data
- [Tom's Hardware Whisper GPU Benchmarks](https://www.tomshardware.com/news/whisper-audio-transcription-gpus-benchmarked) — hardware comparison
- [whisper.cpp vs faster-whisper comparison](https://github.com/ggml-org/whisper.cpp/issues/1127) — performance discussion
- [How to use Beam Search in Whisper correctly](https://discuss.huggingface.co/t/how-do-you-use-beam-search-in-whisper-correctly/131244) — beam search analysis
