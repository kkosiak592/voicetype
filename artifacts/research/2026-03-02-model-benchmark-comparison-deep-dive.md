# Deep Dive: ASR Model Benchmark Comparison

## Strategic Summary

Your three implemented models — Whisper Small.en, Whisper Large-v3-Turbo, and NVIDIA Parakeet TDT v2 — occupy distinct points on the accuracy/speed/size tradeoff curve. Parakeet TDT v2 delivers the best accuracy (6.05% avg WER) and fastest inference (RTFx 3386) but is English-only and the largest download (2.56 GB). Whisper Large-v3-Turbo offers good accuracy (~7.75% avg WER) with multilingual support. Whisper Small.en is the most lightweight option (190 MB quantized) with acceptable English accuracy.

## Key Questions Answered

- Which model is most accurate for English transcription?
- Which model has the lowest latency?
- How does quantization affect Whisper model accuracy?
- What are the real-world tradeoffs between these three models?

---

## Model Specifications

| Spec | Whisper Small.en | Whisper Large-v3-Turbo | Parakeet TDT v2 |
|------|-----------------|----------------------|-----------------|
| **Developer** | OpenAI | OpenAI | NVIDIA |
| **Parameters** | 244M | 809M | 600M |
| **Architecture** | Transformer Enc-Dec | Transformer Enc-Dec (pruned) | FastConformer-TDT |
| **Decoder Layers** | 12 | 4 (pruned from 32) | Token-and-Duration Transducer |
| **Languages** | English only | 99 languages | English only |
| **Your File Format** | GGML Q5_1 | GGML Q5_0 | ONNX FP32 |
| **Your File Size** | 190 MB | 574 MB | 2.56 GB |
| **Full FP16 Size** | ~461 MB | ~1.5 GB | ~1.2 GB |
| **VRAM Required** | ~2 GB | ~6 GB | ~2 GB min |
| **Training Data** | 680,000 hrs | 5,000,000 hrs | ~120,000 hrs |
| **Release** | Sep 2022 | Oct 2024 | May 2025 |

---

## Accuracy Benchmarks (Word Error Rate %)

### LibriSpeech (Gold Standard Academic Benchmark)

| Dataset | Whisper Small.en | Whisper Large-v3-Turbo | Parakeet TDT v2 |
|---------|-----------------|----------------------|-----------------|
| **test-clean** | 3.05% | ~2.5% | **1.69%** |
| **test-other** | 7.53% | ~5.2% | **3.19%** |

> Lower WER = better accuracy. Parakeet wins by a large margin on LibriSpeech.

### HuggingFace Open ASR Leaderboard (8 Real-World Datasets)

| Dataset | Whisper Small.en | Whisper Large-v3-Turbo | Parakeet TDT v2 |
|---------|-----------------|----------------------|-----------------|
| **LibriSpeech clean** | 3.05% | ~2.5% | **1.69%** |
| **LibriSpeech other** | 7.53% | ~5.2% | **3.19%** |
| **AMI (meetings)** | — | — | 11.16% |
| **Earnings-22 (financial)** | — | — | 11.15% |
| **GigaSpeech** | — | — | 9.74% |
| **SPGI Speech** | — | — | **2.17%** |
| **TEDLIUM-v3** | — | — | 3.38% |
| **VoxPopuli** | — | — | 5.95% |
| **Average WER** | ~12-14%* | ~7.75-10%* | **6.05%** |

> *Whisper models were not benchmarked on the full HF Open ASR suite by OpenAI. Average estimates come from third-party benchmarks (AssemblyAI, Northflank, Modal).

### Noise Robustness (Parakeet TDT v2 Only — Published Data)

| SNR Level | Parakeet TDT v2 Avg WER | Degradation |
|-----------|------------------------|-------------|
| Clean | 6.05% | baseline |
| SNR 10 dB | 6.95% | +15% |
| SNR 5 dB | 8.23% | +36% |
| SNR 0 dB | 11.88% | +96% |
| SNR -5 dB | 20.26% | +235% |

---

## Speed / Latency Benchmarks

### Real-Time Factor (RTFx) — Higher is Faster

| Metric | Whisper Small.en | Whisper Large-v3-Turbo | Parakeet TDT v2 |
|--------|-----------------|----------------------|-----------------|
| **RTFx (GPU, batched)** | ~40x* | ~216x | **3,386x** |
| **Relative Speed (vs Whisper Large)** | ~4x faster | ~8x faster | ~15x faster than Turbo |
| **Time to transcribe 1 min audio** | ~1.5s | ~0.28s | **~0.018s** |
| **Time to transcribe 60 min audio** | ~90s | ~17s | **~1s** |

> *Whisper Small.en RTFx is estimated based on relative speed ratios from the Whisper paper. All GPU numbers assume NVIDIA A100/similar.

### What This Means Practically

- **Parakeet TDT v2**: Effectively instant. 1 hour of audio in ~1 second (batched). Even on consumer GPUs, extremely fast.
- **Whisper Large-v3-Turbo**: Fast enough for real-time use. 8x faster than the full large model it was distilled from.
- **Whisper Small.en**: Still real-time capable. 4x faster than large models. Good for CPU-only scenarios.

### Quantization Impact on Whisper Speed

| Format | Relative Speed | Notes |
|--------|---------------|-------|
| FP16 (baseline) | 1.0x | Full precision |
| Q8_0 | ~1.15x | 15% faster, minimal quality loss |
| Q5_1 | ~1.2x | Moderate speedup |
| Q5_0 | ~1.25x | Slightly faster than Q5_1 |
| Q4_0 | ~1.3x | Fastest quantized, most quality loss |

