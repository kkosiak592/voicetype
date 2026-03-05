# Feasibility Assessment: Local LLM Transcription Refinement

## Strategic Summary

**Feasible with conditions.** Sub-1-second refinement is achievable on NVIDIA GPUs with <6GB VRAM using quantized 0.5B-1.5B models via llama.cpp sidecar, but only with warm inference (model preloaded in VRAM). The main risk is stacked latency — transcription + refinement combined may feel sluggish. CPU-only is not viable for sub-1-second. Per-app configurable formatting (light cleanup to full summarization) is architecturally straightforward given the existing settings system.

## What We're Assessing

Adding an optional local LLM refinement step to the voice-to-text pipeline. After Whisper/Parakeet/Moonshine transcribes audio, an open-source LLM would clean up, reformat, and restructure the text based on configurable per-app templates. Different apps would get different formatting treatments — from light cleanup (fix punctuation) to structured formatting (bullets, templates) to full rewrite/summarization.

---

## Technical Feasibility

**Can we build it?**

### Models That Fit <6GB VRAM (Q4_K_M quantization)

| Model | VRAM | Est. Speed (mid-range GPU) | Text Cleanup Quality |
|-------|------|---------------------------|---------------------|
| Qwen2.5-0.5B-Instruct | ~1.0 GB | 300-450 tok/s | Adequate for light cleanup |
| Qwen2.5-1.5B-Instruct | ~1.7 GB | 200-300 tok/s | Good for structured formatting |
| Gemma 2 2B | ~1.7-2.0 GB | 150-250 tok/s | Strong quality-per-parameter |
| Qwen2.5-3B-Instruct | ~2.6 GB | 100-180 tok/s | Strong for complex reformatting |
| Phi-3.5 mini (3.8B) | ~2.4 GB | 80-150 tok/s | Excellent but slower |

All fit comfortably in <6GB VRAM even alongside a Whisper model.

