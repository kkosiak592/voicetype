# Feasibility Assessment: Two Installers (CPU + GPU)

## Strategic Summary

The GitHub Actions matrix build for two artifacts is straightforward (~2 hours of work). However, **the auto-updater is the real blocker** — your current setup produces a single `latest.json` pointing to one installer URL. Two installers require two update channels, which Tauri's updater doesn't natively support. This turns a simple CI change into an architectural problem that touches the updater endpoint, installer naming, and update flow.

## What We're Assessing

Producing two separate NSIS installers from GitHub Actions:
- **CPU variant**: Built without CUDA features (`--no-default-features --features whisper,parakeet` with CUDA removed from whisper-rs and parakeet-rs feature lists)
- **GPU/CUDA variant**: Built with full CUDA features (current behavior), bundling redistributable CUDA DLLs

Users pick the right installer from GitHub Releases. Auto-updates keep them on their chosen track.

## Technical Feasibility

### GitHub Actions Matrix Build — Easy
- Add a build matrix with `variant: [cpu, gpu]`
- CPU job skips CUDA toolkit installation, builds with `--no-default-features --features whisper,parakeet` (with a new cpu-safe feature set)
- GPU job keeps current behavior
- Artifacts named: `VoiceType-1.1.0-cpu-x64.nsis.exe` and `VoiceType-1.1.0-gpu-x64.nsis.exe`
- **Effort: ~1-2 hours. No risk.**

### Cargo Feature Restructuring — Easy
Current:
```toml
default = ["whisper", "parakeet"]
whisper-rs = { version = "0.15", features = ["cuda"] }
parakeet-rs = { version = "0.1.9", features = ["cuda", "directml"] }
```
Needed:
```toml
[features]
default = ["whisper", "parakeet"]
whisper = ["dep:whisper-rs", "dep:nvml-wrapper"]
parakeet = ["dep:parakeet-rs"]
cuda = []  # New: gates CUDA on both engines

[dependencies]
whisper-rs = { version = "0.15", optional = true }  # No cuda by default
parakeet-rs = { version = "0.1.9", features = ["directml"], optional = true }  # DirectML always, CUDA optional
```
Plus conditional CUDA activation in build.rs or via feature composition.
- **Effort: ~2-3 hours including testing. Medium risk** (feature flag interactions with patches).

### Auto-Updater — THE BLOCKER

**Current system:**
- `tauri-apps/tauri-action` generates ONE `latest.json` per release
- `latest.json` has ONE URL per platform (`windows-x86_64`)
- App checks `https://github.com/kkosiak592/voicetype/releases/latest/download/latest.json`
- There is no concept of "variant" in the Tauri updater protocol

**Problem:** If you have two installers, which one does `latest.json` point to? A CPU user who auto-updates would get the GPU installer (or vice versa), breaking their system.

**Solutions (ranked by complexity):**

1. **Two `latest.json` files** (e.g., `latest-cpu.json`, `latest-gpu.json`)
   - Requires building the app with different `tauri.conf.json` updater endpoints per variant
   - CPU build points to `latest-cpu.json`, GPU build points to `latest-gpu.json`
   - `tauri-action` would need to run twice with different configs (supported via `--config` flag)
   - Post-build step renames the generated `latest.json` to variant-specific names
   - **Effort: ~3-4 hours. This is the most viable approach.**

2. **Custom update server** that detects variant from a header/query param
   - Overkill for this project. Requires hosting infrastructure.

3. **Disable auto-updater, manual downloads only**
   - Terrible UX regression. Not recommended.

**Verdict: Option 1 is feasible but adds ongoing maintenance burden.**

### CUDA DLL Bundling (GPU variant) — Medium

For the GPU installer, you'd need to bundle redistributable CUDA DLLs:
- `cudart64_12.dll` (~500KB) — redistributable per NVIDIA EULA
- `cublas64_12.dll` (~150MB+) — redistributable per NVIDIA EULA
- `cublasLt64_12.dll` (~150MB+) — redistributable per NVIDIA EULA

**Note:** `nvcuda.dll` is NOT redistributable. It comes from the NVIDIA driver. GPU users must have NVIDIA drivers installed (they already do if they have an NVIDIA GPU).

This means the GPU installer would be ~300MB+ larger than the CPU installer.
- Bundle via `tauri.conf.json` `bundle.resources` or copy in CI step
- **Effort: ~1-2 hours. Low risk.**

