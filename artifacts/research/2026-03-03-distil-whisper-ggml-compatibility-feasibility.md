# Feasibility Assessment: Distil-Whisper Models in VoiceType

## Strategic Summary

**GO.** All three distil-whisper models (v3.5, v3, v2) have pre-converted GGML binaries on HuggingFace, whisper.cpp has supported distil architectures since Nov 2023, and VoiceType's hold-to-talk use case (short clips) sidesteps the only known quality issue (long-form chunked decoding). Integration requires adding a model entry in `download.rs` and updating selection logic — no architectural changes needed.

## What We're Assessing

Whether `distil-whisper/distil-large-v3.5`, `distil-large-v3`, or `distil-large-v2` can replace or supplement the current `ggml-large-v3-turbo-q5_0.bin` (601 MB) as the GPU model in VoiceType's whisper.cpp pipeline via whisper-rs 0.15.

---

## Technical Feasibility

**Can we build it? YES.**

### Pre-Converted GGML Files Exist

| Model | GGML Repo | File | Size (fp16) |
|-------|-----------|------|-------------|
| distil-large-v3.5 | `distil-whisper/distil-large-v3.5-ggml` | `ggml-model.bin` | 1.52 GB |
| distil-large-v3 | `distil-whisper/distil-large-v3-ggml` | `ggml-distil-large-v3.bin` | 1.52 GB |
| distil-large-v3 (fp32) | same | `ggml-distil-large-v3.fp32.bin` | 3.03 GB |
| distil-large-v2 | `distil-whisper/distil-large-v2-ggml` | (available) | ~1.52 GB |

No manual model conversion needed — the HuggingFace distil-whisper team published these.

### Quantization Path

whisper.cpp's `quantize` tool supports Q4_0, Q4_1, Q5_0, Q5_1, Q8_0 on any GGML model.

Estimated Q5_0 sizes (based on large-v3-turbo ratio of ~0.4x from fp16):
- **distil-large-v3/v3.5 Q5_0: ~550-600 MB** (comparable to current turbo at 601 MB)
- **distil-large-v3/v3.5 fp16: 1.52 GB** (usable as-is, just larger download)

### whisper.cpp Distil Support

