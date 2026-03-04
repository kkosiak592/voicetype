# Phase 20: Bundle CUDA DLLs in Single Installer with Runtime GPU Fallback - Research

**Researched:** 2026-03-04
**Domain:** Tauri NSIS bundling, CUDA redistributable DLLs, CI workflow, Windows DLL search order
**Confidence:** HIGH (core mechanics), MEDIUM (exact resource placement path)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Single Installer — No Split**
- One installer for all users, bundling CUDA DLLs regardless of hardware
- Installer size will increase to ~450-500MB (from ~50-80MB) — acceptable tradeoff vs dual-installer complexity
- Auto-updater stays unchanged: single `latest.json` endpoint, no variant routing

**CUDA DLL Bundling**
- Bundle cudart64_12.dll, cublas64_12.dll, cublasLt64_12.dll from CI CUDA toolkit
- DLLs must be placed at the install root (not resources/ subdirectory) so they're on the DLL search path
- Use Tauri `bundle.resources` map syntax to place DLLs at the correct location
- CI step copies DLLs from `$CUDA_PATH/bin/` to staging directory before build

**Runtime GPU Fallback**
- GGML/whisper-rs falls back to CPU when CUDA driver is not present but DLLs are on disk — confirmed behavior
- Parakeet/ONNX Runtime already falls back to CPU at runtime — no changes needed
- Existing nvml-wrapper GPU detection continues to work as-is
- No NSIS pre-install GPU check needed — it just works on any machine

**No Variant Tracking**
- No CPU/GPU variant indicator in the app UI — there's only one variant now
- No `get_build_variant` command needed
- No config overlays needed

### Claude's Discretion
- Whether to add a CI step that logs the bundled DLL sizes for monitoring
- Whether to add .gitignore entries for staging directory (cuda-libs/)
- Exact Cargo feature restructuring approach (new `cuda` feature flag vs keeping current hardcoded cuda features)
- Whether current `features = ["cuda"]` on whisper-rs and parakeet-rs dependencies needs changing at all (may already work if DLLs are bundled)

### Deferred Ideas (OUT OF SCOPE)

None — scope is minimal by design
</user_constraints>

---

## Summary

Phase 20 has three concrete tasks: (1) add a CI step to copy three CUDA DLLs from `$CUDA_PATH/bin/` to a staging directory, (2) update `tauri.conf.json` to bundle those DLLs via the `bundle.resources` map syntax so they land at the install root alongside the exe, and (3) add a `.gitignore` entry for the staging directory. No Rust or frontend changes are needed.

The runtime fallback is well-confirmed: GGML/whisper.cpp silently falls back to CPU when `ggml_cuda_init` fails due to an absent NVIDIA driver, even when the CUDA DLLs are present on disk. This is the behavior the plan depends on. The only genuine technical uncertainty is DLL search-path placement — research confirms Windows searches the application directory (where the exe lives) first, and Tauri's NSIS template places resource files at `$INSTDIR` with the map-specified relative path as the output name, so a flat filename destination puts the DLL next to the exe.

**Primary recommendation:** Use the `bundle.resources` map syntax with flat filename destinations (e.g., `"cuda-libs/cudart64_12.dll": "cudart64_12.dll"`) so DLLs land at `$INSTDIR` alongside the exe. The CI step runs immediately after the CUDA toolkit install step and before the Tauri build step.

---

## Standard Stack

### Core
| Component | Version | Purpose | Why Standard |
|-----------|---------|---------|--------------|
| Jimver/cuda-toolkit | v0.2.21 (already in CI) | Install CUDA 12.6.3 on runner | Already used; provides `$CUDA_PATH` |
| Tauri `bundle.resources` map | Tauri v2 | Bundle DLLs into NSIS installer | Official Tauri mechanism for file bundling |
| NSIS installer | Tauri v2 (already configured) | Delivery vehicle | Already in use; no change |
| CUDA redistributable DLLs | CUDA 12.6.3 | Runtime CUDA support on end-user machines | NVIDIA EULA Attachment A explicitly permits redistribution |