Source: [GPUStack Qwen2.5 benchmarks](https://gpustack.ai/running-full-qwen-2-5-series/) — RTX 4080 Q4_K_M: 0.5B=454 tok/s, 1.5B=301 tok/s, 3B=202 tok/s.

### Latency Analysis (warm inference, 100-150 token output)

| Model | Estimated Total Latency | Sub-1s Achievable? |
|-------|------------------------|--------------------|
| Qwen2.5-0.5B Q4_K_M | 400-700ms | Yes |
| Qwen2.5-1.5B Q4_K_M | 600-1000ms | Borderline — yes for short chunks |
| Gemma 2 2B Q4_K_M | 700-1100ms | Borderline |
| Qwen2.5-3B Q4_K_M | 800-1200ms | No for longer outputs |

**Critical**: Model must be preloaded at app startup and kept warm in VRAM. Cold-start takes 3-8 seconds — completely non-viable per-request.

### Inference Engine: llama.cpp as Tauri Sidecar (Recommended)

Bundle `llama-server.exe` (prebuilt with CUDA) as a Tauri sidecar process:

- Spawned on app startup, runs as background process
- Exposes OpenAI-compatible REST API at `localhost:PORT`
- Rust backend calls `/v1/chat/completions` after transcription
- Tauri 2 already supports sidecars via `tauri-plugin-shell`
- Reference: [dillondesilva/tauri-local-lm](https://github.com/dillondesilva/tauri-local-lm)
- MIT license, production-proven, best Windows + CUDA support
- llama.cpp is **1.8x faster** than Ollama ([source](https://www.arsturn.com/blog/ollama-vs-llama-cpp-which-should-you-use-for-local-llms))

**Why not alternatives?**

| Engine | Problem |
|--------|---------|
| Ollama | 1.8x slower, CORS issues with Tauri ([#10507](https://github.com/ollama/ollama/issues/10507)), model management overhead |
| vLLM | No native Windows support |
| mistral.rs | Broken Windows CUDA build ([#1122](https://github.com/EricLBuehler/mistral.rs/issues/1122)) |
| candle (Rust) | 2-5x slower than llama.cpp CUDA kernels |
| ONNX Runtime | Variable-length input perf degradation ([#15394](https://github.com/microsoft/onnxruntime/issues/15394)) |
| llama-cpp-2 crate (in-process) | Requires bundling CUDA DLLs (~hundreds of MB), complex distribution |

### Pipeline Integration

Current: `Record → VAD → Transcribe → Corrections → Inject`

New: `Record → VAD → Transcribe → Corrections → [LLM Refinement] → Inject`

The LLM step slots naturally between corrections and injection. Existing regex corrections should run first to fix domain vocabulary before the LLM processes text.

### Per-App Configuration Design

The app already detects the active window. Per-app LLM settings would be:

```json
{
  "app_profiles": {
    "slack.exe": {
      "refinement_enabled": true,
      "refinement_level": "structured",
      "formatting_template": "Format as concise bullet points"
    },
    "outlook.exe": {
      "refinement_enabled": true,
      "refinement_level": "full",
      "formatting_template": "Format as professional email"
    },
    "code.exe": {
      "refinement_enabled": false
    }
  }
}
```

UI: Dropdown of presets (light cleanup, bullet points, email, summary, custom) per app.

### Model Quality for This Task

Text refinement is a constrained task — output is mostly determined by input, no hallucination needed, simple instructions. This is one of the few tasks where 1-3B models perform close to 7B+ models. The 2024+ generation of small models (Qwen2.5, Phi-3.5, Gemma 2) all use knowledge distillation, further closing the gap.

**Caution**: Research shows LLM post-processing can over-edit high-quality Whisper transcripts. System prompts should be conservative — fix formatting and filler words, don't "improve" content.

- **Known approaches:** Yes — llama.cpp sidecar well-documented
- **Technology maturity:** Proven
- **Technical risks:**
  - VRAM contention between Whisper + LLM models (Medium)
  - Stacked latency: transcription + refinement exceeds user tolerance (Medium)
  - 0.5B model quality for complex reformatting (Low-Medium)
  - Bundle size: model file (1-2.5GB) + llama-server.exe (~50MB) (Low)

**Technical verdict: Feasible**

---

## Resource Feasibility

**Do we have what we need?**

- **Skills**: Rust + Tauri already in use. HTTP calls to llama-server are trivial. No new language/framework.
- **Infrastructure**: llama.cpp prebuilt binaries available. GGUF models free on HuggingFace.
- **Distribution**: Model files (1-2.5GB) downloaded by user — same UX pattern as current Whisper model downloads.
- **Development effort**: Medium — sidecar lifecycle management, settings UI for per-app templates, prompt engineering.

**Resource verdict: Feasible**

---

## External Dependency Feasibility

- **llama.cpp**: MIT license, 60k+ stars, extremely active. Zero abandonment risk.
- **GGUF models**: Standard format, HuggingFace hosts thousands. Multiple model families as fallback.
- **CUDA**: Stable, backwards-compatible.
- **No API keys or cloud dependencies.** Everything runs locally.

**External verdict: Feasible**

---

## Blockers

| Blocker | Severity | Mitigation |
|---------|----------|------------|
| VRAM contention (Whisper + LLM both in VRAM) | Medium | Use 0.5B-1.5B LLM (small footprint); or unload transcription model before LLM, reload after |
| Stacked latency (transcription + refinement) | Medium | Make refinement optional per-app; use 0.5B for speed-sensitive apps; show progress indicator |
| Cold start latency (model load: 3-8s) | High | Keep model loaded in memory after first use; lazy-load on first refinement request |
| Model download size (1-2.5GB) | Low | Reuse existing model download/progress UI pattern |
| 0.5B quality for complex formatting | Low-Medium | Default to 1.5B; let users choose; conservative prompts |
| CPU-only systems | Medium | Disable refinement or warn about 3-10s delay; SmolLM2-135M as CPU fallback |

---

## CPU Fallback Reality Check

| Model | CPU Speed (est.) | 150-token output | Viable for sub-1s? |
|-------|-----------------|-------------------|---------------------|
| SmolLM2-135M | 100-300 tok/s | 0.5-1.5s | Maybe, very limited quality |
| Qwen2.5-0.5B | 30-50 tok/s | 3-5s | No |
| Qwen2.5-1.5B | 15-25 tok/s | 6-10s | No |

**Recommendation**: GPU-only feature. Disable on CPU systems or offer degraded mode with explicit warning.

---

## De-risking Options

- **Prototype first**: Build sidecar integration + test Qwen2.5-1.5B latency on actual hardware before full UI. Cost: 1-2 days. Validates the core assumption.
- **Tiered models**: Ship 0.5B (fast, light) and 1.5B (better, slightly slower). User chooses based on GPU. Minimal extra cost.
- **Conservative prompts**: Well-crafted system prompts prevent over-editing. Start minimal, let users escalate. Cost: prompt engineering time only.
- **Async refinement**: For apps where 1s+ is OK, inject raw first, replace with refined text after. Cost: complexity for deferred injection.

---

## Overall Verdict

**Go with conditions**

### Conditions:
1. **GPU required** for acceptable performance (disable or degrade gracefully on CPU-only)
2. **Model must be preloaded** and kept warm — no cold-start per request
3. **Start with Qwen2.5-1.5B Q4_K_M** as default; offer 0.5B for speed, 3B for quality
4. **Prototype first** — benchmark sidecar latency on actual GPU before building full UI
5. **Keep refinement optional** per-app — users who don't need it shouldn't pay the latency cost

### Implementation Context

```
approach: llama.cpp server as Tauri sidecar, OpenAI-compatible REST API
start_with: Prototype sidecar lifecycle management + single refinement call
critical_path: Warm inference latency must be under 1s on target GPU hardware
```

```
risks:
  technical: VRAM contention with Whisper model, stacked latency
  external: None significant (all local, open-source)
  mitigation: Tiered model sizes, optional per-app toggle, VRAM budgeting
```

```
alternatives:
  if_blocked: Use ONNX Runtime GenAI (already have ONNX infra) or candle Rust crate
  simpler_version: Light cleanup only (punctuation, caps, filler words) with 0.5B model — definitely feasible and fast
```

**Next Action**: Prototype the llama-server sidecar integration. Benchmark Qwen2.5-1.5B Q4_K_M warm inference on your GPU. If latency is acceptable, proceed to per-app settings UI and prompt engineering.

---

## Sources

- [GPUStack Qwen2.5 Full Series Performance](https://gpustack.ai/running-full-qwen-2-5-series/)
- [Phi-3.5-mini-instruct GGUF sizes](https://huggingface.co/bartowski/Phi-3.5-mini-instruct-GGUF)
- [LocalScore consumer GPU benchmarks](https://www.localscore.ai/model/1)
- [Ollama vs llama.cpp speed comparison](https://www.arsturn.com/blog/ollama-vs-llama-cpp-which-should-you-use-for-local-llms)
- [mistral.rs Windows CUDA failure #1122](https://github.com/EricLBuehler/mistral.rs/issues/1122)
- [Ollama CORS with Tauri #10507](https://github.com/ollama/ollama/issues/10507)
- [Tauri local LLM reference](https://github.com/dillondesilva/tauri-local-lm)
- [llama-cpp-2 Rust crate](https://crates.io/crates/llama-cpp-2)
- [CPU inference benchmarks](https://dev.to/maximsaplin/running-local-llms-cpu-vs-gpu-a-quick-speed-test-2cjn)
- [AMD CPU llama.cpp acceleration](https://www.amd.com/en/blogs/2024/accelerating-llama-cpp-performance-in-consumer-llm-applications.html)
- [STT cleanup system prompt](https://github.com/danielrosehill/STT-Basic-Cleanup-System-Prompt)
- [SmolLM2-1.7B](https://huggingface.co/HuggingFaceTB/SmolLM2-1.7B)
- [Gemma 2 2B](https://apxml.com/models/gemma-2-2b)
- [ONNX Runtime DirectML issue #15394](https://github.com/microsoft/onnxruntime/issues/15394)
- [Qwen2.5-0.5B-Instruct-GGUF](https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF)
- [Qwen2.5-1.5B-Instruct-GGUF](https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF)