### CI Build Time Impact — Minor

Current build: ~20-40 minutes (single variant).
With matrix: Two parallel jobs, each ~20-40 minutes. Total wall-clock time stays the same (parallel), but CI minutes double.

## Resource Feasibility

| Resource | Status |
|----------|--------|
| GitHub Actions minutes | Free tier may be tight (2x Windows builds ~60-80 min). Paid plans fine. |
| Signing keys | Same keys work for both variants |
| Developer maintenance | Two builds to test on each release |
| Storage | GPU installer ~300MB+ larger due to CUDA DLLs |

## External Dependency Feasibility

| Dependency | Status |
|------------|--------|
| `tauri-apps/tauri-action` | Supports `--config` flag for variant builds. Works. |
| NVIDIA CUDA EULA | Allows redistribution of cudart/cublas. Confirmed. |
| GitHub Releases | No issue hosting multiple artifacts per release. |
| Tauri updater protocol | Supports custom endpoints. Two `latest.json` files work. |

## Blockers

| Blocker | Severity | Mitigation |
|---------|----------|------------|
| Auto-updater needs variant-aware endpoints | **High** | Two `latest.json` files with variant-specific updater URLs baked into each build |
| GPU installer size (~300MB+ for CUDA DLLs) | Medium | Expected for CUDA apps. Clearly label on Releases page. |
| Feature flag restructuring touches patched crates | Medium | Test both variants thoroughly. Patches may need adjustment. |
| `nvcuda.dll` not redistributable | Low | Document that GPU variant requires NVIDIA drivers (universal for GPU users) |

## De-risking Options

1. **Start with CPU-only single installer** (drop CUDA from whisper-rs, keep parakeet with DirectML): Zero updater changes, zero complexity increase. Parakeet+DirectML handles GPU. Add CUDA variant later if users request it.

2. **Two installers without auto-update for GPU variant**: Ship GPU as a manual download, CPU variant keeps auto-update. Simpler but worse GPU user experience.

3. **Prototype the two-`latest.json` approach first**: Before full implementation, verify `tauri-action` can produce variant-specific update manifests with a test release.

## Overall Verdict

**Go with conditions.**

It's feasible but the updater complexity is the real cost. The CI matrix build is trivial. The feature restructuring is moderate. The updater variant routing is where most of the effort and ongoing maintenance lives.

### Honest comparison:

| Approach | Effort | Maintenance | UX |
|----------|--------|-------------|-----|
| **Two installers** | ~8-12 hours | Higher (test both, maintain two update channels) | User must choose correctly |
| **Single installer** (drop whisper CUDA, keep parakeet DirectML) | ~2-3 hours | Same as today | Just works everywhere |

The single-installer approach (Option 2 from earlier discussion) achieves 90% of the goal with 20% of the effort. Two installers is standard for CLI tools (llama.cpp) but unusual for desktop apps with auto-updaters.

## Implementation Context

### If Go (Two Installers):
- **Approach**: GitHub Actions matrix + two `latest.json` files + feature flag restructuring
- **Start with**: Feature flag restructuring in Cargo.toml to cleanly separate CPU/GPU builds
- **Critical path**: Verify `tauri-action` produces correct variant-specific `latest.json` files

### Risks:
- Technical: Feature flag interactions with patched crates (esaxx-rs CRT, parakeet-rs vocab)
- External: `tauri-action` behavior with `--config` overrides for updater endpoints
- Mitigation: Test with a pre-release tag (e.g., `v1.2.0-rc1`) before real release

### Alternatives:
- **If blocked**: Single installer with whisper CPU + parakeet DirectML (definitely feasible, ~2-3 hours)
- **Simpler version**: Two installers but only CPU variant gets auto-updates; GPU is manual download from Releases

## Sources
- Tauri v2 updater docs: https://v2.tauri.app/plugin/updater/
- Tauri v2 configuration: https://v2.tauri.app/develop/configuration-files/
- NVIDIA CUDA EULA Attachment A: https://docs.nvidia.com/cuda/eula/index.html
- llama.cpp releases (reference pattern): https://github.com/ggml-org/llama.cpp/releases
- Current release workflow: .github/workflows/release.yml
- Current updater config: src-tauri/tauri.conf.json
