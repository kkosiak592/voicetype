---
phase: 20-bundle-cuda-dlls-single-installer-with-runtime-gpu-fallback
plan: 01
subsystem: infra
tags: [cuda, nsis, tauri, ci, github-actions, dll, windows-installer]

requires: []
provides:
  - CI step that stages cudart64_12.dll, cublas64_12.dll, cublasLt64_12.dll from CUDA toolkit into src-tauri/cuda-libs/ before Tauri build
  - bundle.resources map in tauri.conf.json that places CUDA DLLs at NSIS install root alongside VoiceType.exe
  - gitignore rule preventing accidental commit of staged DLLs
affects: [phase-21-integration-and-distribution]

tech-stack:
  added: []
  patterns:
    - "CI DLL staging: cp from $CUDA_PATH/bin/ to src-tauri/cuda-libs/ before tauri-action runs"
    - "Tauri bundle.resources map syntax (object not array) for flat install-root DLL placement"

key-files:
  created: []
  modified:
    - .github/workflows/release.yml
    - src-tauri/tauri.conf.json
    - .gitignore

key-decisions:
  - "bundle.resources uses MAP syntax (object) not array — array syntax preserves subdirectory structure, putting DLLs off the DLL search path"
  - "DLL staging runs after 'Set CUDA architecture targets' and before 'Install frontend dependencies' so $CUDA_PATH is available and files exist before tauri-action"
  - "Flat filename destinations (e.g. 'cudart64_12.dll' not 'cuda-libs/cudart64_12.dll') ensure DLLs land at $INSTDIR alongside VoiceType.exe"

patterns-established:
  - "CUDA DLL staging pattern: mkdir src-tauri/cuda-libs then cp from $CUDA_PATH/bin/ before Tauri build"

requirements-completed: []

duration: 5min
completed: 2026-03-05
---

# Phase 20 Plan 01: Bundle CUDA DLLs in NSIS Installer Summary

**CI stages three CUDA redistribution DLLs from toolkit into Tauri bundle.resources map so a single NSIS installer delivers GPU acceleration without requiring users to install CUDA Toolkit separately**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-03-05T00:00:00Z
- **Completed:** 2026-03-05
- **Tasks:** 1/2 (Task 2 is human-verify checkpoint — awaiting review)
- **Files modified:** 3

## Accomplishments
- CI workflow gains "Stage CUDA redistributable DLLs" step that copies cudart64_12.dll, cublas64_12.dll, cublasLt64_12.dll from $CUDA_PATH/bin/ to src-tauri/cuda-libs/ before the Tauri build step
- tauri.conf.json bundle section gains a `resources` map using object syntax with flat filename destinations so DLLs land at $INSTDIR (install root) alongside VoiceType.exe — on the DLL search path
- .gitignore prevents accidental commit of the 500MB+ staging directory

## Task Commits

Each task was committed atomically:

1. **Task 1: Add CI DLL staging step and Tauri bundle.resources map** - `754301c` (feat)

## Files Created/Modified
- `.github/workflows/release.yml` - New "Stage CUDA redistributable DLLs" step inserted between "Set CUDA architecture targets" and "Install frontend dependencies"
- `src-tauri/tauri.conf.json` - Added `bundle.resources` map with three CUDA DLL entries using flat filename destinations
- `.gitignore` - Added `src-tauri/cuda-libs/` entry with explanatory comment

## Decisions Made
- bundle.resources uses MAP syntax (object not array) — Tauri's array syntax preserves subdirectory structure, which would place DLLs in a cuda-libs/ subdirectory off the DLL search path and prevent runtime loading
- DLL staging positioned after CUDA toolkit installation (so $CUDA_PATH is set) and before tauri-action (so bundler can find the files)
- Flat destinations: `"cuda-libs/cudart64_12.dll": "cudart64_12.dll"` — source path is relative to src-tauri/, destination is install root

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Full validation requires a CI run (tag push) — DLL placement at $INSTDIR can only be confirmed by inspecting the built installer
- Task 2 (human-verify checkpoint) must be approved before plan is considered complete
- Once verified, phase 21 (Integration and Distribution) can proceed

---
*Phase: 20-bundle-cuda-dlls-single-installer-with-runtime-gpu-fallback*
*Completed: 2026-03-05*
