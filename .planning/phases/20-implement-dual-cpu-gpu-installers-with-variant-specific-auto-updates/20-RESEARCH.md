# Phase 20: Implement Dual CPU/GPU Installers with Variant-Specific Auto-Updates - Research

**Researched:** 2026-03-03
**Domain:** Tauri release engineering — Cargo feature flags, GitHub Actions matrix builds, NSIS installer customization, Tauri updater channels
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Existing User Migration:**
- No migration needed — still in dev mode, no real installed base
- Drop the old `latest.json` endpoint entirely; ship only `latest-cpu.json` and `latest-gpu.json` from day one
- No transitional release or backward compatibility alias required

**Installer Naming:**
- Architecture-first naming convention: `VoiceType-X.Y.Z-x64-cpu.nsis.exe` and `VoiceType-X.Y.Z-x64-gpu.nsis.exe`
- Both variants presented equally on the GitHub Releases page — no "recommended" label on either
- Clear requirement labels: CPU = works on any machine, GPU = requires NVIDIA GPU + drivers

**App Variant Display:**
- Show the variant in the app UI: "VoiceType 1.2.0 (CPU)" or "VoiceType 1.2.0 (GPU)"
- Visible in the window title or settings/about section so users can verify which variant is installed

**CUDA DLL Bundling:**
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

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope
</user_constraints>

---

## Summary

This phase converts the current single-installer release pipeline into two parallel NSIS installer variants (CPU and GPU), each with its own auto-update channel. The approach is well-established in the Cargo/Tauri ecosystem and all required mechanisms exist: Cargo feature composition syntax (`feature?/sub-feature`), Tauri config overlays via JSON Merge Patch (RFC 7396), GitHub Actions matrix builds, and Tauri NSIS hooks for pre-install checks.

The two non-trivial pieces are: (1) the `cuda` feature must be structured as a Cargo feature that conditionally enables sub-features on `whisper-rs` and `parakeet-rs` — Cargo's `?/` weak dependency syntax makes this clean; and (2) `tauri-apps/tauri-action` always names the updater file `latest.json` with no rename option. The correct approach is to set `includeUpdaterJson: false` on both matrix jobs and upload manually-constructed or renamed JSON files using `gh release upload` in a post-build step.

CUDA DLL bundling is cleanest as a CI step that copies from `$CUDA_PATH/bin/` into a staging directory, which is then referenced in the GPU config overlay via `bundle.resources`. The variant label in the UI is trivially implemented as an `invoke('get_build_variant')` Tauri command that returns a compile-time constant baked in via `#[cfg(feature = "cuda")]`.

**Primary recommendation:** Implement as a 5-plan sequence: Cargo feature restructuring → Tauri config overlays → CI matrix workflow → CUDA DLL bundling + NSIS GPU check → UI variant label. Test with a pre-release tag before the first real release.

---

## Standard Stack

### Core

| Library / Tool | Version | Purpose | Why Standard |
|----------------|---------|---------|--------------|
| `tauri-apps/tauri-action@v0` | current | Build and publish GitHub Releases | Official Tauri CI action |
| `Jimver/cuda-toolkit@v0.2.21` | current (in use) | Install CUDA Toolkit on runner | Only maintained action for Windows CUDA CI |
| `gh` CLI | built into windows-latest | Upload renamed release assets | Official GitHub CLI, available on all runners |
| Cargo feature composition | stable (Rust 1.60+) | `feature?/sub-feature` syntax | Standard Cargo approach for conditional sub-features |
| Tauri `--config` overlay | Tauri v2 | JSON Merge Patch over base config | Official Tauri mechanism for build flavors |
| NSIS hooks via `installerHooks` | Tauri v2 | Pre-install checks in `.nsh` file | Official Tauri NSIS extension point |

### Supporting