- Added via [PR #1424](https://github.com/ggml-org/whisper.cpp/issues/1423) (Nov 2023)
- Architecture handled by GGML binary metadata — no code-side layer config needed
- whisper-rs 0.15 wraps a recent whisper.cpp version — distil support included

### Known Limitation: Long-Form Chunked Decoding

whisper.cpp does **NOT** implement the overlapping-chunk decoding strategy that distil-whisper uses for long-form audio (15s chunks with 2.5s overlap). This causes quality degradation on audio > 30 seconds.

**This does NOT affect VoiceType** because:
- Hold-to-talk produces short clips (typically < 30 seconds)
- VoiceType uses single-segment mode (no long-form decoding)
- Temperature = 0.0, greedy decoding, English-forced

distil-large-v3 and v3.5 were also specifically designed to work with the standard sequential algorithm (unlike v2 which depended more on chunking).

### Short-Form Performance (VoiceType's use case)

| Model | Short-Form WER | Speed vs turbo | Params |
|-------|---------------|----------------|--------|
| large-v3 (full) | 7.14% | baseline | 1550M |
| **distil-large-v3.5** | **7.10%** | **1.46x faster** | 756M |
| large-v3-turbo (current) | 7.25% | 1.0x | 809M |
| distil-large-v3 | 7.41% | 1.44x faster | 756M |

distil-large-v3.5 actually **beats the full large-v3** on short-form accuracy while being 1.46x faster than turbo.

**Technical verdict: Feasible** — drop-in compatible with existing pipeline.

---

## Resource Feasibility

**Do we have what we need? YES.**

- **Skills**: No new skills needed. Same whisper-rs API, same model loading path.
- **Tooling**: whisper.cpp quantize tool available if Q5_0 desired.
- **Infrastructure**: Models hosted on HuggingFace (same as current source).
- **Code changes**: ~20 lines in `download.rs` + optional selection logic update.

**Resource verdict: Feasible** — minimal effort.

---

## External Dependency Feasibility

**Are external factors reliable? YES.**

- **Model hosting**: HuggingFace CDN (same provider as current models) — stable.
- **GGML format**: Maintained by ggml-org, actively developed — low risk.
- **whisper-rs**: v0.15 already in use, no version change needed.
- **Pre-converted files**: Published by the official distil-whisper team (HuggingFace research) — authoritative source.

**External verdict: Feasible** — no new dependencies.

---

## Blockers

| Blocker | Severity | Mitigation |
|---------|----------|------------|
| No pre-quantized Q5_0 distil GGML files on HF | Low | Use fp16 (1.52 GB) as-is, or run quantize tool once and self-host |
| SHA256 checksums not pre-computed | Low | Download once, compute hash, add to `download.rs` |
| distil-large-v3.5-ggml named `ggml-model.bin` (generic) | Low | Rename on download or in model_info mapping |
| Long-form quality gap in whisper.cpp | N/A | VoiceType uses short clips — not affected |

No high-severity blockers.

---

## De-risking Options

- **5-minute smoke test**: Download `ggml-distil-large-v3.bin` (1.52 GB), point VoiceType at it via hardcoded path, test transcription quality on typical hold-to-talk clips. Validates compatibility with zero code changes.
- **Quantize and benchmark**: Run whisper.cpp's `quantize` on the fp16 GGML to produce Q5_0. Compare speed/quality to current large-v3-turbo Q5_0. Cost: ~10 minutes.
- **Ship fp16 first**: Skip quantization entirely. 1.52 GB download is reasonable for a GPU model. Quantize later if download size becomes a user complaint.

---

## Overall Verdict

**GO — no conditions.**

All three distil models are compatible. Recommended model ranking for VoiceType:

| Rank | Model | Why |
|------|-------|-----|
| 1 | **distil-large-v3.5** | Best short-form WER (7.10%), fastest (1.46x turbo), 1.52 GB fp16 |
| 2 | distil-large-v3 | Slightly worse WER (7.41%), same speed class, 1.52 GB fp16 |
| 3 | distil-large-v2 | Worse long-form (16.35% WER), older, no advantage over v3/v3.5 |

distil-large-v3.5 is the clear winner — better accuracy than even the full large-v3 on short-form, 1.46x faster than turbo, and same fp16 GGML size as v3.

### vs. Current large-v3-turbo Q5_0 (601 MB)

| Metric | Current (turbo Q5_0) | distil-v3.5 (fp16) | distil-v3.5 (est. Q5_0) |
|--------|---------------------|---------------------|--------------------------|
| Download | 601 MB | 1.52 GB | ~550-600 MB |
| Short-form WER | 7.25% | 7.10% | ~7.1-7.2% |
| Speed | baseline | 1.46x faster | ~1.4x faster |
| Params | 809M | 756M | 756M |

---

## Implementation Context

```
<claude_context>
<if_go>
- approach: Add distil-large-v3.5-ggml as new model option in download.rs, download from HF
- start_with: Smoke test — download ggml-model.bin, load via existing WhisperContext::new_with_params()
- critical_path: Verify whisper-rs 0.15 loads the 2-decoder-layer GGML without errors
</if_go>
<risks>
- technical: Quantization of distil models untested (but fp16 works out of the box)
- external: None — all files already published on HuggingFace
- mitigation: Ship fp16 first, quantize later if needed
</risks>
<alternatives>
- if_blocked: Stay with large-v3-turbo Q5_0 (current, proven)
- simpler_version: Just add distil-large-v3 fp16 as "high quality" GPU option alongside turbo
</alternatives>
</claude_context>
```

**Next Action:** Smoke test distil-large-v3.5 GGML with current build, then add to `download.rs`.

---

## Sources

- [distil-whisper/distil-large-v3-ggml](https://huggingface.co/distil-whisper/distil-large-v3-ggml) — pre-converted GGML files
- [distil-whisper/distil-large-v3.5-ggml](https://huggingface.co/distil-whisper/distil-large-v3.5-ggml) — v3.5 GGML files
- [whisper.cpp Issue #1423](https://github.com/ggml-org/whisper.cpp/issues/1423) — distil-whisper support discussion
- [distil-whisper/distil-large-v3.5 model card](https://huggingface.co/distil-whisper/distil-large-v3.5) — benchmark tables
- [whisper.cpp quantization PR #540](https://github.com/ggml-org/whisper.cpp/pull/540) — quantization support
- [whisper.cpp models README](https://github.com/ggml-org/whisper.cpp/blob/master/models/README.md) — supported quantization formats
