---
phase: quick-17
plan: "01"
subsystem: model-selection
tags: [ui, backend, first-run, model-recommendation]
dependency_graph:
  requires: []
  provides: [recommended-model-fp32-gpu, fastest-badge-removed]
  affects: [FirstRun.tsx, ModelSelector.tsx, lib.rs]
tech_stack:
  added: []
  patterns: [recommended-flag-driven-by-gpu-mode]
key_files:
  created: []
  modified:
    - src-tauri/src/lib.rs
    - src/components/FirstRun.tsx
decisions:
  - "parakeet-tdt-v2-fp32 is now the recommended model for GPU users — replaces large-v3-turbo"
  - "Fastest badge removed entirely — superlative no longer appropriate without dedicated badge"
  - "int8 quality label changed to 'Fast (GPU)' from 'Fastest (GPU)' to match badge removal"
metrics:
  duration_seconds: 109
  completed_date: "2026-03-02"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 2
---

# Quick Task 17: Remove Fastest Badge and Move Recommended to fp32 Summary

**One-liner:** Removed "Fastest" badge from FirstRun entirely and shifted the "Recommended" badge from Large v3 Turbo to Parakeet TDT fp32 for GPU users in both backend list_models and check_first_run.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Update backend recommendation to Parakeet TDT fp32 | 0b90832 | src-tauri/src/lib.rs |
| 2 | Remove Fastest badge and update FirstRun card text | ccd1e5d | src/components/FirstRun.tsx |

## What Was Done

### Task 1 — Backend (lib.rs)

In `list_models()`:
- `large-v3-turbo`: `recommended: gpu_mode` -> `recommended: false`
- `parakeet-tdt-v2` (int8): `recommended: false` unchanged
- `parakeet-tdt-v2-fp32`: `recommended: false` -> `recommended: gpu_mode`
- Updated stale comment ("Whisper is recommended per locked decision" — no longer accurate)

In `check_first_run()`:
- `recommended_model` for GPU: `"large-v3-turbo"` -> `"parakeet-tdt-v2-fp32"`
- CPU path unchanged: `"small-en"`

### Task 2 — Frontend (FirstRun.tsx)

- Deleted `const isFastest = model.id === 'parakeet-tdt-v2' && gpuDetected;`
- Deleted the Fastest badge JSX block (`{isFastest && (<span ...>Fastest</span>)}`)
- Changed `quality: 'Fastest (GPU)'` to `quality: 'Fast (GPU)'` for int8 entry in MODELS array

## Verification

- `cargo check` passes (Rust compiles cleanly)
- `npx tsc --noEmit` passes (TypeScript compiles with no errors)
- No remaining references to `isFastest` or "Fastest" in `src/`

## Deviations from Plan

None — plan executed exactly as written.

## Self-Check

- [x] src-tauri/src/lib.rs modified — `recommended: gpu_mode` on fp32, `recommended: false` on large-v3-turbo, `recommended_model` set to fp32 for GPU
- [x] src/components/FirstRun.tsx modified — isFastest removed, Fastest badge JSX removed, int8 quality label updated
- [x] Commit 0b90832 exists (backend)
- [x] Commit ccd1e5d exists (frontend)
- [x] No isFastest or Fastest references remain in src/

## Self-Check: PASSED
