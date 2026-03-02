---
phase: 13-ci-cd-pipeline
plan: "01"
subsystem: infra
tags: [github-actions, ci-cd, tauri, release, signing, cuda, llvm]

# Dependency graph
requires:
  - TAURI_SIGNING_PRIVATE_KEY and TAURI_SIGNING_PRIVATE_KEY_PASSWORD in GitHub repo secrets (Phase 11)
  - tauri.conf.json with NSIS target, createUpdaterArtifacts v1Compatible, Ed25519 pubkey (Phase 11)
provides:
  - GitHub Actions workflow at .github/workflows/release.yml
  - Automated release pipeline triggered by v* tag push
  - NSIS installer + .sig + latest.json published to GitHub Release on tag push
affects: [14-release-workflow]

# Tech tracking
tech-stack:
  added:
    - tauri-apps/tauri-action@v0 (GitHub Actions step for Tauri build + release)
    - Jimver/cuda-toolkit@v0.2.21 (CUDA 12.6.3 minimal subset install in CI)
    - KyleMayes/install-llvm-action@v2 (LLVM 18 for whisper-rs bindgen)
    - Swatinem/rust-cache@v2 (Rust build artifact caching)
    - dtolnay/rust-toolchain@stable (Rust toolchain install)
  patterns:
    - CMAKE_CUDA_ARCHITECTURES=61;75;86;89 targets Pascal/Turing/Ampere/Ada GPUs in one binary
    - npm ci for reproducible frontend dependency installs in CI
    - CUDA minimal sub-packages to avoid 4 GB full toolkit download

key-files:
  created:
    - .github/workflows/release.yml
  modified: []

key-decisions:
  - "Used Jimver/cuda-toolkit@v0.2.21 with minimal sub-packages (nvcc, cudart, cublas, cublas_dev, thrust, visual_studio_integration) — avoids 4 GB full toolkit download while providing all headers/libs needed for whisper-rs and parakeet-rs compilation"
  - "CMAKE_CUDA_ARCHITECTURES=61;75;86;89 chosen to support Pascal through Ada Lovelace GPUs in a single binary"
  - "tagName: github.ref_name used instead of v__VERSION__ pattern — tag is the source of truth, not Cargo.toml version"
  - "KyleMayes/install-llvm-action@v2 with LLVM 18 chosen for whisper-rs bindgen LIBCLANG_PATH requirement"

# Metrics
duration: ~2min
completed: 2026-03-02
---

# Phase 13 Plan 01: CI/CD Pipeline — Release Workflow Summary

**GitHub Actions release workflow created: v* tag push triggers windows-latest build with CUDA 12.6.3 + LLVM 18, produces signed NSIS installer with latest.json, and publishes immediately to GitHub Release**

## Performance

- **Duration:** ~2 min
- **Started:** 2026-03-02T20:08:33Z
- **Completed:** 2026-03-02T20:10:34Z
- **Tasks:** 2
- **Files modified:** 1 (.github/workflows/release.yml created)

## Accomplishments

- Created `.github/workflows/release.yml` with all five CICD requirements addressed
- Workflow triggers on any `v*` tag push (CICD-01)
- windows-latest runner installs CUDA 12.6.3 (minimal subset via Jimver action) and LLVM 18 (via KyleMayes action) for whisper-rs/parakeet-rs compilation (CICD-02)
- TAURI_SIGNING_PRIVATE_KEY and TAURI_SIGNING_PRIVATE_KEY_PASSWORD from GitHub secrets enable Ed25519 signing during build, producing .sig files alongside installer (CICD-03)
- `includeUpdaterJson: true` generates latest.json with download URLs and Ed25519 signatures for the auto-updater endpoint (CICD-04)
- `releaseDraft: false` publishes the GitHub Release immediately with installer, .sig, and latest.json attached (CICD-05)
- `updaterJsonPreferNsis: true` ensures latest.json points to the NSIS .exe installer (consistent with tauri.conf.json `targets: ["nsis"]`)
- Rust build caching via Swatinem/rust-cache@v2 with `cache-on-failure: true` to benefit incremental builds
- Task 2 validated workflow structure and cross-referenced with tauri.conf.json — no issues found

## Task Commits

Each task committed atomically:

1. **Task 1: Create GitHub Actions release workflow** — `a9c9bff` (feat)
2. **Task 2: Validate workflow YAML syntax and structure** — no commit (validation only, no file changes)

## Files Created/Modified

- `.github/workflows/release.yml` — Complete release pipeline: v* tag trigger, windows-latest runner, CUDA + LLVM setup, Rust + Node toolchain, tauri-action with signing secrets and updater JSON configuration

## Decisions Made

- Used `Jimver/cuda-toolkit@v0.2.21` with minimal sub-packages to avoid downloading the full 4 GB CUDA toolkit
- `CMAKE_CUDA_ARCHITECTURES=61;75;86;89` supports Pascal (GTX 10xx), Turing (RTX 20xx), Ampere (RTX 30xx), and Ada Lovelace (RTX 40xx) GPUs
- `tagName: ${{ github.ref_name }}` uses the pushed tag directly rather than the `v__VERSION__` pattern — tag is the release source of truth
- LLVM 18 chosen as the most recent stable major version; `KyleMayes/install-llvm-action@v2` sets LIBCLANG_PATH automatically for bindgen

## Deviations from Plan

None — plan executed exactly as written.

## Cross-reference Validation (Task 2 Findings)

- `tauri.conf.json bundle.targets: ["nsis"]` → `updaterJsonPreferNsis: true` in workflow (consistent)
- `tauri.conf.json createUpdaterArtifacts: "v1Compatible"` → TAURI_SIGNING_PRIVATE_KEY triggers Tauri signing → .sig files produced (consistent)
- `tauri.conf.json plugins.updater.endpoints` points to GitHub Releases latest.json → `includeUpdaterJson: true` uploads latest.json to same release (consistent)

## Next Phase Readiness

- Phase 14 (Release Workflow): Push a `v*` tag to trigger the workflow and verify end-to-end: build completes, GitHub Release created with installer + latest.json + .sig, auto-updater endpoint returns valid JSON

---
*Phase: 13-ci-cd-pipeline*
*Completed: 2026-03-02*
