# Phase 20: Bundle CUDA DLLs in Single Installer with Runtime GPU Fallback - Context

**Gathered:** 2026-03-04
**Status:** Ready for planning
**Source:** Conversation pivot from dual-installer to single-installer approach

<domain>
## Phase Boundary

Bundle redistributable CUDA DLLs (cudart64_12.dll, cublas64_12.dll, cublasLt64_12.dll) directly in the single NSIS installer. On non-NVIDIA machines, GGML/whisper-rs detects no driver and falls back to CPU — the DLLs sit unused but cause no harm. No installer split, no matrix CI, no dual update channels. One installer for all users.

</domain>

<decisions>
## Implementation Decisions

### Single Installer — No Split
- One installer for all users, bundling CUDA DLLs regardless of hardware
- Installer size will increase to ~450-500MB (from ~50-80MB) — acceptable tradeoff vs dual-installer complexity
- Auto-updater stays unchanged: single `latest.json` endpoint, no variant routing

### CUDA DLL Bundling
- Bundle cudart64_12.dll, cublas64_12.dll, cublasLt64_12.dll from CI CUDA toolkit
- DLLs must be placed at the install root (not resources/ subdirectory) so they're on the DLL search path
- Use Tauri `bundle.resources` map syntax to place DLLs at the correct location
- CI step copies DLLs from `$CUDA_PATH/bin/` to staging directory before build

### Runtime GPU Fallback
- GGML/whisper-rs falls back to CPU when CUDA driver is not present but DLLs are on disk — confirmed behavior
- Parakeet/ONNX Runtime already falls back to CPU at runtime — no changes needed
- Existing nvml-wrapper GPU detection continues to work as-is
- No NSIS pre-install GPU check needed — it just works on any machine

### No Variant Tracking
- No CPU/GPU variant indicator in the app UI — there's only one variant now
- No `get_build_variant` command needed
- No config overlays needed

### Claude's Discretion
- Whether to add a CI step that logs the bundled DLL sizes for monitoring
- Whether to add .gitignore entries for staging directory (cuda-libs/)
- Exact Cargo feature restructuring approach (new `cuda` feature flag vs keeping current hardcoded cuda features)
- Whether current `features = ["cuda"]` on whisper-rs and parakeet-rs dependencies needs changing at all (may already work if DLLs are bundled)

</decisions>

<specifics>
## Specific Ideas

- Whisper4Windows (BaderJabri) ships exactly this pattern: single ~660MB installer with bundled CUDA DLLs, runtime GPU detection
- llama.cpp's cudart-llama-bin package (373MB compressed) confirms the DLL sizes: cublasLt64_12.dll alone is ~530MB uncompressed
- The `bundle.resources` map syntax `{"cuda-libs/foo.dll": "foo.dll"}` places DLLs at install root, not in resources/

</specifics>

<code_context>
## Existing Code Insights

### Already Working
- `src-tauri/src/transcribe.rs`: `detect_gpu()` and `detect_gpu_full()` via nvml-wrapper — runtime GPU detection already functional
- `src-tauri/src/transcribe_parakeet.rs`: ONNX Runtime provider selection already falls back to CPU
- `src-tauri/Cargo.toml`: `whisper-rs = { features = ["cuda"] }` and `parakeet-rs = { features = ["cuda", "directml"] }` — CUDA already compiled in
- `.github/workflows/release.yml`: Already installs CUDA 12.6.3 toolkit on CI runner

### Needs Adding
- CI step to copy CUDA DLLs from `$CUDA_PATH/bin/` to a staging directory (e.g., `src-tauri/cuda-libs/`)
- Tauri config update: `bundle.resources` map entries for the three CUDA DLLs
- Possibly `.gitignore` entry for the staging directory

### Integration Points
- CI workflow: Add DLL copy step between CUDA toolkit install and Tauri build
- tauri.conf.json: Add `bundle.resources` map for DLL placement
- No changes to Rust code, frontend, or updater config

</code_context>

<deferred>
## Deferred Ideas

None — scope is minimal by design

</deferred>

---

*Phase: 20-bundle-cuda-dlls-single-installer-with-runtime-gpu-fallback*
*Context gathered: 2026-03-04 via conversation pivot*
