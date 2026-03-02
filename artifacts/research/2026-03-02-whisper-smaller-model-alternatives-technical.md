# Technical Research: Faster Whisper Model Alternatives

## Strategic Summary

Your large-v3-turbo Q5_0 is already fast (~3x faster than full large-v3) but for "instantaneous" feel you need sub-200ms inference. Three viable step-downs exist: **small.en** (already in your codebase as CPU fallback — just enable it on GPU), **base.en** (new download, ~50-100ms inference), or **distil-large-v3.5** (same-class model, ~1.5x faster than turbo with *better* accuracy). The .en English-only models are strictly superior for your use case since you already force English.

**Recommendation: small.en Q5_1 on GPU** as the first test — it's already downloaded, requires a one-line change, and should hit ~100-200ms inference on your P2000. If accuracy is fine (it usually is for short dictation), done. If not, distil-large-v3.5 is the fallback.

## Requirements

- **Target**: "Instantaneous" feel — sub-200ms inference for 1-5s clips
- **Hardware**: Quadro P2000 (5GB VRAM, Pascal compute 6.1, 140 GB/s bandwidth)
- **Current**: large-v3-turbo Q5_0, 602 MB, ~300-600ms inference
- **Constraint**: whisper.cpp via whisper-rs, local/offline, English-only dictation
- **Acceptable tradeoff**: Some accuracy loss for significantly better speed

## Current Baseline vs Whisper Model Hierarchy

**Speed factors relative to large-v3 (from benchmarks):**

| Model | Params | Speed vs large-v3 | Speed vs turbo (current) | English WER | Quantized Size (Q5) |
|-------|--------|-------------------|--------------------------|-------------|---------------------|
| large-v3 | 1,550M | 1.0x (baseline) | 0.33x (3x slower) | 2.4% | ~1.1 GB |
| **large-v3-turbo (current)** | **809M** | **~3x** | **1.0x** | **2.5%** | **602 MB** |
| distil-large-v3.5 | 756M | ~4.5x | ~1.5x | 2.5%* | ~500 MB |
| medium | 769M | ~2x | 0.67x (slower!) | 2.9% | 514 MB |
| **medium.en** | **769M** | **~2.3x** | **0.77x (slower!)** | **2.9%** | **~500 MB** |
| **small.en** | **244M** | **~4.5x** | **~1.5x faster** | **3.4%** | **181 MB** |
| **base.en** | **74M** | **~8x** | **~2.7x faster** | **5.0%** | **57 MB** |
| tiny.en | 39M | ~12x | ~4x faster | 7.6% | 31 MB |

\* distil-large-v3.5 short-form WER is 7.08% vs turbo's 7.30% on standard benchmarks — see [prior research](2026-03-02-whisper-large-speed-optimization-technical.md).

**Key insight: medium.en is SLOWER than turbo.** Despite having fewer decoder layers, turbo's pruned architecture (32 encoder + 4 decoder) is more efficient than medium's full architecture (24 encoder + 24 decoder). Stepping down to medium is not useful.

## Estimated Inference Times on P2000 (GPU, 5s clip)

| Model | Estimated Inference | Feel |
|-------|-------------------|------|
| large-v3-turbo Q5_0 (current) | 300-600ms | Noticeable pause |
| distil-large-v3.5 Q5_0 | 200-400ms | Slight pause |
| small.en Q5_1 | 100-200ms | Near-instant |
| base.en Q5_1 | 50-100ms | Instant |
| tiny.en Q5_1 | 30-60ms | Instant |

Note: On GPU, smaller models see diminishing speed gains because the P2000's 1003 CUDA cores become underutilized. The overhead of CUDA kernel launches and memory transfers becomes a larger fraction of total time. That's why small→base→tiny don't show the same dramatic 2-3x jumps you'd see on CPU.

---

## Approach 1: small.en Q5_1 on GPU (Recommended First Test)

**How it works:** You already have `ggml-small.en-q5_1.bin` (181 MB) downloaded as your CPU fallback model. Currently, your code picks this model only when no NVIDIA GPU is detected. Simply exposing it as a selectable GPU model lets you test it immediately — zero download, one code change.

