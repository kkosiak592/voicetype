# Technical Research: Speeding Up Whisper Large Inference

## Strategic Summary

Your Whisper large-v3-turbo q5_0 is already well-optimized (greedy decoding, flash attention, single segment) at 300-600ms. Two high-impact approaches remain: (1) swap to **distil-large-v3.5**, which has 2 decoder layers vs turbo's 4, runs ~1.5x faster, and is actually *more accurate* on short-form audio (7.08% vs 7.30% WER); (2) **more aggressive quantization** (q4_0) for ~15-20% additional speed at minimal accuracy cost. Combined, these could bring Whisper inference to **130-280ms** — a 2-3x improvement over current. A third approach — runtime optimizations (WhisperState reuse, thread tuning) — adds incremental gains of 5-20ms.

**Recommendation: distil-large-v3.5 with q5_0 quantization** — faster, more accurate on your short dictation clips, and drop-in compatible with your existing whisper-rs code.

## Requirements

- **Target**: Reduce Whisper inference from 300-600ms toward 150-300ms
- **Hardware**: Quadro P2000, 5GB VRAM, CUDA compute 6.1
- **Accuracy**: Small WER increase acceptable (0.1-0.5%), prefer improvement
- **Constraint**: Must remain local/offline, Rust/whisper-rs/whisper.cpp stack
- **Audio**: English only, 1-10s hold-to-talk dictation clips, 16kHz mono f32
- **Current model**: `ggml-large-v3-turbo-q5_0.bin` (602MB, 809M params, 4 decoder layers)

## Current Baseline

| Parameter | Value |
|-----------|-------|
| Model | large-v3-turbo q5_0 |
| Params | 809M (32 encoder + 4 decoder layers) |
| File size | 602 MB |
| Inference (5s clip) | 300-600ms |
| Short-form WER | 7.30% |
| Decoding | Greedy (best_of=1) |
| Flash attention | Enabled |
| VRAM usage | ~1.5-2GB estimated |

---

## Approach 1: Switch to distil-large-v3.5 (Primary Recommendation)

### How it works

distil-large-v3.5 is a knowledge-distilled variant of Whisper large-v3. It keeps all 32 encoder layers (which are shared and run once) but reduces the decoder from large-v3's 32 layers → turbo's 4 layers → **distil's 2 layers**. Since the autoregressive decoder runs once per output token and dominates inference time for short-form audio, halving the decoder layers from 4→2 yields ~1.5x speedup.

The critical insight: **distil-large-v3.5 is both faster AND more accurate than large-v3-turbo on short-form audio** (7.08% vs 7.30% WER). This isn't a tradeoff — it's a strict upgrade for your use case.

### Architecture comparison

| Property | large-v3-turbo (current) | distil-large-v3.5 | distil-large-v3 |
|----------|--------------------------|---------------------|-----------------|
| Encoder layers | 32 | 32 | 32 |
| Decoder layers | 4 | 2 | 2 |
| Parameters | 809M | 756M | 756M |
| Short-form WER | 7.30% | **7.08%** | 7.53% |
| Long-form WER | 10.25% | 11.39% | 11.60% |
| Speed vs turbo | 1.0x | **~1.5x** | ~1.4x |
| Training data | OpenAI internal | 98k hours public | 22k hours public |

distil-large-v3.5 was trained on 4x more data than v3, improving robustness. The long-form WER is slightly worse (11.39% vs 10.25%), but your max clip length is ~10s — well within short-form territory where it excels.

### Expected performance on P2000

| Metric | Current (turbo q5_0) | Projected (distil-v3.5 q5_0) |
|--------|----------------------|-------------------------------|
| Inference (5s clip) | 300-600ms | **200-400ms** |
| Inference (2s clip) | 150-350ms | **100-230ms** |
| File size | 602MB | ~500MB (after q5_0) |
| VRAM | ~1.5-2GB | ~1.2-1.7GB |
| Short-form WER | 7.30% | 7.08% (better) |

### Libraries/tools