### Supporting
| Component | Version | Purpose | When to Use |
|-----------|---------|---------|-------------|
| `.gitignore` entry for `src-tauri/cuda-libs/` | N/A | Prevent accidental commit of 500MB+ binaries | Staging dir is CI-created, should never be committed |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `bundle.resources` map | `externalBinaries` / sidecar | Sidecars are for executables, not DLLs; wrong mechanism |
| Flat filename destination in map | Subdirectory in map | Subdirectory would put DLLs in `$INSTDIR\subdir\`, off the implicit DLL search path |
| Staging in `src-tauri/cuda-libs/` | Staging directly in `src-tauri/` | Subdirectory keeps the tree clean; both work |

---

## Architecture Patterns

### Recommended Project Structure

```
src-tauri/
├── cuda-libs/              # CI-created staging dir (gitignored)
│   ├── cudart64_12.dll     # Copied from $CUDA_PATH/bin/ by CI step
│   ├── cublas64_12.dll
│   └── cublasLt64_12.dll
├── tauri.conf.json         # Updated with bundle.resources map
└── ...
.github/workflows/
└── release.yml             # Updated with DLL copy step
.gitignore                  # Updated with src-tauri/cuda-libs/
```

### Pattern 1: Tauri v2 Resources Map Syntax for DLL Placement

**What:** Use the map form of `bundle.resources` where the key is the source path (relative to `tauri.conf.json`) and the value is the destination path (relative to `$INSTDIR` in NSIS).

**When to use:** Whenever you need DLLs/files at the install root rather than in a `resources/` subdirectory.

**How it works (NSIS internals):** Tauri's NSIS template runs `SetOutPath $INSTDIR` then uses `File /a "/oname={{destination}}" "{{source}}"` for each resource entry. The destination value is used verbatim as the output filename under `$INSTDIR`. A flat filename (no subdirectory prefix) lands the file directly at `$INSTDIR`.

**Example:**
```json
// Source: https://v2.tauri.app/develop/resources/
{
  "bundle": {
    "resources": {
      "cuda-libs/cudart64_12.dll": "cudart64_12.dll",
      "cuda-libs/cublas64_12.dll": "cublas64_12.dll",
      "cuda-libs/cublasLt64_12.dll": "cublasLt64_12.dll"
    }
  }
}
```

**Why flat filename destination matters:** Windows DLL search order (documented by Microsoft at https://learn.microsoft.com/en-us/windows/win32/dlls/dynamic-link-library-search-order) searches the application directory (directory from which the application loaded) **first**, before System32, Path, etc. The exe is at `$INSTDIR\VoiceType.exe`, so `$INSTDIR` is the application directory. DLLs must land there, not in `$INSTDIR\resources\`.

**Caution — documentation terminology mismatch:** Tauri docs say "target location relative to `$RESOURCE`", but the NSIS template behavior shows resources going to `$INSTDIR` with the destination as the sub-path. Community reports confirm DLLs placed via resources end up "in the root directory" alongside the exe. The docs use an abstract `$RESOURCE` label; for NSIS it equals `$INSTDIR`.

### Pattern 2: CI DLL Staging Before Tauri Build

**What:** Add a bash step in `release.yml` between the CUDA toolkit install and the Tauri build to copy DLLs into the staging directory.

**When to use:** Any time files must be bundled but cannot be committed to git (because of size).

**Example:**
```yaml
# Source: standard shell pattern, validated against existing release.yml structure
- name: Stage CUDA redistributable DLLs
  shell: bash
  run: |
    mkdir -p src-tauri/cuda-libs
    cp "$CUDA_PATH/bin/cudart64_12.dll"    src-tauri/cuda-libs/
    cp "$CUDA_PATH/bin/cublas64_12.dll"    src-tauri/cuda-libs/
    cp "$CUDA_PATH/bin/cublasLt64_12.dll"  src-tauri/cuda-libs/
    echo "Staged DLLs:"
    ls -lh src-tauri/cuda-libs/