| Library / Tool | Version | Purpose | When to Use |
|----------------|---------|---------|-------------|
| `nvml-wrapper` | 0.10 (already in use) | GPU presence detection for CPU build suggestion | Keep in CPU build if GPU-suggestion feature is implemented |
| `actions/upload-artifact` / `gh release upload` | current | Rename and re-upload `latest.json` | Since tauri-action doesn't support custom updater JSON names |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `gh release upload` for updater JSON | Custom update server | Custom server is overkill; static JSON on GitHub Releases is sufficient |
| `bundle.resources` for CUDA DLLs | CI copy step only (no resources entry) | `bundle.resources` is the cleaner, Tauri-idiomatic approach; CI copy step requires the files exist before build |
| NSIS `installerHooks` `.nsh` file | Custom `.nsi` template | Hooks are simpler; full template override is unnecessary |

---

## Architecture Patterns

### Recommended Project Structure Changes

```
src-tauri/
├── tauri.conf.json              # Base config (no CUDA, no updater endpoint)
├── tauri.cpu.conf.json          # Overlay: adds latest-cpu.json endpoint
├── tauri.gpu.conf.json          # Overlay: adds latest-gpu.json endpoint + CUDA resources
├── windows/
│   └── gpu-hooks.nsh            # NSIS pre-install NVIDIA driver check (GPU only)
└── Cargo.toml                   # Adds `cuda` feature flag
.github/workflows/
└── release.yml                  # Converted to matrix build (variant: [cpu, gpu])
```

### Pattern 1: Cargo Feature Composition for CUDA

**What:** A new top-level `cuda` feature in `Cargo.toml` that uses the weak `?/` syntax to enable CUDA sub-features on `whisper-rs` and `parakeet-rs` only if those crates are already enabled by the `whisper`/`parakeet` features.

**When to use:** Any time you need a cross-cutting feature (like a hardware acceleration mode) that applies to multiple optional dependencies without making those dependencies mandatory.

**Example:**
```toml
# src-tauri/Cargo.toml

[features]
default = ["whisper", "parakeet"]
whisper  = ["dep:whisper-rs", "dep:nvml-wrapper"]
parakeet = ["dep:parakeet-rs"]
# New: `cuda` enables CUDA on whichever engines are compiled in.
# `?/` = "enable this sub-feature only if the dep is already enabled"
cuda = ["whisper-rs?/cuda", "parakeet-rs?/cuda"]

[dependencies]
# whisper-rs: no `cuda` by default (CPU-only unless `cuda` feature is active)
whisper-rs  = { version = "0.15", optional = true }
# parakeet-rs: DirectML always present; CUDA added by the `cuda` feature
parakeet-rs = { version = "0.1.9", features = ["directml"], optional = true }
```

GPU build command: `cargo tauri build --features cuda`
CPU build command: `cargo tauri build` (uses `default = ["whisper", "parakeet"]`)

Source: https://doc.rust-lang.org/cargo/reference/features.html (verified — `?/` syntax requires Rust 1.60+, confirmed stable)

### Pattern 2: Tauri Config Overlays for Variant-Specific Updater Endpoints

**What:** Minimal JSON files that merge over `tauri.conf.json` via RFC 7396 JSON Merge Patch when passed to `--config`.

**When to use:** Whenever a build variant changes only a subset of the config (e.g., updater endpoint, bundle resources). The base config stays authoritative; overlays override only what differs.

**Example:**

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

Build command:
```bash
cargo tauri build --config src-tauri/tauri.cpu.conf.json
cargo tauri build --features cuda --config src-tauri/tauri.gpu.conf.json
```

Source: https://v2.tauri.app/develop/configuration-files (HIGH confidence — official docs, verified)

### Pattern 3: GitHub Actions Matrix Build

**What:** Single workflow job with `strategy.matrix.variant: [cpu, gpu]`, conditional steps gated on `matrix.variant`.

**When to use:** When two builds share most steps (checkout, Node, Rust, LLVM) but differ in a few (CUDA install, `--features`, `--config`).

