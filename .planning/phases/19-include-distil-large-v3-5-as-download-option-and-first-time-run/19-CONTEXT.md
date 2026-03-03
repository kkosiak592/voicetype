# Phase 19: Include distil-large-v3.5 as download option and first-time run - Context

**Gathered:** 2026-03-03
**Status:** Ready for planning

<domain>
## Phase Boundary

Add distil-large-v3.5 (fp16 GGML) as a downloadable Whisper model option in both the first-run setup flow and the settings model selector. The model sits alongside existing options — no models are removed, no migration logic needed.

</domain>

<decisions>
## Implementation Decisions

### Model lineup
- Add distil-large-v3.5 alongside existing models — do NOT remove large-v3-turbo or any other model
- Final lineup: distil-large-v3.5, large-v3-turbo, parakeet-tdt, small-en (4 models total)
- No migration handling for existing users (still in dev mode)

### Download format
- Ship fp16 GGML as-is from HuggingFace (1.52 GB)
- No quantization — use the pre-built file from `distil-whisper/distil-large-v3.5-ggml` repo
- Different HF repo than current models (current: `ggerganov/whisper.cpp`, distil: `distil-whisper/distil-large-v3.5-ggml`)

### First-run presentation
- Card name: "Distil Large v3.5"
- Show to all users regardless of GPU detection (works on CPU, just slower)
- Not GPU-only — available to all hardware
- Show all applicable models (up to 4 cards for GPU users)

### Recommendation logic
- Keep parakeet-tdt as recommended for GPU users (unchanged — user will adjust after testing)
- Keep small-en as recommended for CPU-only users (unchanged)
- distil-v3.5 appears as a regular (non-recommended) option for now

### Claude's Discretion
- Local filename for the downloaded distil-v3.5 GGML file (upstream is generic `ggml-model.bin`)
- Requirement text on the first-run card (e.g., "GPU recommended" vs "Works on any hardware")
- How to handle the different HF repo URL in download.rs (per-model URL in model_info vs separate function)
- Card quality/description text

</decisions>

<specifics>
## Specific Ideas

- Research confirms distil-large-v3.5 beats large-v3 on short-form WER (7.10% vs 7.25%) and is 1.46x faster than turbo
- fp16 file is at `distil-whisper/distil-large-v3.5-ggml/ggml-model.bin` on HuggingFace (1.52 GB)
- SHA256 checksum will need to be computed (not pre-published)
- Quantization to Q5_0 (~600 MB) is a future optimization if download size becomes an issue

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `download.rs:model_info()` — lookup returns `(filename, sha256, size)` per model ID; add new entry for distil-v3.5
- `download.rs:download_model()` — streaming download with SHA256 validation; reusable as-is
- `download.rs:download_url()` — currently hardcodes `ggerganov/whisper.cpp` repo; needs modification for distil's different HF repo
- `FirstRun.tsx:MODELS` array — hardcoded model cards with id, name, size, quality, requirement, gpuOnly flag
- `ModelSelector.tsx` — settings model selector with download buttons; uses `ModelInfo` interface from backend

### Established Patterns
- Model download: streaming with Channel<DownloadEvent>, progress bar, SHA256 checksum validation
- First-run flow: GPU detection badge, responsive card grid, "Recommended" badge, download + autostart enable
- Model filtering: `gpuOnly` flag filters cards based on GPU detection in FirstRun.tsx

### Integration Points
- `download.rs:model_info()` — add new model entry
- `download.rs:download_url()` — needs to support per-model HF repos
- `FirstRun.tsx:MODELS` array — add distil-v3.5 card definition
- Backend model listing IPC — must include distil-v3.5 in available models for ModelSelector
- `transcribe.rs` — model loading path (should work as-is since whisper-rs loads any GGML)

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 19-include-distil-large-v3-5-as-download-option-and-first-time-run*
*Context gathered: 2026-03-03*
