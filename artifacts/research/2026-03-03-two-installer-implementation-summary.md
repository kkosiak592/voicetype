# Two-Installer Implementation: CPU + GPU with Auto-Updates

## Problem

The current release workflow builds a single installer with CUDA features enabled by default (`whisper-rs[cuda]` + `parakeet-rs[cuda]`). This dynamically links against CUDA runtime DLLs (`cudart64_12.dll`, `cublas64_12.dll`, `nvcuda.dll`). When a user without CUDA/NVIDIA drivers runs the installer, Windows can't load the exe — it crashes before any code executes.

## Decision

Ship **two separate NSIS installers** from GitHub Releases:
- `VoiceType-X.Y.Z-cpu-x64.nsis.exe` — works on any machine, no CUDA required
- `VoiceType-X.Y.Z-gpu-x64.nsis.exe` — requires NVIDIA GPU + drivers, bundles CUDA runtime DLLs

Both variants support auto-updates. Each stays on its own update channel.

This is the industry standard approach (used by llama.cpp, whisper.cpp, and similar GPU-accelerated native apps).

## Why Two Installers (Not Single Installer)

Whisper may become the default transcription engine. Whisper with CUDA runs at 7-10x realtime; without CUDA it drops to 2-5x on CPU. Unlike parakeet-rs (which uses ONNX Runtime execution providers that fall back gracefully), whisper-rs with the `cuda` feature dynamically links CUDA DLLs at the OS level — the binary won't start without them. There's no runtime fallback possible.

## What Changes

### 1. Cargo Feature Flags (src-tauri/Cargo.toml)

**Current:**
```toml
[features]
default = ["whisper", "parakeet"]
whisper = ["dep:whisper-rs", "dep:nvml-wrapper"]
parakeet = ["dep:parakeet-rs"]

[dependencies]
whisper-rs = { version = "0.15", features = ["cuda"], optional = true }
parakeet-rs = { version = "0.1.9", features = ["cuda", "directml"], optional = true }
```

**Target:**
```toml
[features]
default = ["whisper", "parakeet"]
whisper = ["dep:whisper-rs", "dep:nvml-wrapper"]
parakeet = ["dep:parakeet-rs"]
cuda = []  # New feature flag: gates CUDA across all engines

[dependencies]
# whisper-rs: CUDA controlled by build-time cfg, not hardcoded feature
whisper-rs = { version = "0.15", optional = true }                              # CPU by default
parakeet-rs = { version = "0.1.9", features = ["directml"], optional = true }   # DirectML always available
```

When `cuda` feature is enabled:
- whisper-rs gets compiled with `features = ["cuda"]`
- parakeet-rs gets `features = ["cuda", "directml"]`

When `cuda` feature is NOT enabled:
- whisper-rs is CPU-only (no CUDA DLL dependency)
- parakeet-rs has DirectML (GPU on any DX12 hardware) + CPU fallback

**Note:** The exact mechanism for conditionally enabling whisper-rs CUDA may require a build.rs adjustment or feature composition. The patched crates (esaxx-rs CRT fix, parakeet-rs vocab fix) need to be tested with both configurations.

### 2. GitHub Actions Workflow (.github/workflows/release.yml)

**Current:** Single build job, installs CUDA, produces one installer + one `latest.json`.

**Target:** Matrix build with two variants running in parallel.

```yaml
jobs:
  build:
    runs-on: windows-latest
    strategy:
      matrix:
        variant: [cpu, gpu]
    steps:
      # Common setup (Node.js, Rust, LLVM)

      # CUDA setup — GPU only
      - name: Install CUDA Toolkit
        if: matrix.variant == 'gpu'
        uses: Jimver/cuda-toolkit@v0.2.30
        with:
          cuda: '12.6.3'
          sub-packages: '["nvcc", "cudart", "cublas", "cublas_dev", "thrust", "visual_studio_integration"]'

      # Build with variant-specific features and updater config
      - name: Build (CPU)
        if: matrix.variant == 'cpu'
        uses: tauri-apps/tauri-action@v0
        with:
          args: --config src-tauri/tauri.cpu.conf.json
          # No --features cuda

      - name: Build (GPU)
        if: matrix.variant == 'gpu'
        uses: tauri-apps/tauri-action@v0
        with:
          args: --features cuda --config src-tauri/tauri.gpu.conf.json
        env:
          CMAKE_CUDA_ARCHITECTURES: "61;75;86;89"

      # Rename latest.json to variant-specific name
      - name: Rename update manifest
        run: mv latest.json latest-${{ matrix.variant }}.json

      # Upload variant-specific artifacts
```

**CI time:** Two parallel Windows builds (~20-40 min each). Wall-clock time stays the same. CI minutes double.

### 3. Auto-Updater (Two Update Channels)

**Current:** Single endpoint in `tauri.conf.json`:
```json
"endpoints": ["https://github.com/kkosiak592/voicetype/releases/latest/download/latest.json"]
```

**Target:** Two variant-specific config overlays.