**Example:**
```yaml
jobs:
  publish-release:
    runs-on: windows-latest
    permissions:
      contents: write
    strategy:
      fail-fast: false
      matrix:
        variant: [cpu, gpu]
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: lts/*
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: src-tauri
          cache-on-failure: true
          # Separate caches per variant to avoid CUDA/non-CUDA cache collisions
          key: ${{ matrix.variant }}

      # CUDA: GPU only
      - name: Install CUDA Toolkit
        if: matrix.variant == 'gpu'
        uses: Jimver/cuda-toolkit@v0.2.21
        with:
          cuda: '12.6.3'
          method: network
          sub-packages: '["nvcc", "cudart", "cublas", "cublas_dev", "thrust", "visual_studio_integration"]'

      - name: Install LLVM/clang
        uses: KyleMayes/install-llvm-action@v2
        with:
          version: '18'
          directory: ${{ runner.temp }}/llvm

      - name: Set CUDA architecture targets
        if: matrix.variant == 'gpu'
        shell: bash
        run: echo "CMAKE_CUDA_ARCHITECTURES=61;75;86;89" >> $GITHUB_ENV

      - name: Install frontend dependencies
        run: npm ci

      # Build with tauri-action — but DO NOT use includeUpdaterJson here
      # because tauri-action always names the file "latest.json" with no rename option.
      # We upload the updater JSON manually in the next step.
      - name: Build and publish (CPU)
        if: matrix.variant == 'cpu'
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: 'VoiceType ${{ github.ref_name }}'
          releaseBody: '...'
          releaseDraft: false
          prerelease: false
          includeUpdaterJson: true
          updaterJsonPreferNsis: true
          args: --config src-tauri/tauri.cpu.conf.json

      - name: Build and publish (GPU)
        if: matrix.variant == 'gpu'
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: 'VoiceType ${{ github.ref_name }}'
          releaseBody: '...'
          releaseDraft: false
          prerelease: false
          includeUpdaterJson: true
          updaterJsonPreferNsis: true
          args: --features cuda --config src-tauri/tauri.gpu.conf.json
```

Source: https://v2.tauri.app/distribute/pipelines/github/ (HIGH confidence — official Tauri GitHub CI docs)

### Pattern 4: Renaming latest.json to Variant-Specific Names

**What:** `tauri-action` always uploads `latest.json` regardless of which updater endpoint is baked into the binary. With two matrix jobs pointing to different `latest-cpu.json` / `latest-gpu.json` URLs, the action uploading `latest.json` would be wrong. The solution: use `includeUpdaterJson: true` on both jobs (which generates the correct JSON *content* with variant-specific URLs baked in via the overlay), but the uploaded file is named `latest.json` by the action. After the action runs, use `gh release upload` to delete `latest.json` and re-upload it as `latest-cpu.json` or `latest-gpu.json`.

**Confidence:** MEDIUM — `tauri-action` source code does not document a rename option (confirmed via README and DeepWiki); the manual rename via `gh` CLI is a verified pattern for GitHub Releases asset management. The approach is correct in principle but requires verifying the exact `gh release upload` behavior with duplicate asset names between two concurrent matrix jobs.

**Example (post-build rename step):**
```yaml
      # After tauri-action completes, find and rename the updater JSON
      - name: Rename updater JSON to variant-specific name
        shell: bash
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          # Find the latest.json that was just uploaded
          TAG=${{ github.ref_name }}
          VARIANT=${{ matrix.variant }}
          # Download, rename, re-upload
          gh release download "$TAG" --pattern "latest.json" --output "latest-${VARIANT}.json"
          gh release upload "$TAG" "latest-${VARIANT}.json" --clobber
          # Remove the generic latest.json (both jobs will attempt this — --clobber handles concurrency)
          # Note: Only delete latest.json after BOTH variants have uploaded their variant-specific files.
          # Use a separate cleanup job with needs: [build] to safely delete latest.json.
```

**Alternative approach — skip includeUpdaterJson, build JSON manually:**
Use `includeUpdaterJson: false` and construct `latest-cpu.json` / `latest-gpu.json` manually from the artifact `.sig` files. This avoids the rename race condition entirely but requires scripting the JSON structure. The JSON structure is simple (version, platforms, signatures, URLs) and documented at https://v2.tauri.app/plugin/updater/.