```

This step must run **after** `Jimver/cuda-toolkit` (which sets `$CUDA_PATH`) and **before** `tauri-apps/tauri-action` (which triggers the Tauri build and bundler).

### Anti-Patterns to Avoid

- **Committing DLLs to the repository:** cublasLt64_12.dll alone is ~530MB uncompressed. This would bloat the git repo and all clone operations. Always gitignore the staging directory.
- **Using `externalBinaries` for DLLs:** That mechanism is for executable sidecars that Tauri manages with `Command::new_sidecar()`. It adds architecture suffix renaming that breaks DLL loading.
- **Using resources array syntax instead of map syntax:** The array form preserves the original path structure (e.g., `cuda-libs/cudart64_12.dll` → installed at `$INSTDIR/cuda-libs/cudart64_12.dll`). That subdirectory would not be on the DLL search path.
- **Adding `.gitignore` entry after testing locally:** If you test locally first without the gitignore, you may accidentally stage a 500MB+ DLL. Add the gitignore entry in the same commit as the `tauri.conf.json` change.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| DLL search path at install time | Custom NSIS plugin to modify PATH | Use flat destination in `bundle.resources` map | Windows application directory is already first in search order — no PATH modification needed |
| CUDA driver detection at install | NSIS GPU detection script | Nothing — GGML handles it at runtime | Runtime fallback is silent and confirmed; install-time detection adds complexity with no benefit |
| Runtime GPU/CPU mode flag | Rust env var or config file written by NSIS | Nothing — GGML and ort handle selection internally | Both backends already have automatic provider selection |

**Key insight:** The entire phase reduces to two file changes (a CI YAML step and a `tauri.conf.json` resources map) and one gitignore entry. Any additional mechanism is over-engineering.

---

## Common Pitfalls

### Pitfall 1: DLLs Land in resources/ Subdirectory, Not Install Root

**What goes wrong:** Using the array form of `bundle.resources` (list of paths) instead of the map form causes Tauri to preserve the source path structure. `"cuda-libs/cudart64_12.dll"` in array form installs to `$INSTDIR\cuda-libs\cudart64_12.dll` — a subdirectory. The app cannot load the DLL because `$INSTDIR\cuda-libs\` is not on Windows' implicit DLL search path.

**Why it happens:** Developers use the simpler array syntax without realizing it preserves the source subdirectory.

**How to avoid:** Always use the map form with flat filename as destination value when DLLs must be at the install root.

**Warning signs:** App launches and shows `CUDA = 0` on a machine that has a GPU and valid drivers, yet the DLLs are present in the install directory — just in a subdirectory.

### Pitfall 2: Staging Directory Not Created Before Tauri Build Step

**What goes wrong:** The CI step that copies DLLs runs after tauri-action, so the resources map references files that don't exist during the build. Tauri bundler errors out or produces an installer without the DLLs.

**Why it happens:** Step ordering mistake — tauri-action is often placed last as the "build and release" step.

**How to avoid:** The DLL staging step must appear between "Install CUDA Toolkit" and "Build and publish release with Tauri" in `release.yml`.

**Warning signs:** Bundler error like "resource file not found" during CI, or installer produced but DLLs absent at install time.

### Pitfall 3: DLL Version Mismatch (12.x vs exact CUDA version)

**What goes wrong:** whisper.cpp is compiled against CUDA 12.6.3 headers and links against cuBLAS 12.x. If different CUDA major.minor version DLLs are bundled (e.g., from CUDA 12.0), initialization may fail with "CUDA driver version is insufficient" or similar.

**Why it happens:** Using a different CUDA version in the DLL staging step than the one used to compile whisper.cpp.

**How to avoid:** The DLL staging step copies from `$CUDA_PATH/bin/` on the same runner that just ran `Jimver/cuda-toolkit` with `cuda: '12.6.3'`. Same runner, same version, no mismatch.

**Warning signs:** `ggml_cuda_init: failed to initialize CUDA` on a machine with a valid NVIDIA driver.

### Pitfall 4: Rust Cache Interaction — Stale Build Without DLLs

**What goes wrong:** Swatinem/rust-cache caches `src-tauri/target/`. The compiled binary embeds no reference to the DLLs (they're loaded at runtime), so a cache hit can produce a valid binary. But if tauri-action caches the entire bundle, a stale bundle might not include updated DLLs.

**Why it doesn't apply here:** Tauri-action bundles at build time, not from a cache. The Rust cache only covers `target/` (compiled artifacts), not the installer. DLLs are bundled fresh each CI run.

**Warning signs:** N/A for this phase, but watch if Swatinem cache scope ever changes.

### Pitfall 5: CUDA Toolkit Install Timeout

**What goes wrong:** Jimver/cuda-toolkit with `method: network` on `windows-latest` runners can take >15 minutes or hang (documented in issue #382). The `sub-packages` list in the current workflow already minimizes this by installing only `nvcc`, `cudart`, `cublas`, `cublas_dev`, `thrust`, `visual_studio_integration`.

**Why it happens:** NVIDIA's network installer downloads packages sequentially; `windows-latest` runner network speed varies.

**How to avoid:** The current sub-packages list already correctly includes `cudart` and `cublas` (which provide the runtime DLLs). No changes to the install step are needed.

**Warning signs:** CI job times out at 60 minutes on the CUDA install step.

---

## Code Examples

Verified patterns from official sources and the existing codebase:

### tauri.conf.json — resources map for DLL placement

```json
// Source: https://v2.tauri.app/develop/resources/
// Source: NSIS template analysis showing $INSTDIR + destination path behavior
{
  "bundle": {
    "active": true,
    "targets": ["nsis"],
    "createUpdaterArtifacts": "v1Compatible",
    "resources": {
      "cuda-libs/cudart64_12.dll": "cudart64_12.dll",
      "cuda-libs/cublas64_12.dll": "cublas64_12.dll",
      "cuda-libs/cublasLt64_12.dll": "cublasLt64_12.dll"
    },
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "windows": {
      "nsis": {
        "installMode": "currentUser",
        "displayLanguageSelector": false,
        "startMenuFolder": "VoiceType"
      }
    }
  }
}
```

Note: The `icon` array and `windows.nsis` block are copied from the existing config to show the full bundle object. All other existing fields remain.

### release.yml — DLL staging step (insert after CUDA install, before Tauri build)

```yaml
# Source: standard bash pattern + Jimver/cuda-toolkit documentation ($CUDA_PATH output)
- name: Stage CUDA redistributable DLLs
  shell: bash
  run: |
    mkdir -p src-tauri/cuda-libs
    cp "$CUDA_PATH/bin/cudart64_12.dll"    src-tauri/cuda-libs/
    cp "$CUDA_PATH/bin/cublas64_12.dll"    src-tauri/cuda-libs/
    cp "$CUDA_PATH/bin/cublasLt64_12.dll"  src-tauri/cuda-libs/
    echo "Staged DLL sizes:"
    ls -lh src-tauri/cuda-libs/