- **Model source**: `https://huggingface.co/distil-whisper/distil-large-v3.5-ggml/resolve/main/ggml-model.bin` (1.52 GB, fp16)
- **Quantization tool**: whisper.cpp `quantize` binary
- **Runtime**: Same whisper-rs 0.15 — zero code changes to `transcribe_audio()`
- **whisper.cpp compatibility**: Full — distil models use identical architecture, just fewer decoder layers. whisper.cpp reads the layer count from the GGML header.

### Implementation steps

**Step 1: Download the fp16 GGML model**
```
# 1.52 GB download
https://huggingface.co/distil-whisper/distil-large-v3.5-ggml/resolve/main/ggml-model.bin
```

**Step 2: Quantize to q5_0**

You need the whisper.cpp `quantize` tool. Two options:

*Option A: Build whisper.cpp quantize tool locally*
```bash
git clone https://github.com/ggml-org/whisper.cpp
cd whisper.cpp
cmake -B build
cmake --build build --config Release --target quantize
# Run quantization
./build/bin/quantize ggml-model.bin ggml-distil-large-v3.5-q5_0.bin q5_0
```

*Option B: Use pre-built whisper.cpp release*
Download from https://github.com/ggml-org/whisper.cpp/releases — the release includes `quantize.exe` for Windows.

**Step 3: Place model file**
```
%APPDATA%/VoiceType/models/ggml-distil-large-v3.5-q5_0.bin
```

**Step 4: Update model path resolution in `transcribe.rs`**
```rust
// Change line 59:
ModelMode::Gpu => "ggml-distil-large-v3.5-q5_0.bin",
```

That's it. No other code changes required — `transcribe_audio()`, `load_whisper_context()`, and all parameters remain identical.

**Step 5 (optional): Add download support in `download.rs`**

Add the distil model as a downloadable option in the UI, similar to how Parakeet models are handled. Download the fp16 GGML file and quantize on the user's machine, or host a pre-quantized version.

### Pros

- **Faster AND more accurate** on short-form audio — not a tradeoff
- **Zero inference code changes** — same whisper-rs API, same parameters
- **Smaller model** — 756M params vs 809M, ~500MB q5_0 vs 602MB
- **Less VRAM** — fits comfortably on P2000's 5GB
- **Same initial_prompt support** — vocabulary biasing works identically
- **Same corrections engine** — no pipeline changes
- **4x more training data** than distil-v3 — more robust across accents/noise

### Cons

- **Slightly worse long-form WER** (11.39% vs 10.25%) — irrelevant for your 1-10s clips
- **Requires quantization step** — no pre-quantized q5_0 available; must build or ship quantize tool
- **1.52GB initial download** — larger than final model (discard fp16 after quantization)
- **New model file to distribute** — need to add download path in UI or ship pre-quantized
- **Less tested in wild** — distil-v3.5 is newer than turbo

### Risk assessment

Low risk. The model uses identical architecture to your current one (same encoder, same Whisper framework), just fewer decoder layers. If something goes wrong, reverting is as simple as pointing back to the turbo model file. Flash attention, greedy decoding, initial_prompt — all work identically.

### Complexity: S

**Best when:** You want meaningful speedup with zero accuracy loss on short dictation. This is the highest-ROI change available.

---

## Approach 2: More Aggressive Quantization (q4_0 / q4_1)

### How it works

Your current model uses q5_0 quantization (5-bit weights). Dropping to q4_0 (4-bit) reduces model size by ~20% and speeds up matrix multiplications because fewer bits need to be processed per weight. On GPU, the effect is primarily through reduced memory bandwidth (weights are smaller → faster to load from VRAM to compute cores). On the P2000's limited memory bandwidth, this matters.

This can be applied to **either** the current large-v3-turbo model **or** the distil-large-v3.5 model (they stack).

### Quantization comparison

| Quantization | Bit width | Size (turbo) | Size (distil-v3.5) | Speed vs q5_0 | WER impact |
|-------------|-----------|-------------|---------------------|---------------|------------|
| q5_0 (current) | 5-bit | 602 MB | ~500 MB | baseline | baseline |
| q5_1 | 5-bit + extra | ~650 MB | ~540 MB | ~same | slightly better |
| q4_0 | 4-bit | ~450 MB | ~380 MB | **~15-20% faster** | +0.1-0.3% WER |
| q4_1 | 4-bit + extra | ~500 MB | ~420 MB | ~10-15% faster | +0.05-0.15% WER |
| q8_0 | 8-bit | ~900 MB | ~750 MB | ~10% slower | near-zero |