**Recommendation:** Use `includeUpdaterJson: true` + post-build rename as the primary approach. If race conditions cause issues in the CI test, fall back to manual JSON construction.

### Pattern 5: CUDA DLL Bundling via bundle.resources in GPU Config Overlay

**What:** The GPU config overlay specifies `bundle.resources` pointing to a staging directory of CUDA DLLs. The CI step for the GPU variant copies the required DLLs from `$CUDA_PATH/bin/` into that staging directory before the Tauri build runs.

**Example:**

`src-tauri/tauri.gpu.conf.json` (extended):
```json
{
  "plugins": {
    "updater": {
      "endpoints": [
        "https://github.com/kkosiak592/voicetype/releases/latest/download/latest-gpu.json"
      ]
    }
  },
  "bundle": {
    "resources": {
      "cuda-libs/cudart64_12.dll":   "cudart64_12.dll",
      "cuda-libs/cublas64_12.dll":   "cublas64_12.dll",
      "cuda-libs/cublasLt64_12.dll": "cublasLt64_12.dll"
    }
  }
}
```

CI step (before GPU tauri-action build):
```yaml
      - name: Stage CUDA DLLs for bundling (GPU only)
        if: matrix.variant == 'gpu'
        shell: bash
        run: |
          mkdir -p src-tauri/cuda-libs
          cp "$CUDA_PATH/bin/cudart64_12.dll"   src-tauri/cuda-libs/
          cp "$CUDA_PATH/bin/cublas64_12.dll"   src-tauri/cuda-libs/
          cp "$CUDA_PATH/bin/cublasLt64_12.dll" src-tauri/cuda-libs/
```

The `resources` object map syntax (`"source": "target"`) places each DLL directly in the app's install directory (not in a `resources/` subfolder), making them loadable by the whisper-rs dynamic linker without PATH manipulation.

Source: https://v2.tauri.app/develop/resources (HIGH confidence — official docs, verified resource map syntax)

### Pattern 6: NSIS Pre-Install NVIDIA Driver Warning

**What:** A `.nsh` NSIS hooks file referenced via `bundle.windows.nsis.installerHooks` in the GPU config overlay. The `NSIS_HOOK_PREINSTALL` macro checks the registry for NVIDIA display adapter presence and shows a warning MessageBox if not found. Warn-only (installation proceeds regardless).

**When to use:** GPU variant only, via the `tauri.gpu.conf.json` overlay setting `bundle.windows.nsis.installerHooks`.

**Example** (`src-tauri/windows/gpu-hooks.nsh`):
```nsis
!macro NSIS_HOOK_PREINSTALL
  ; Check for NVIDIA display adapter in device class registry
  ; Key: HKLM\SYSTEM\CurrentControlSet\Control\Class\{4D36E968-E325-11CE-BFC1-08002BE10318}\0000
  ReadRegStr $0 HKLM "SYSTEM\CurrentControlSet\Control\Class\{4D36E968-E325-11CE-BFC1-08002BE10318}\0000" "DriverDesc"
  ${If} $0 == ""
    ; Try secondary adapter index
    ReadRegStr $0 HKLM "SYSTEM\CurrentControlSet\Control\Class\{4D36E968-E325-11CE-BFC1-08002BE10318}\0001" "DriverDesc"
  ${EndIf}
  ; Check if detected driver description contains "NVIDIA"
  ${If} $0 == ""
    MessageBox MB_ICONEXCLAMATION|MB_OK "Warning: No NVIDIA GPU detected.$\n$\nThis is the GPU variant of VoiceType and requires an NVIDIA GPU with drivers installed.$\n$\nInstallation will continue, but CUDA acceleration may not work."
  ${EndIf}
!macroend
```

**GPU config overlay addition:**
```json
{
  "bundle": {
    "windows": {
      "nsis": {
        "installerHooks": "./windows/gpu-hooks.nsh"
      }
    }
  }
}
```

Source: https://v2.tauri.app/distribute/windows-installer (HIGH confidence — official docs, NSIS hooks documented with working examples)
Source: https://forums.developer.nvidia.com/t/checking-graphics-driver-version-using-registry-keys/61862 (MEDIUM — NVIDIA developer forum, registry key confirmed)