---

## Quantization Impact on Accuracy

Your Whisper models use quantized GGML formats. Here's what the research shows:

### Whisper Large (Q5_0 — your Turbo model's format)

| Format | WER | Score | Notes |
|--------|-----|-------|-------|
| FP16 (reference) | 0.02 | 3.01 | Baseline |
| Q5_1 | 0.02 | 6.51 | Same WER, higher perplexity |
| **Q5_0** | **0.04** | **13.52** | **~2x WER increase** |
| Q4_0 | 0.05 | 17.30 | Noticeable degradation |

### Whisper Small (Q5_1 — your Small.en format)

| Format | WER | Score | Notes |
|--------|-----|-------|-------|
| FP16 (reference) | 0.06 | 22.00 | Baseline |
| **Q5_1** | **0.08** | **28.04** | **~33% WER increase** |
| Q5_0 | 0.08 | 28.54 | Similar to Q5_1 |
| Q4_0 | 0.10 | 35.00 | Significant degradation |

### Key Quantization Findings

1. **Q5_1 is safer than Q5_0** for accuracy preservation — your Small.en uses the better format
2. **Q5_0 doubles the WER on large models** compared to FP16 — your Turbo model takes a noticeable hit
3. **Smaller models degrade more** from quantization than larger ones
4. **Parakeet TDT v2 runs at FP32** in your setup — no quantization loss at all

---

## Head-to-Head Summary

### Accuracy Ranking (English)

```
1st: Parakeet TDT v2     — 6.05% avg WER (1.69% LibriSpeech clean)
2nd: Whisper Large-v3-Turbo — ~7.75% avg WER (~2.5% LibriSpeech clean)*
3rd: Whisper Small.en     — ~12% avg WER (3.05% LibriSpeech clean)*

* Note: Your quantized versions will have slightly higher WER than these FP16 numbers
```

### Speed Ranking

```
1st: Parakeet TDT v2     — RTFx 3,386 (transcribes 1hr in ~1s)
2nd: Whisper Large-v3-Turbo — RTFx ~216 (transcribes 1hr in ~17s)
3rd: Whisper Small.en     — RTFx ~40 (transcribes 1hr in ~90s)
```

### Download Size Ranking (Smallest First)

```
1st: Whisper Small.en     — 190 MB (Q5_1 quantized)
2nd: Whisper Large-v3-Turbo — 574 MB (Q5_0 quantized)
3rd: Parakeet TDT v2     — 2,560 MB (FP32)
```

### Best For Each Use Case

| Use Case | Best Model | Why |
|----------|-----------|-----|
| **Maximum accuracy (English)** | Parakeet TDT v2 | Lowest WER across all benchmarks |
| **Fast + accurate (English)** | Parakeet TDT v2 | Best at both speed AND accuracy |
| **Multilingual** | Whisper Large-v3-Turbo | Only model supporting 99 languages |
| **Low bandwidth / small download** | Whisper Small.en | 190 MB, still decent accuracy |
| **CPU-only / weak hardware** | Whisper Small.en | Lowest resource requirements |
| **Noisy audio** | Parakeet TDT v2 | Only 15% WER degradation at SNR 10dB |

---

## Limitations & Edge Cases

- **Parakeet TDT v2**: English-only. Cannot transcribe other languages. Requires GPU (CUDA or DirectML). Largest download at 2.56 GB.
- **Whisper Large-v3-Turbo (Q5_0)**: Quantization to Q5_0 roughly doubles error rate vs FP16. The pruned decoder (4 layers vs 32) degrades accuracy on some languages (Thai, Cantonese notably worse).
- **Whisper Small.en (Q5_1)**: Quantization adds ~33% more errors. Limited accuracy on challenging audio (meeting recordings, financial calls). Struggles more with accents and background noise.
- **All models**: LibriSpeech WER numbers are optimistic — real-world audio with background noise, overlapping speakers, and domain jargon will produce higher error rates.

---

## Sources

- [NVIDIA Parakeet TDT 0.6B v2 — HuggingFace Model Card](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v2) — Primary benchmark data for Parakeet
- [OpenAI Whisper GitHub](https://github.com/openai/whisper) — Model size/speed comparison table
- [OpenAI Whisper Paper (arXiv:2212.04356)](https://arxiv.org/abs/2212.04356) — Original WER benchmarks
- [Whisper Large-v3-Turbo — HuggingFace](https://huggingface.co/openai/whisper-large-v3-turbo) — Turbo model specs
- [Whisper Small.en — HuggingFace](https://huggingface.co/openai/whisper-small.en) — Small.en benchmarks
- [whisper.cpp Quantized Model Performance](https://github.com/ggml-org/whisper.cpp/discussions/859) — Quantization accuracy data
- [Quantization for OpenAI's Whisper Models (arXiv:2503.09905)](https://arxiv.org/html/2503.09905v1) — Quantization latency/accuracy tradeoffs
- [Top Open Source STT Models 2025 — Modal](https://modal.com/blog/open-source-stt) — Cross-model comparison
- [Best Open Source STT 2026 — Northflank](https://northflank.com/blog/best-open-source-speech-to-text-stt-model-in-2026-benchmarks) — Aggregate WER and RTFx
- [NVIDIA Parakeet TDT Blog](https://developer.nvidia.com/blog/turbocharge-asr-accuracy-and-speed-with-nvidia-nemo-parakeet-tdt/) — Architecture details
- [Whisper Model Sizes Explained — OpenWhispr](https://openwhispr.com/blog/whisper-model-sizes-explained) — Approximate WER table across sizes