### Expected performance

**On current large-v3-turbo:**

| Metric | q5_0 (current) | q4_0 |
|--------|-----------------|------|
| Inference (5s) | 300-600ms | **250-500ms** |
| File size | 602 MB | ~450 MB |
| WER | 7.30% | ~7.4-7.6% |

**On distil-large-v3.5 (combined with Approach 1):**

| Metric | distil q5_0 | distil q4_0 |
|--------|-------------|-------------|
| Inference (5s) | 200-400ms | **170-340ms** |
| File size | ~500 MB | ~380 MB |
| WER | 7.08% | ~7.2-7.3% |

### Implementation

**Quantize existing turbo model:**
```bash
# Using whisper.cpp quantize tool
./quantize ggml-large-v3-turbo-q5_0.bin ggml-large-v3-turbo-q4_0.bin q4_0
```

Note: You should quantize from the **fp16 source**, not from q5_0. Quantizing an already-quantized model compounds errors. For turbo, download the fp16 from ggerganov's HuggingFace repo first:
```
https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin
```
Then quantize:
```bash
./quantize ggml-large-v3-turbo.bin ggml-large-v3-turbo-q4_0.bin q4_0
```

**Code change:** Same as Approach 1 — just change the filename in `resolve_model_path()`.

### Pros

- **Simplest possible change** — swap one model file
- **Stacks with Approach 1** — apply q4_0 to distil model for compounding gains
- **Smaller file** — less disk, less VRAM, less bandwidth on download
- **Well-understood** — GGML quantization is battle-tested in whisper.cpp

### Cons

- **Measurable accuracy loss** — q4_0 adds ~0.1-0.3% WER (within your tolerance)
- **Must quantize from fp16** — can't reliably re-quantize from q5_0
- **Diminishing returns on GPU** — GPU compute isn't always bandwidth-bound; gains vary by hardware. P2000 should benefit more than a 4090 would
- **q4_1 vs q4_0 tradeoff** — q4_1 has slightly better accuracy but is slightly slower/larger; worth testing both

### Risk assessment

Very low. Model file swap with easy rollback.

### Complexity: S

**Best when:** You want incremental speed with minimal effort, or as a multiplier on top of Approach 1.

---

## Approach 3: Runtime Optimizations (WhisperState Reuse + Thread Tuning)

### How it works

Several runtime changes can shave 5-25ms per transcription call without changing the model:

#### 3a. WhisperState reuse

Currently (`transcribe.rs:136`), a fresh `WhisperState` is created per call:
```rust
let mut state = ctx.create_state().map_err(|e| e.to_string())?;
```

This allocates scratch buffers, CUDA memory, and internal caches each time (~5-20ms). Reusing a state across calls avoids this allocation overhead. The whisper-rs API supports this — `create_state()` returns a `WhisperState` that can be called with `state.full()` multiple times.

**Implementation:**
```rust
// In lib.rs managed state, alongside Arc<WhisperContext>:
pub struct WhisperEngine {
    ctx: WhisperContext,
    state: Mutex<WhisperState>,  // reuse across calls
}

// In transcribe_audio(), change to:
pub fn transcribe_audio(engine: &WhisperEngine, audio: &[f32], initial_prompt: &str) -> Result<String, String> {
    let start = Instant::now();
    let mut state = engine.state.lock().map_err(|e| e.to_string())?;
    // ... rest unchanged, use &mut state instead of creating new one
}
```

**Caveat:** Verify that reusing state doesn't cause context bleed between calls (the `set_no_context(true)` parameter should handle this, but test carefully). The whisper.cpp docs say state is safe to reuse if you call `full()` with fresh params each time.

**Savings:** ~5-20ms per call.

#### 3b. Thread count tuning