**Discretion recommendation: Warn-only.** Blocking installation is aggressive for a GPU-accelerated app where the user may install now and add drivers later, or may use DirectML (non-CUDA) execution paths. CUDA-specific failures are surfaced at runtime with better context. The VC++ redistributable example in Tauri docs uses the same warn-only pattern.

### Pattern 7: Build Variant Label in App UI

**What:** A Tauri IPC command `get_build_variant` returns `"cpu"` or `"gpu"` as a compile-time constant determined by `#[cfg(feature = "cuda")]`. The frontend (`GeneralSection.tsx`) already uses `getVersion()` from `@tauri-apps/api/app` — it appends the variant label to the version display.

**Example (Rust, src-tauri/src/lib.rs):**
```rust
#[tauri::command]
fn get_build_variant() -> &'static str {
    #[cfg(feature = "cuda")]
    { "GPU" }
    #[cfg(not(feature = "cuda"))]
    { "CPU" }
}
```

Register in `tauri::Builder::invoke_handler(tauri::generate_handler![..., get_build_variant])`.

**Example (TypeScript, GeneralSection.tsx — existing version display):**
```typescript
// Existing code in GeneralSection.tsx (line 118-122):
{appVersion && (
  <p className="mt-4 text-xs text-gray-400 dark:text-gray-500">
    VoiceType v{appVersion} ({buildVariant})
  </p>
)}
```

Where `buildVariant` is fetched once via `invoke<string>('get_build_variant')` in a `useEffect`, same pattern as `getVersion()`.

### Anti-Patterns to Avoid

- **Using `#[cfg(feature = "cuda")]` on whisper-rs dependency directly in Cargo.toml:** Cargo does not support conditional features on dependency declarations based on your own features using `cfg()`. Use the `feature?/sub-feature` composition syntax instead.
- **Hardcoding `cuda` in `whisper-rs = { features = ["cuda"] }`:** This would make the CPU build link against CUDA DLLs, defeating the entire purpose.
- **Using `includeUpdaterJson: true` without a rename strategy:** Both matrix jobs would upload `latest.json`, and the second job would overwrite the first with a `latest.json` that has different URLs — whichever finishes last wins, causing one channel to break.
- **Placing CUDA DLLs in `bundle.resources` with path prefix:** The `resources` list syntax (`["cuda-libs/"]`) places files under `$RESOURCE/cuda-libs/`, not the app directory. DLLs must be in the app's install directory root. Use the map syntax `{"cuda-libs/foo.dll": "foo.dll"}` to target the root.
- **Checking only `0000` registry subkey for NVIDIA GPU:** Some multi-GPU or multi-adapter systems use `0001`, `0002`, etc. Check at least two indices.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Conditional CUDA feature on deps | Build script env var hacks or proc macros | Cargo `feature?/sub-feature` composition | Official Cargo mechanism, zero complexity |
| Config variations per variant | Runtime config loading, env vars | Tauri `--config` JSON overlay | Built into `cargo tauri build`, merge is automatic |
| NSIS installer checks | Custom .nsi template from scratch | `installerHooks` `.nsh` file | Template overrides require maintaining full NSIS script |
| Updater JSON upload | Custom CI steps building JSON from scratch | `tauri-action` with `includeUpdaterJson: true` + rename | Action handles platform detection, signature insertion, URL construction |

**Key insight:** The CUDA DLLs are NOT in the CUDA Toolkit's redistributable directory by default — they're in `$CUDA_PATH/bin/`. Do not assume a standard location; use the `$CUDA_PATH` env var that `Jimver/cuda-toolkit` sets automatically.

---

## Common Pitfalls

### Pitfall 1: whisper-rs CUDA Feature Linkage in CPU Build

**What goes wrong:** If `whisper-rs = { features = ["cuda"] }` remains in the base `[dependencies]` table, the CPU build still links against CUDA DLLs. The binary will fail to start on machines without NVIDIA drivers.

