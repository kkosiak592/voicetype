# Phase 20: Implement Dual CPU/GPU Installers with Variant-Specific Auto-Updates - Context

**Gathered:** 2026-03-03
**Status:** Ready for planning

<domain>
## Phase Boundary

Split the single NSIS installer build into two variants (CPU and GPU) with independent auto-update channels, so users without NVIDIA hardware can run VoiceType. Each variant gets its own `latest-{variant}.json` updater manifest. The CI workflow produces both installers in parallel via a matrix build.

</domain>

<decisions>
## Implementation Decisions

### Existing User Migration
- No migration needed — still in dev mode, no real installed base
- Drop the old `latest.json` endpoint entirely; ship only `latest-cpu.json` and `latest-gpu.json` from day one
- No transitional release or backward compatibility alias required

### Installer Naming
- Architecture-first naming convention: `VoiceType-X.Y.Z-x64-cpu.nsis.exe` and `VoiceType-X.Y.Z-x64-gpu.nsis.exe`
- Both variants presented equally on the GitHub Releases page — no "recommended" label on either
- Clear requirement labels: CPU = works on any machine, GPU = requires NVIDIA GPU + drivers

### App Variant Display
- Show the variant in the app UI: "VoiceType 1.2.0 (CPU)" or "VoiceType 1.2.0 (GPU)"
- Visible in the window title or settings/about section so users can verify which variant is installed

### CUDA DLL Bundling
- GPU installer bundles redistributable CUDA DLLs (cudart64_12.dll, cublas64_12.dll, cublasLt64_12.dll) — ~300MB+ extra size is acceptable
- GPU installer includes an NSIS pre-install check for NVIDIA drivers — warns if not detected

### Claude's Discretion
- Whether to also generate `latest.json` as a GPU alias for future backward compatibility
- Whether to use a pre-release tag (e.g., v1.2.0-rc1) to test the CI pipeline before a real release
- CUDA DLL bundling mechanism (Tauri `bundle.resources` vs CI copy step)
- NSIS driver check behavior: warn-only vs block installation (decide based on standard practices)
- Whether CPU build should detect NVIDIA GPU and suggest the GPU version (one-time suggestion)
- Whether CPU build keeps nvml-wrapper or strips it for cleaner separation
- Release body format (table vs bullet list)
- README download section approach (direct links vs Releases page link)

</decisions>

<specifics>
## Specific Ideas

- Naming follows architecture-first convention: `x64-cpu` / `x64-gpu` (not `cpu-x64` / `gpu-x64`)
- Both variants presented as equal choices — user picks based on hardware, no "safe default" positioning
- Research docs already detail the full implementation approach: feature flags, matrix CI, two `latest.json` files, config overlays

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src-tauri/Cargo.toml`: Already has `whisper` and `parakeet` feature flags; needs a new `cuda` feature to gate CUDA across both engines
- `.github/workflows/release.yml`: Single-job workflow to convert to matrix build with `variant: [cpu, gpu]`
- `src-tauri/tauri.conf.json`: Updater config with single endpoint; needs variant-specific config overlays (`tauri.cpu.conf.json`, `tauri.gpu.conf.json`)
- `nvml-wrapper`: Already a dependency gated behind the `whisper` feature — can detect NVIDIA GPUs at runtime

### Established Patterns
- Tauri config overlays via JSON Merge Patch (RFC 7396) — `--config` flag merges overlay into base config
- `tauri-apps/tauri-action` supports `--config` flag and `includeUpdaterJson` for generating update manifests
- Patched crates (esaxx-rs for CRT fix, parakeet-rs for vocab fix) need testing with both CPU and GPU feature configurations
- `createUpdaterArtifacts: "v1Compatible"` is set in base config

### Integration Points
- CI workflow: Matrix build adds `variant` dimension, conditional CUDA toolkit install, variant-specific `--config` and `--features` flags
- Cargo features: New `cuda` feature flag that composes with existing `whisper` and `parakeet` features
- Updater endpoints: Two config overlays change only the `plugins.updater.endpoints` array
- NSIS installer: Custom pre-install page or check for NVIDIA driver detection (GPU variant only)
- App UI: Variant indicator in title bar or settings — needs build-time feature detection passed to frontend

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 20-implement-dual-cpu-gpu-installers-with-variant-specific-auto-updates*
*Context gathered: 2026-03-03*