`src-tauri/tauri.cpu.conf.json`:
```json
{
  "plugins": {
    "updater": {
      "endpoints": [
        "https://github.com/kkosiak592/voicetype/releases/latest/download/latest-cpu.json"
      ]
    }
  }
}
```

`src-tauri/tauri.gpu.conf.json`:
```json
{
  "plugins": {
    "updater": {
      "endpoints": [
        "https://github.com/kkosiak592/voicetype/releases/latest/download/latest-gpu.json"
      ]
    }
  }
}
```

These are Tauri config overlays — they merge with the base `tauri.conf.json` via JSON Merge Patch (RFC 7396). Only the `endpoints` field changes; everything else (signing key, bundle settings, etc.) stays in the base config.

**Result:** A CPU-installed app only ever checks `latest-cpu.json`, a GPU-installed app only ever checks `latest-gpu.json`. Users never cross update channels.

### 4. CUDA DLL Bundling (GPU Variant Only)

The GPU installer needs to include redistributable CUDA DLLs:

| DLL | Size | Redistributable? |
|-----|------|------------------|
| `cudart64_12.dll` | ~500KB | Yes (NVIDIA EULA Attachment A) |
| `cublas64_12.dll` | ~150MB+ | Yes |
| `cublasLt64_12.dll` | ~150MB+ | Yes |
| `nvcuda.dll` | N/A | **No** — part of NVIDIA driver, user must have drivers installed |

Bundle via Tauri resources in the GPU config overlay:
```json
{
  "bundle": {
    "resources": ["native-libs/*.dll"]
  }
}
```

Or collect them in the CI step from `$CUDA_PATH/bin/` and place them next to the exe.

**GPU installer will be ~300MB+ larger** than the CPU installer due to these DLLs.

### 5. Installer Naming

**Current artifacts per release:**
```
VoiceType-1.1.0-x64.nsis.exe
VoiceType-1.1.0-x64.nsis.exe.sig
latest.json
```

**New artifacts per release:**
```
VoiceType-1.1.0-cpu-x64.nsis.exe
VoiceType-1.1.0-cpu-x64.nsis.exe.sig
latest-cpu.json

VoiceType-1.1.0-gpu-x64.nsis.exe
VoiceType-1.1.0-gpu-x64.nsis.exe.sig
latest-gpu.json
```

### 6. Release Checklist Updates (RELEASING.md)

The release process stays mostly the same:
1. Bump version in 3 files (unchanged)
2. Update CHANGELOG (unchanged)
3. Commit, tag, push (unchanged)
4. CI builds both variants in parallel (automatic)
5. **New:** Verify both installers on GitHub Releases page
6. **New:** Test CPU installer on a machine without NVIDIA GPU
7. **New:** Test GPU installer on a machine with NVIDIA GPU

## What Each Variant Gets

| Capability | CPU Installer | GPU Installer |
|------------|--------------|---------------|
| Whisper engine | CPU-only (small/medium models) | CUDA-accelerated (large models) |
| Parakeet engine | DirectML (any DX12 GPU) + CPU fallback | CUDA + DirectML + CPU fallback |
| Whisper speed | 2-5x realtime | 7-10x realtime |
| Parakeet speed | 15-25x realtime (DirectML) | 18-30x realtime (CUDA) |
| Installer size | ~current size | ~current + 300MB (CUDA DLLs) |
| Works without NVIDIA | Yes | No (requires NVIDIA drivers) |
| Auto-updates | Yes (cpu channel) | Yes (gpu channel) |

## Effort Estimate

| Task | Effort |
|------|--------|
| Cargo feature restructuring + testing | ~2-3 hours |
| GitHub Actions matrix workflow | ~2-3 hours |
| Updater config overlays (cpu/gpu .json) | ~1 hour |
| CUDA DLL bundling in GPU variant | ~1-2 hours |
| Testing both variants end-to-end | ~2-3 hours |
| RELEASING.md update | ~30 min |
| **Total** | **~8-12 hours** |

## Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| Feature flag interactions with patched crates | Medium | Test CPU build early — esaxx-rs and parakeet-rs patches may behave differently without CUDA |
| `tauri-action` config overlay behavior | Medium | Test with a pre-release tag (e.g., `v1.2.0-rc1`) before real release |
| `latest-cpu.json` / `latest-gpu.json` naming with tauri-action | Medium | May need a post-build rename step; verify the action's `includeUpdaterJson` output path |
| GPU installer size (300MB+) | Low | Expected for CUDA apps. Label clearly on Releases page. |
| User picks wrong installer | Low | Clear naming + README instructions. CPU variant is the safe default. |

## Existing Users

Current users already have the GPU variant (since CUDA was always enabled). On the first release with two variants:
- The existing `latest.json` endpoint stops being generated
- **Migration needed:** Either keep generating `latest.json` pointing to the GPU variant (for one transitional release), or accept that existing users will need to manually download the new GPU installer once
- Alternatively, keep `latest.json` as an alias for `latest-gpu.json` to maintain backward compatibility
