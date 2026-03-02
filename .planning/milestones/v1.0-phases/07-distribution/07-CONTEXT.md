# Phase 7: Distribution - Context

**Gathered:** 2026-03-01
**Status:** Ready for planning

<domain>
## Phase Boundary

First-run model download with progress UI, GPU auto-detection with model recommendation, and a single NSIS installer — making VoiceType installable on any Windows machine regardless of hardware. Models are downloaded on first launch, not bundled with the installer.

</domain>

<decisions>
## Implementation Decisions

### Model lineup
- Drop the medium model entirely — only two models: Large v3 Turbo (GPU) and Small English (CPU)
- Remove medium from `list_models()` in lib.rs and `model_id_to_path()`
- Model cards show file size and quality label (e.g., "Large v3 Turbo — 398 MB — Best accuracy")

### First-run experience
- On first launch with no model, show a first-run setup flow (blocks normal usage until a model is downloaded)
- Brief context: one sentence explaining that offline transcription needs a model file, then GPU detection result + model options + download
- Show both models with the recommended one highlighted based on GPU detection — user confirms which to download
- Users can download additional models later from the Model section in settings

### Download UX
- Download progress, cancel/retry, and error recovery details are Claude's discretion based on standard practice
- Model section in settings gets download buttons for non-downloaded models (ModelSelector already has `downloaded` boolean — needs download trigger wired in)

### Installer
- Standard NSIS installer, details (wizard vs one-click, shortcuts, silent support) are Claude's discretion based on standard practice
- Auto-start with Windows enabled by default (tauri-plugin-autostart already in dependencies)
- Launch after install behavior is Claude's discretion based on standard practice
- Installer must be under 5 MB (models excluded)

### Claude's Discretion
- First-run UI approach (dedicated window vs settings banner — pick best fit for existing code)
- Download progress display (progress bar style, speed/ETA, cancel button)
- Error recovery on failed downloads (auto-retry with resume vs manual retry — standard practice)
- Whether to prompt GPU users who chose small model to upgrade later
- SHA256 checksum validation (required per success criteria)
- Installer flow details (location chooser, shortcuts, silent install)
- Launch after install behavior

</decisions>

<specifics>
## Specific Ideas

- User asked about bundling models with the installer — decided against it due to 200-400 MB size and inability to tailor to GPU vs CPU at package time
- Medium model eliminated because large-v3-turbo is faster AND more accurate on GPU, and medium is too slow on CPU for real-time dictation latency targets

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `detect_gpu()` in `transcribe.rs`: NVML-based GPU detection already implemented — returns `ModelMode::Gpu` or `ModelMode::Cpu`
- `resolve_model_path()` in `transcribe.rs`: knows model filenames and HuggingFace download URLs — currently errors with PowerShell instructions when model missing
- `models_dir()` in `transcribe.rs`: `%APPDATA%/VoiceType/models` path established
- `ModelSelector` component: already has `downloaded` boolean, shows "Not downloaded" state, has `ModelInfo` interface with `id`, `name`, `description`, `recommended`, `downloaded`
- `list_models()` Tauri command: returns model list with download status and GPU-based recommendation
- `tauri-plugin-autostart`: already in Cargo.toml dependencies

### Established Patterns
- Tauri invoke/command pattern for frontend-backend communication
- tauri-plugin-store for settings persistence
- Feature flags (`whisper` feature) for conditional compilation
- Settings panel with sidebar navigation (Sidebar.tsx)
- Tauri event system for real-time data (used for pill overlay audio levels — can be reused for download progress)

### Integration Points
- First-run flow needs to gate before `load_whisper_context()` in startup (`lib.rs` setup function)
- Download progress events: Rust backend → Tauri event → React frontend (same pattern as RMS level streaming)
- `tauri.conf.json` bundle section needs NSIS-specific configuration
- Medium model removal: `list_models()` (lib.rs:686-708), `model_id_to_path()` (lib.rs:656-664)

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 07-distribution*
*Context gathered: 2026-03-01*