```

### .gitignore — staging directory entry

```
# CUDA redistributable DLLs staged at build time (CI-created, not committed)
src-tauri/cuda-libs/
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Ship separate CPU+GPU installers | Single installer with bundled CUDA DLLs | 2022–2024 trend (llama.cpp, Whisper4Windows) | Eliminates user choice friction; all users get GPU support when hardware is present |
| Require user to install CUDA Toolkit | Bundle redistributable DLLs | ~2023 with ggml ecosystem maturation | Zero setup on end-user machines with NVIDIA GPUs |
| `bundle.resources` array syntax | `bundle.resources` map syntax | Tauri v2 (stable 2024) | Precise destination path control; required for DLL placement at install root |

**Deprecated/outdated:**
- Tauri v1 resources API: slightly different syntax but same concept; this project is on Tauri v2
- Requiring users to install CUDA Toolkit separately: widespread precedent in llama.cpp and whisper.cpp distributions shows bundling is now the norm

---

## CUDA DLL Sizes and Installer Impact

From community research (llama.cpp release assets, NVIDIA forum data):

| DLL | Approx. uncompressed size | Note |
|-----|--------------------------|------|
| cudart64_12.dll | ~2-4 MB | Small — CUDA runtime |
| cublas64_12.dll | ~50-100 MB | BLAS library |
| cublasLt64_12.dll | ~530 MB | Large — includes kernels for all GPU architectures |
| **Total** | **~600-650 MB uncompressed** | LZMA compression in NSIS will reduce installer to ~450-500 MB |

The installer size increase from ~50-80 MB to ~450-500 MB is consistent with CONTEXT.md's estimate and matches the llama.cpp cudart-llama package (373 MB compressed, per GitHub Releases).

NSIS uses LZMA compression by default. The CUDA DLLs are already compiled binaries that compress moderately well. No NSIS compression configuration changes are needed; the default is sufficient.

---

## CUDA Redistributable License

NVIDIA CUDA EULA Attachment A explicitly lists `cudart.dll`, `cublas.dll`, and `cublasLt.dll` (and their 64-bit/versioned variants) as distributable. Key requirements:
- Application must have "material additional functionality" beyond the included SDK portions (VoiceType is a voice dictation app — clearly satisfies this)
- Include attribution notice: "This software contains source code provided by NVIDIA Corporation"
- Distribute terms consistent with the CUDA agreement

Source: https://docs.nvidia.com/cuda/eula/index.html

**Confidence:** HIGH — Verified against official NVIDIA EULA. Attribution notice should be added to the installer or about screen.

---

## Open Questions