The small.en model has 244M parameters with 12 encoder + 12 decoder layers. The `.en` suffix means it was trained English-only, which gives ~10-15% better English accuracy than the multilingual `small` model and slightly faster inference (smaller vocabulary = fewer output tokens to consider).

**Expected performance:**

| Metric | Turbo Q5_0 (current) | Small.en Q5_1 (proposed) |
|--------|---------------------|--------------------------|
| Inference (5s clip) | 300-600ms | **100-200ms** |
| Inference (2s clip) | 150-350ms | **50-120ms** |
| English WER | 2.5% | 3.4% |
| File size | 602 MB | 181 MB |
| VRAM usage | ~1.5-2 GB | ~500 MB |
| Initial prompt support | Yes | Yes |

**What 3.4% vs 2.5% WER means in practice:** For a 100-word dictation, you'd see roughly 1 extra error per 100 words. For short 5-15 word hold-to-talk clips (your use case), the chance of any single clip having an error goes from ~12.5% to ~17% — noticeable but not dramatic. For structural engineering terms biased via initial_prompt, the practical gap is likely smaller since the prompt compensates.

**Implementation:**

The only change needed is in model selection logic. Currently `transcribe.rs` hard-codes the model per GPU/CPU mode. To test, you can just temporarily point the GPU path at the small model.

Longer-term, the model is already registered in `download.rs:71-75` as `"small-en"`. The UI model selector should already list it. You just need the backend to allow running it on GPU.

**Pros:**
- Zero download — model already exists on disk
- Massive speed improvement (~2-3x faster)
- Much less VRAM — frees memory for other work
- English-only model is optimal for your English-only use case
- Initial prompt vocabulary biasing still works

**Cons:**
- WER increase from 2.5% → 3.4% (roughly 1 extra error per 100 words)
- 12 encoder layers vs 32 means less capacity for noisy audio / complex vocabulary
- Domain-specific terms (structural engineering) may suffer more than general vocabulary
- No turbo-style decoder pruning — 12 decoder layers means decoder is proportionally slower vs encoder

**Best when:** You want the fastest path to testing a speed improvement with zero setup.

**Complexity:** S (trivial — model file already exists)

---

## Approach 2: base.en Q5_1 (Maximum Speed)

**How it works:** The base.en model (74M params, 6 encoder + 6 decoder layers) is the smallest model that still produces usable dictation quality for clear English speech. At 57 MB quantized, it loads nearly instantly and infers in under 100ms for short clips.

**Expected performance:**

| Metric | Turbo Q5_0 (current) | Base.en Q5_1 (proposed) |
|--------|---------------------|--------------------------|
| Inference (5s clip) | 300-600ms | **50-100ms** |
| Inference (2s clip) | 150-350ms | **30-60ms** |
| English WER | 2.5% | 5.0% |
| File size | 602 MB | 57 MB |
| VRAM usage | ~1.5-2 GB | ~200 MB |

**What 5.0% WER means in practice:** About 1 error per 20 words. For a typical 10-word dictation clip, ~50% chance of a perfect transcription. Errors tend to cluster around uncommon words, homophones, and domain-specific terms. For simple conversational dictation, it's fine. For structural engineering terms like "W-section" or "ACI 318", accuracy will degrade more — the corrections engine can compensate for known patterns, but novel terms will suffer.

**Implementation:**

1. Add to `download.rs`:
```rust
"base-en" => Some((
    "ggml-base.en-q5_1.bin",
    "<sha256>",  // need to look up from whisper.cpp models README
    59_703_808,  // ~57 MB
)),
```

2. Update UI model list and transcribe.rs model resolution.

**Available from:** `https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en-q5_1.bin`

**Pros:**
- Truly instant inference — 50-100ms is imperceptible
- Tiny file size (57 MB) — fast download, minimal disk/VRAM
- Corrections engine can patch known domain-term errors post-hoc
- Good enough for general dictation and simple note-taking