whisper-rs/whisper.cpp defaults to using all available CPU threads for the encoder. On a system with a P2000 GPU, the encoder runs on GPU, but some operations (pre/post processing, token sampling) still run on CPU. Setting optimal thread count can avoid contention:

```rust
params.set_n_threads(4);      // CPU threads for encoder fallback
params.set_n_threads_for(4);  // CPU threads for decoder
```

On a GPU system, fewer CPU threads can actually be faster by reducing context-switching overhead. The optimal value is hardware-specific — benchmark with 1, 2, 4, and 8 threads.

**Savings:** 0-15ms depending on configuration.

#### 3c. Avoid full buffer clone

Currently `get_buffer()` clones the entire audio buffer (identified as P1 latency issue in your performance review). Moving to `Arc<Vec<f32>>` or swapping the buffer instead of cloning avoids this 5-50ms cost:

```rust
// Instead of cloning:
let buffer = shared_buffer.lock().unwrap().clone();

// Swap with empty vec (O(1)):
let buffer = std::mem::take(&mut *shared_buffer.lock().unwrap());
```

**Savings:** 5-50ms depending on audio length.

### Combined savings

| Optimization | Savings | Effort |
|-------------|---------|--------|
| WhisperState reuse | 5-20ms | Low |
| Thread tuning | 0-15ms | Trivial (parameter) |
| Buffer swap vs clone | 5-50ms | Low |
| **Total** | **10-85ms** | Low |

### Pros

- **No model changes** — same accuracy guaranteed
- **Low risk** — each change is small and independently reversible
- **Stacks with Approaches 1 and 2** — these are orthogonal optimizations
- **Buffer swap has the highest single impact** in this group

### Cons

- **Ceiling is low** — max ~85ms savings, doesn't touch the dominant inference cost
- **WhisperState reuse needs testing** — must verify no context bleed between calls
- **Thread tuning is hardware-specific** — optimal value differs per machine

### Complexity: S

**Best when:** You've already applied Approaches 1-2 and want to squeeze out remaining milliseconds.

---

## Comparison

| Aspect | Approach 1: distil-v3.5 | Approach 2: q4_0 quant | Approach 3: Runtime opts |
|--------|-------------------------|------------------------|--------------------------|
| **Speed gain** | ~1.5x (100-200ms saved) | ~15-20% (50-100ms saved) | ~10-85ms saved |
| **Short-form WER** | 7.08% (better!) | +0.1-0.3% (slightly worse) | No change |
| **Code changes** | Filename swap + download | Filename swap | Small refactors |
| **Risk** | Low | Very low | Very low |
| **Complexity** | S | S | S |
| **P2000 VRAM** | ~1.2-1.7GB (fine) | ~1.0-1.5GB (fine) | No change |
| **Stacks with others** | Yes (+ q4_0, + runtime) | Yes (+ distil, + runtime) | Yes (+ both) |

### Combined projection: All three approaches

| Configuration | Est. inference (5s) | Short-form WER | File size |
|--------------|---------------------|----------------|-----------|
| Current (turbo q5_0) | 300-600ms | 7.30% | 602 MB |
| distil-v3.5 q5_0 | 200-400ms | 7.08% | ~500 MB |
| distil-v3.5 q4_0 | 170-340ms | ~7.2% | ~380 MB |
| distil-v3.5 q4_0 + runtime | **130-280ms** | ~7.2% | ~380 MB |

That's a **2-3x speedup** over current, with comparable or better accuracy.

---

## Recommendation

**Primary: Approach 1 (distil-large-v3.5) with q5_0 quantization.**

Rationale:
- It's faster AND more accurate on your exact use case (short English dictation)
- Zero code changes to inference logic — just a different model file
- Fits comfortably on P2000 VRAM
- Stacks with q4_0 and runtime optimizations if you want more later

**Secondary: Apply q4_0 quantization on top** if 200-400ms still feels slow. The ~0.1-0.3% WER increase is within your stated tolerance.

**Tertiary: Buffer swap optimization** from Approach 3c — it's the highest-ROI runtime change (up to 50ms saved) and fixes a known P1 issue in your codebase regardless of model choice.

