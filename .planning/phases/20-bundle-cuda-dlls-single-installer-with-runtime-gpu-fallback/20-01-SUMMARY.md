---
phase: 20-bundle-cuda-dlls-single-installer-with-runtime-gpu-fallback
plan: 01
subsystem: infra
tags: [cuda, nsis, tauri, ci, github-actions, dll, windows-installer, tauri-config]

requires: []
provides:
  - CI step that stages cudart64_12.dll, cublas64_12.dll, cublasLt64_12.dll from CUDA toolkit into src-tauri/cuda-libs/ before Tauri build
  - TAURI_CONFIG env var in release workflow that injects bundle.resources map at CI time only (flat filename destinations at install root)
  - gitignore rule preventing accidental commit of staged DLLs
affects: [phase-21-integration-and-distribution]

tech-stack:
  added: []
  patterns:
    - "CI DLL staging: cp from $CUDA_PATH/bin/ to src-tauri/cuda-libs/ before tauri-action runs"
    - "TAURI_CONFIG env var for CI-only config injection — keeps tauri.conf.json clean for local dev"
    - "bundle.resources map syntax (object not array) with flat filename destinations for install-root DLL placement"

key-files:
  created: []
  modified:
    - .github/workflows/release.yml
    - .gitignore

key-decisions:
  - "TAURI_CONFIG env var used instead of tauri.conf.json bundle.resources — static resources config breaks local dev because cuda-libs/ only exists in CI"
  - "bundle.resources uses MAP syntax (object) not array — array syntax preserves subdirectory structure putting DLLs off the DLL search path"
  - "DLL staging runs after 'Set CUDA architecture targets' and before 'Install frontend dependencies' so $CUDA_PATH is available and files exist before tauri-action"
  - "Flat filename destinations (cudart64_12.dll not cuda-libs/cudart64_12.dll) ensure DLLs land at $INSTDIR alongside VoiceType.exe"

patterns-established:
  - "CUDA DLL staging pattern: mkdir src-tauri/cuda-libs then cp from $CUDA_PATH/bin/ before Tauri build"
  - "CI-only Tauri config override: TAURI_CONFIG env var on tauri-action step for resources that only exist during CI"

requirements-completed: []

duration: 10min
completed: 2026-03-05
---

# Phase 20 Plan 01: Bundle CUDA DLLs in NSIS Installer Summary

**CI stages three CUDA redistributable DLLs from toolkit then injects them into the NSIS installer via TAURI_CONFIG env var, avoiding local dev breakage from a static resources config pointing at CI-only files**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-03-05
- **Completed:** 2026-03-05
- **Tasks:** 2/2
- **Files modified:** 2 (tauri.conf.json reverted to clean state; release.yml and .gitignore modified)

## Accomplishments
- CI workflow stages cudart64_12.dll, cublas64_12.dll, cublasLt64_12.dll from $CUDA_PATH/bin/ to src-tauri/cuda-libs/ before the Tauri build step
- TAURI_CONFIG env var on the tauri-action step injects the bundle.resources map at CI time only — tauri.conf.json stays clean so local dev works without the staging directory present
- .gitignore prevents accidental commit of the 500MB+ staging directory

## Task Commits

1. **Task 1: Add CI DLL staging step and Tauri bundle.resources map** - `754301c` (feat)
2. **Fix: Move CUDA resources to CI-only TAURI_CONFIG env var** - `eeec523` (fix) — applied during human-verify

**Plan metadata:** `e2b43c4` (docs)

## Files Created/Modified
- `.github/workflows/release.yml` - "Stage CUDA redistributable DLLs" step + TAURI_CONFIG env var on tauri-action with bundle.resources map as inline JSON
- `.gitignore` - Added `src-tauri/cuda-libs/` entry with explanatory comment
- `src-tauri/tauri.conf.json` - bundle.resources added then removed (net: unchanged from pre-plan state)

## Decisions Made
- TAURI_CONFIG env var instead of static tauri.conf.json entry — `bundle.resources` in the static config causes `tauri dev` to fail locally because cuda-libs/ only exists during CI. TAURI_CONFIG is merged at build time by tauri-action and only injected in the release workflow.
- bundle.resources uses MAP (object) syntax not array — array syntax preserves subdirectory structure, which would place DLLs at cuda-libs/ inside $INSTDIR rather than at the root, taking them off the DLL search path.
- Flat destinations: `"cuda-libs/cudart64_12.dll": "cudart64_12.dll"` — source path relative to src-tauri/, destination is install root.

## Deviations from Plan

### Post-checkpoint Fix (Human-initiated)

**[Fix applied during human-verify] Move bundle.resources from tauri.conf.json to TAURI_CONFIG env var**
- **Found during:** Task 2 (human-verify checkpoint)
- **Issue:** Static `bundle.resources` in tauri.conf.json breaks `tauri dev` locally because cuda-libs/ only exists during CI — Tauri errors if a resources path doesn't exist at build time
- **Fix:** Removed `bundle.resources` from tauri.conf.json; added `TAURI_CONFIG` env var on the tauri-action step in release.yml with the same resources map as inline JSON
- **Files modified:** .github/workflows/release.yml, src-tauri/tauri.conf.json
- **Commit:** `eeec523` (fix(20-01): move CUDA resources to CI-only TAURI_CONFIG env var)

---

**Total deviations:** 1 (post-checkpoint human fix)
**Impact on plan:** Correct approach — same DLL bundling outcome during CI builds, no local dev regression.

## Issues Encountered

None beyond the resources config local-dev breakage, which was caught and fixed at the verification checkpoint.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Full validation requires a CI run (tag push) — DLL placement at $INSTDIR can only be confirmed by inspecting the built installer
- Phase 21 (Integration and Distribution) can proceed

---
*Phase: 20-bundle-cuda-dlls-single-installer-with-runtime-gpu-fallback*
*Completed: 2026-03-05*