**Cons:**
- 5.0% WER — noticeable accuracy drop, especially on domain vocabulary
- 6 encoder layers = poor handling of background noise, accents, fast speech
- Initial prompt effectiveness diminishes with smaller models (less capacity to leverage context)
- May feel unreliable for professional use (structural engineering specs)

**Best when:** Speed is the absolute priority and you're doing simple dictation with clean audio.

**Complexity:** S

---

## Approach 3: distil-large-v3.5 Q5_0 (Speed + Quality)

**How it works:** Already detailed in [prior research](2026-03-02-whisper-large-speed-optimization-technical.md). This is a knowledge-distilled large model with 32 encoder + 2 decoder layers (vs turbo's 4 decoder layers). It's ~1.5x faster than turbo with **better** short-form accuracy (7.08% vs 7.30% on standard benchmarks, ~2.5% English WER).

**Expected performance:**

| Metric | Turbo Q5_0 (current) | distil-v3.5 Q5_0 |
|--------|---------------------|-------------------|
| Inference (5s clip) | 300-600ms | **200-400ms** |
| Inference (2s clip) | 150-350ms | **100-230ms** |
| English WER | 2.5% | ~2.5% (same class) |
| File size | 602 MB | ~500 MB |

**Tradeoff:** This doesn't get you to "instantaneous" by itself, but combined with the runtime optimizations from prior research (buffer swap, state reuse) it could reach 150-300ms range. The advantage is you lose **zero** accuracy.

**See:** [2026-03-02-whisper-large-speed-optimization-technical.md](2026-03-02-whisper-large-speed-optimization-technical.md) for full implementation details.

**Best when:** You want faster but can't accept any accuracy loss.

**Complexity:** S (model swap + one-time quantization step)

---

## Approach 4: Offer All Models as User-Selectable Options

**How it works:** Instead of picking one model, expose the full range as a dropdown in your existing model selection UI. Users (you) can choose based on the situation:
- **Turbo / distil-v3.5:** When accuracy matters (engineering specs, client-facing work)
- **small.en:** Default for daily use (good balance)
- **base.en:** When you want instant response and content is simple

Your app already has the infrastructure for this — the `ModelSection` UI, the `download_model` command, and the model resolution logic in `transcribe.rs`. It's just a matter of expanding the model registry.

**Models to add:**

| Model ID | Filename | Download Size | Use Case Label |
|----------|----------|---------------|----------------|
| `large-v3-turbo` | ggml-large-v3-turbo-q5_0.bin | 602 MB | "High Accuracy" |
| `small-en` | ggml-small.en-q5_1.bin | 181 MB | "Balanced" |
| `base-en` | ggml-base.en-q5_1.bin | 57 MB | "Fast" |

(distil-large-v3.5 requires a quantization step, so it's better as a separate effort.)

**Pros:**
- Maximum flexibility — switch models without code changes
- Infrastructure already exists
- Each model is a one-line addition to `download.rs` and `transcribe.rs`

**Cons:**
- More models = more UI complexity
- Users need to understand the tradeoffs
- Each model needs SHA256 checksum lookup

**Best when:** You want to experiment with different speed/accuracy points without recompiling.

**Complexity:** M (UI + backend registry expansion)

---

## Comparison

| Aspect | small.en (Approach 1) | base.en (Approach 2) | distil-v3.5 (Approach 3) | Multi-model (Approach 4) |
|--------|----------------------|---------------------|--------------------------|--------------------------|
| Inference (5s) | 100-200ms | 50-100ms | 200-400ms | Varies |
| English WER | 3.4% | 5.0% | ~2.5% | User choice |
| File size | 181 MB | 57 MB | ~500 MB | Sum of selected |
| Code changes | 1 line | Registry + download | Quantization step | UI + registry |
| Already available | Yes (downloaded) | No (57 MB download) | No (1.5 GB download + quantize) | No |
| Best for | Quick test now | Max speed | No accuracy loss | Long-term solution |
| Risk | Low | Low | Low | Low |

## Recommendation

**Immediate test (today):** Try **small.en Q5_1 on GPU** (Approach 1). Change one line in `transcribe.rs` to point the GPU model at `ggml-small.en-q5_1.bin`. Dictate some engineering terms and see if the accuracy is acceptable for your daily use. If it feels instant and accurate enough — done.

**If small.en accuracy isn't enough:** Go with **distil-large-v3.5** (Approach 3) — it's faster than turbo with no accuracy penalty, but requires a quantization step.

**If you want maximum speed for simple dictation:** Add **base.en** (Approach 2) alongside the current turbo model.

**Long-term:** Implement Approach 4 — let the user pick their speed/accuracy tradeoff from the UI. Your app already has the model selection infrastructure.

**Skip medium.en entirely** — it's actually slower than turbo due to its full 24-layer decoder, despite having a smaller encoder.

---

## Implementation Context

<claude_context>
<chosen_approach>
- name: small.en Q5_1 GPU test (immediate), then multi-model support
- libraries: whisper-rs 0.15 (unchanged)
- install: no new dependencies for small.en (already downloaded)
</chosen_approach>
<architecture>
- pattern: Expand model registry to support multiple whisper models selectable at runtime
- components: download.rs model_info(), transcribe.rs resolve_model_path(), frontend ModelSection
- data_flow: User selects model in UI → stored in settings → transcribe.rs resolves path → same inference pipeline
</architecture>
<files>
- modify: src-tauri/src/transcribe.rs (model path resolution to support multiple models)
- modify: src-tauri/src/download.rs (add base-en model entry with checksum)
- modify: frontend model selection components
- existing: ggml-small.en-q5_1.bin already in %APPDATA%/VoiceType/models/
- download: ggml-base.en-q5_1.bin from huggingface.co/ggerganov/whisper.cpp (~57 MB)
</files>
<implementation>
- start_with: Change transcribe.rs GPU model to small.en, test dictation accuracy
- order: 1) Test small.en on GPU 2) Benchmark latency 3) Test domain vocabulary accuracy 4) If acceptable, add base.en as additional option 5) Expand UI model selector
- gotchas:
  - small.en uses q5_1 (not q5_0 like turbo) — slightly different quantization but whisper.cpp handles both transparently
  - .en models have a different tokenizer — initial_prompt may behave slightly differently
  - GPU utilization may be low with small/base models on P2000 (not enough compute to saturate 1003 CUDA cores)
  - Flash attention benefit decreases with fewer layers
  - Corrections engine becomes more important with smaller models (compensates for accuracy gap)