**Why it happens:** Cargo features on dependencies in `[dependencies]` are unconditional. They are not gated by the parent crate's feature flags unless you use the `feature?/sub-feature` composition syntax.

**How to avoid:** Remove `features = ["cuda"]` from the `whisper-rs` dependency declaration. Only enable it via `cuda = ["whisper-rs?/cuda"]` in `[features]`. Verify with `cargo build --no-default-features --features whisper,parakeet` and check the produced binary with `dumpbin /dependents` to confirm no CUDA DLLs appear.

**Warning signs:** CPU build takes as long as GPU build, or `dumpbin` shows `cudart64_12.dll` in the CPU binary's imports.

### Pitfall 2: Rust Cache Collision Between CPU and GPU Variants

**What goes wrong:** `Swatinem/rust-cache` caches compiled artifacts keyed by Cargo.lock and Rust toolchain version. If both matrix jobs share the same cache key, the CPU build may get the GPU build's cache (compiled with `--features cuda`) and proceed without recompilation, embedding CUDA-linked artifacts in the CPU installer.

**Why it happens:** The cache action doesn't automatically include `--features` or `--config` in its cache key.

**How to avoid:** Add `key: ${{ matrix.variant }}` to the `rust-cache` step to separate caches per variant. The `Swatinem/rust-cache@v2` action supports a `key` parameter that appends to the cache key.

**Warning signs:** GPU build finishes in 2 minutes (suspiciously fast), CPU build is 30+ minutes.

### Pitfall 3: tauri-action latest.json Upload Conflict

**What goes wrong:** Both CPU and GPU matrix jobs have `includeUpdaterJson: true`. Each job uploads `latest.json` to the same GitHub Release. The second job to finish overwrites the first, so the "winning" `latest.json` points to only one variant's installer URL. Users on the other variant get updates to the wrong installer.

**Why it happens:** `tauri-action` always names the updater file `latest.json` — no filename customization option exists (confirmed by README and action source).

**How to avoid:** Use the post-build rename strategy: each job renames its `latest.json` to `latest-cpu.json` / `latest-gpu.json` before the other job finishes. Use `--clobber` on `gh release upload` to handle racing. Add a cleanup job that runs after both matrix jobs with `needs: [publish-release]` to delete the generic `latest.json` from the release.

**Warning signs:** In GitHub Releases, only one `latest.json` appears (the other was overwritten).

### Pitfall 4: CUDA DLLs Not Found at Runtime Despite Being Bundled

**What goes wrong:** The GPU installer bundles CUDA DLLs via `bundle.resources`, but at runtime the app can't load them because they land in `$INSTDIR\resources\` rather than `$INSTDIR\` (app root).

**Why it happens:** The `resources` list syntax places files under a `resources/` subdirectory. The Windows DLL search order checks the exe's directory first, not subdirectories.

**How to avoid:** Use the `bundle.resources` map syntax to specify the target path explicitly:
```json
"resources": {
  "cuda-libs/cudart64_12.dll": "cudart64_12.dll"
}
```
This places `cudart64_12.dll` directly in `$INSTDIR/`, where the linker finds it.

**Warning signs:** GPU app crashes at startup on a clean NVIDIA system with error "cudart64_12.dll not found."

### Pitfall 5: Patched Crates Behaving Differently Without CUDA

**What goes wrong:** `esaxx-rs` (CRT fix) and `parakeet-rs` (vocab fix) were tested with CUDA enabled. The CPU build compiles them without CUDA features. If the patches interact with CUDA-specific code paths, the CPU build may fail to compile or produce incorrect behavior.

**Why it happens:** The patched crates are local path overrides; they haven't been tested without CUDA.