1. **Exact `$RESOURCE` vs `$INSTDIR` placement for Tauri NSIS**
   - What we know: NSIS template uses `SetOutPath $INSTDIR` and `File /a "/oname={{destination}}" "{{source}}"`. Multiple community reports confirm DLLs in resources end up "in the root directory." Tauri docs abstractly say "relative to `$RESOURCE`."
   - What's unclear: Whether `$RESOURCE = $INSTDIR` for NSIS specifically, or if there's an intermediate path prefix added for NSIS that doesn't appear in the template.
   - Recommendation: Test locally before shipping. After `cargo tauri build`, inspect `src-tauri/target/release/bundle/nsis/` installer contents with 7-Zip or install to a temp location and verify DLL placement. If DLLs land in a subdirectory, switch to placing DLLs directly in `src-tauri/` (no staging subdirectory) and use array syntax — multiple reports confirm that approach works.
   - Fallback verified approach: `"resources": ["./cudart64_12.dll", "./cublas64_12.dll", "./cublasLt64_12.dll"]` (array syntax with files in `src-tauri/`) — community reports (GitHub discussion #11382, issue #9525) confirm these land "in the root directory" after NSIS installation.

2. **GGML CPU fallback: silent vs logged**
   - What we know: Issue #2297 on whisper.cpp shows the app continues running on CPU when `CUDA = 0` even when GPU was requested. Application does not crash.
   - What's unclear: Whether the fallback is completely silent to end users or emits a warning log visible at runtime.
   - Recommendation: This is cosmetic only. The app will work on CPU on non-NVIDIA machines regardless. nvml-wrapper's `detect_gpu()` in `transcribe.rs` will return false for no-GPU machines, so Whisper's `use_gpu` flag can be set to false correctly before model init — this may actually prevent even attempting CUDA, making the fallback path cleaner.

3. **Does Rust cache invalidation affect DLL bundling?**
   - What we know: Swatinem/rust-cache caches `src-tauri/target/`. The DLLs are in `src-tauri/cuda-libs/` (outside target). Tauri bundles during build, not from cache.
   - What's unclear: Whether a full cache hit (binary unchanged) still re-runs bundling with the new DLLs.
   - Recommendation: Low risk. Tauri-action runs `tauri build` which always re-runs the bundler. Even with a Rust cache hit (no recompilation), the bundler copies resources fresh.

---

## Sources

### Primary (HIGH confidence)
- https://v2.tauri.app/develop/resources/ — Tauri v2 resources map syntax (Context7 ID: /tauri-apps/tauri-docs)
- https://learn.microsoft.com/en-us/windows/win32/dlls/dynamic-link-library-search-order — Windows DLL search order, application directory first
- https://docs.nvidia.com/cuda/eula/index.html — CUDA EULA Attachment A, redistributable components list
- https://raw.githubusercontent.com/tauri-apps/tauri/dev/crates/tauri-bundler/src/bundle/windows/nsis/installer.nsi — NSIS template source; confirms `SetOutPath $INSTDIR` and `oname={{destination}}` pattern
- Existing codebase: `release.yml`, `tauri.conf.json`, `Cargo.toml` — read directly

### Secondary (MEDIUM confidence)
- https://github.com/tauri-apps/tauri/discussions/11382 — "After installation the DLL is automatically in the root directory" (user-confirmed for NSIS)
- https://github.com/tauri-apps/tauri/issues/9525 — "bundling the dll works great without fiddling with the install" (user-confirmed)
- https://github.com/ggml-org/whisper.cpp/issues/2297 — GGML silently falls back to CPU (`CUDA = 0` output, app keeps running)
- https://github.com/ggml-org/llama.cpp/releases — cudart-llama-bin-win-cuda-12.4-x64.zip is 373 MB compressed (DLL size reference)
- https://forums.developer.nvidia.com/t/windows-dll-sizes/235015 — cublasLt64_11.dll ~531 MB (CUDA 11 baseline)

### Tertiary (LOW confidence)
- https://github.com/Jimver/cuda-toolkit/issues/382 — CUDA toolkit install timeout risk on windows-latest (not version-specific, circumstantial)
- CONTEXT.md references Whisper4Windows as precedent — uses MSI not NSIS; not directly comparable for DLL placement mechanics

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all components already in use; only configuration changes
- Architecture: HIGH for CI step; MEDIUM for exact NSIS resource destination path (open question #1)
- Pitfalls: HIGH — all verified against official sources or community reports

**Research date:** 2026-03-04
**Valid until:** 2026-06-04 (stable APIs; CUDA EULA and Tauri NSIS behavior are not fast-moving)