Skip the thread tuning unless you're chasing the last 10ms — it requires benchmarking and the gains are uncertain on GPU.

---

## Implementation Context

<claude_context>
<chosen_approach>
- name: distil-large-v3.5 model swap with optional q4_0 quantization
- libraries: whisper-rs 0.15 (unchanged), whisper.cpp quantize tool (one-time)
- install: no new dependencies — model file swap only
- model_url_fp16: https://huggingface.co/distil-whisper/distil-large-v3.5-ggml/resolve/main/ggml-model.bin
- model_size_fp16: 1.52 GB
- model_size_q5_0: ~500 MB (estimated)
- model_size_q4_0: ~380 MB (estimated)
</chosen_approach>
<architecture>
- pattern: Drop-in model replacement (same whisper.cpp GGML format)
- components: Only transcribe.rs model path + download.rs model entry
- data_flow: Audio → WhisperContext (new model) → same greedy decode → text (unchanged)
</architecture>
<files>
- modify: src-tauri/src/transcribe.rs (line 59 — model filename)
- modify: src-tauri/src/download.rs (add distil model download URL/checksum)
- modify: frontend model selection UI (add distil-large-v3.5 as option)
- reference: existing Parakeet download flow in download.rs for multi-model support
- one-time: whisper.cpp quantize tool to produce q5_0/q4_0 from fp16
</files>
<implementation>
- start_with: Download fp16 ggml-model.bin, quantize to q5_0, test locally
- order: 1) Get quantized model working locally 2) Compare inference times 3) Add download support in UI 4) Apply runtime optimizations (buffer swap)
- gotchas:
  - Must quantize from fp16 source, NOT from existing q5_0 (double-quantization degrades quality)
  - distil-v3.5 ggml file is named ggml-model.bin on HuggingFace — rename to something descriptive
  - No pre-quantized versions exist; you'll need to either ship the quantize step or host pre-quantized files
  - WhisperState reuse requires testing for context bleed with initial_prompt
  - P2000 compute capability 6.1 is supported by whisper.cpp CUDA but won't benefit from newer GPU features (tensor cores, etc.)
- testing:
  - A/B compare same audio clips: turbo q5_0 vs distil-v3.5 q5_0 vs distil-v3.5 q4_0
  - Log inference times (already instrumented in transcribe.rs:169)
  - Test accuracy on your domain vocabulary (structural engineering terms)
  - Verify initial_prompt biasing works correctly with distil model
  - Test cold start (first inference after model load) and warm inference
</implementation>
</claude_context>

**Next Action:** Download distil-large-v3.5 fp16 GGML, quantize to q5_0, drop into models directory, and benchmark against current turbo q5_0. Single-line code change in `transcribe.rs:59` to test.

## Sources

- [distil-whisper/distil-large-v3.5](https://huggingface.co/distil-whisper/distil-large-v3.5) — Model card with WER benchmarks (7.08% short-form)
- [distil-whisper/distil-large-v3.5-ggml](https://huggingface.co/distil-whisper/distil-large-v3.5-ggml) — GGML format weights (1.52 GB fp16)
- [distil-whisper/distil-large-v3-ggml](https://huggingface.co/distil-whisper/distil-large-v3-ggml) — v3 GGML weights for reference
- [ggml-org/whisper.cpp](https://github.com/ggml-org/whisper.cpp) — whisper.cpp repo with quantize tool
- [whisper.cpp models README](https://github.com/ggml-org/whisper.cpp/blob/master/models/README.md) — Quantization methods and model hashes
- [openai/whisper-large-v3-turbo](https://huggingface.co/openai/whisper-large-v3-turbo) — Current model specs
- [Quantization for OpenAI's Whisper Models](https://arxiv.org/html/2503.09905v1) — Academic analysis of quantization impact (2025)
- [Speculative Decoding for Whisper](https://huggingface.co/blog/whisper-speculative-decoding) — HuggingFace blog (not available in whisper.cpp)
- [Demystifying OpenAI's Whisper Turbo](https://amgadhasan.substack.com/p/demystifying-openais-new-whisper) — Architecture comparison (turbo vs distil)