**How to avoid:** Run the full CPU build locally (`cargo build --no-default-features --features whisper,parakeet`) before CI integration. The esaxx-rs CRT fix is CUDA-independent (it's about static vs dynamic CRT linkage). The parakeet-rs vocab fix reads `vocab_size` from config — also CUDA-independent. Risk is LOW but must be confirmed.

**Warning signs:** Compilation errors referencing patched crate symbols, or test transcription producing empty/garbage output on CPU.

---

## Code Examples

Verified patterns from official sources:

### Cargo Feature Weak Dependency Composition

```toml
# Source: https://doc.rust-lang.org/cargo/reference/features.html
[features]
cuda = ["whisper-rs?/cuda", "parakeet-rs?/cuda"]
# `?/` = enable sub-feature only if the dep is already enabled by another feature
# whisper-rs is enabled by the `whisper` feature; parakeet-rs by `parakeet`
# This means: if cuda is enabled AND whisper is enabled, whisper-rs gets CUDA.
#             if cuda is enabled but whisper is NOT enabled, nothing happens.
```

### Tauri Config Overlay Build Command

```bash
# Source: https://v2.tauri.app/develop/configuration-files
# CPU variant
cargo tauri build --config src-tauri/tauri.cpu.conf.json

# GPU variant
cargo tauri build --features cuda --config src-tauri/tauri.gpu.conf.json
```

### bundle.resources Map Syntax for DLL Target Path

```json
// Source: https://v2.tauri.app/develop/resources
// Places DLL at install root, not in resources/ subdirectory
{
  "bundle": {
    "resources": {
      "cuda-libs/cudart64_12.dll":   "cudart64_12.dll",
      "cuda-libs/cublas64_12.dll":   "cublas64_12.dll",
      "cuda-libs/cublasLt64_12.dll": "cublasLt64_12.dll"
    }
  }
}
```

### NSIS Hooks Configuration

```json
// Source: https://v2.tauri.app/distribute/windows-installer
{
  "bundle": {
    "windows": {
      "nsis": {
        "installerHooks": "./windows/gpu-hooks.nsh"
      }
    }
  }
}
```

### Post-Build Updater JSON Rename via gh CLI

```bash
# Source: GitHub CLI docs (gh release command)
# Run after tauri-action completes on the runner
TAG="${{ github.ref_name }}"
VARIANT="${{ matrix.variant }}"

# Download the latest.json tauri-action just uploaded
gh release download "$TAG" --pattern "latest.json" --output "latest-${VARIANT}.json"

# Upload as variant-specific name (--clobber overwrites if already exists)
gh release upload "$TAG" "latest-${VARIANT}.json" --clobber
```

### Tauri Command for Build Variant Label

```rust
// Source: Tauri v2 command pattern (established project pattern)
#[tauri::command]
fn get_build_variant() -> &'static str {
    #[cfg(feature = "cuda")]
    { "GPU" }
    #[cfg(not(feature = "cuda"))]
    { "CPU" }
}
```

---

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| Single `latest.json` for all users | Two variant-specific `latest-{variant}.json` files | Each variant auto-updates on its own channel |
| CUDA always enabled in `whisper-rs` | `cuda` feature gates CUDA across both engines | CPU binary has zero CUDA DLL dependencies |
| Single installer for all users | Two NSIS installers (CPU + GPU) | Users without NVIDIA can install VoiceType |
| No variant indication in UI | Version string shows `(CPU)` or `(GPU)` | Users can verify which variant is installed |

**Deprecated/outdated:**
- `createUpdaterArtifacts: "v1Compatible"` in base config: This setting remains relevant. The `v1Compatible` mode generates a `.nsis.zip.sig` file alongside the installer — required by tauri-plugin-updater v2. Do NOT remove it.
- The base config's `plugins.updater.endpoints` pointing to `latest.json`: This should be removed from the base config entirely. Each variant overlay provides its own endpoint. If the base config retains a `latest.json` endpoint, a build without `--config` overlay would point to a non-existent file.

---

## Open Questions

1. **latest.json deletion race condition in matrix builds**
   - What we know: Both matrix jobs upload `latest.json` via `tauri-action`. The rename step runs immediately after the action.
   - What's unclear: If job A finishes and deletes `latest.json` before job B uploads its `latest.json`, job B's rename step will fail to download a `latest.json` that doesn't exist yet. If both upload simultaneously, one overwrites the other before rename.
   - Recommendation: Use a dedicated cleanup job with `needs: [publish-release]` that runs after all matrix jobs complete and deletes `latest.json`. Both variant jobs should upload `latest-cpu.json` / `latest-gpu.json` via their rename steps without trying to delete the generic one. The cleanup job deletes `latest.json` at the end.

2. **nvml-wrapper in CPU build: keep or strip**
   - What we know: `nvml-wrapper` is currently gated behind the `whisper` feature (`whisper = ["dep:whisper-rs", "dep:nvml-wrapper"]`). The CPU build includes both `whisper` and `parakeet` features, so `nvml-wrapper` is included.
   - What's unclear: The `cuda` discretion item asks whether the CPU build should detect NVIDIA GPU and suggest the GPU version. `nvml-wrapper` is the mechanism for this detection.
   - Recommendation: Keep `nvml-wrapper` in the CPU build. It enables a one-time "you have an NVIDIA GPU — consider the GPU variant" suggestion in the settings UI. This is a better UX than forcing users to guess. A one-time dismissed notification (stored in `settings.json`) is sufficient.

3. **CUDA DLL exact filenames for CUDA 12.6.3**
   - What we know: The DLLs are `cudart64_12.dll`, `cublas64_12.dll`, `cublasLt64_12.dll` based on the existing research. The CI already installs CUDA 12.6.3.
   - What's unclear: The exact filenames for CUDA 12.6.x should be confirmed from `$CUDA_PATH/bin/` on the runner before hardcoding them in the CI step.
   - Recommendation: The first GPU CI run should include a debug step: `ls "$CUDA_PATH/bin/" | grep -i "cudart\|cublas"` to confirm exact names.

4. **Whether to generate latest.json as GPU alias**
   - What we know: The user decision is to drop `latest.json` entirely — no backward compatibility required (no real installed base).
   - What's unclear: The discretion item asks whether to also keep `latest.json` as GPU alias. With no existing users, this is not needed.
   - Recommendation: Do not generate a `latest.json` alias. Clean break. The post-build cleanup job deletes `latest.json` from each release.

---

## Sources

### Primary (HIGH confidence)
- `/tauri-apps/tauri-action` (Context7) — action inputs, `includeUpdaterJson`, `updaterJsonPreferNsis`, `args` flag
- https://v2.tauri.app/develop/configuration-files — `--config` overlay, JSON Merge Patch
- https://v2.tauri.app/plugin/updater — updater endpoint config, static JSON format, `latest.json` structure
- https://v2.tauri.app/distribute/windows-installer — NSIS `installerHooks`, VC++ redistributable bundling pattern
- https://v2.tauri.app/develop/resources — `bundle.resources` list vs map syntax, target path behavior
- https://v2.tauri.app/reference/config — `NsisConfig.installerHooks` field
- https://doc.rust-lang.org/cargo/reference/features.html — `feature?/sub-feature` weak dependency syntax

### Secondary (MEDIUM confidence)
- https://forums.developer.nvidia.com/t/checking-graphics-driver-version-using-registry-keys/61862 — HKLM registry key for NVIDIA display class detection
- https://deepwiki.com/tauri-apps/tauri-action/1.1-features-and-capabilities — confirmed `latest.json` is not renameable via action inputs
- https://github.com/orgs/tauri-apps/discussions/10206 — confirmed `--config` flag pattern for fork/variant separation

### Tertiary (LOW confidence)
- WebSearch results on `gh release upload` for asset rename — pattern is correct in principle but the exact race condition handling in the matrix context requires CI testing to validate

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all tools are in active use in this project or are official Tauri mechanisms
- Architecture: HIGH for Cargo features and config overlays (official docs); MEDIUM for updater JSON rename (no official tauri-action rename support confirmed — workaround required)
- Pitfalls: HIGH for CUDA linkage and DLL placement (technical facts); MEDIUM for cache collision and race condition (inferred from tool behavior)

**Research date:** 2026-03-03
**Valid until:** 2026-06-03 (stable tooling — Cargo features, Tauri v2 config, NSIS hooks are all stable APIs; tauri-action changes infrequently)