- testing:
  - A/B compare: turbo vs small.en on same 10 audio clips
  - Test structural engineering vocabulary specifically
  - Measure end-to-end latency (hotkey release → text injected) not just inference
  - Test with initial_prompt enabled and disabled
</implementation>
</claude_context>

**Next Action:** Change `transcribe.rs` line 59 to use `ggml-small.en-q5_1.bin` for GPU mode, rebuild, and benchmark a few dictation clips.

## Sources

- [Whisper Model Sizes Explained](https://openwhispr.com/blog/whisper-model-sizes-explained) — WER and speed comparisons across all sizes
- [Which Whisper Model Should I Choose?](https://whisper-api.com/blog/models/) — Model selection guide
- [whisper.cpp Model Comparison Discussion #3074](https://github.com/ggml-org/whisper.cpp/discussions/3074) — Benchmark data (base through large-v3)
- [ggerganov/whisper.cpp on HuggingFace](https://huggingface.co/ggerganov/whisper.cpp) — GGML model downloads with quantized variants
- [openai/whisper-large-v3-turbo](https://huggingface.co/openai/whisper-large-v3-turbo) — Turbo model specs
- [distil-whisper/distil-large-v3-ggml](https://huggingface.co/distil-whisper/distil-large-v3-ggml) — Distilled model GGML weights
- [Quantization for OpenAI's Whisper Models](https://arxiv.org/html/2503.09905v1) — Academic quantization analysis (2025)
- [Best open source STT model in 2026](https://northflank.com/blog/best-open-source-speech-to-text-stt-model-in-2026-benchmarks) — Current landscape benchmarks
- [Prior research: Whisper Large Speed Optimization](2026-03-02-whisper-large-speed-optimization-technical.md) — distil-large-v3.5 deep dive
