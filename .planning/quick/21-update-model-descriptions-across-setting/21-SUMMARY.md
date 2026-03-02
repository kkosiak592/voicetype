---
phase: quick-21
plan: 01
subsystem: ui/model-descriptions
tags: [model-descriptions, settings, first-run, backend, frontend]
dependency_graph:
  requires: []
  provides: [accurate-model-descriptions]
  affects: [settings-model-selector, first-run-model-cards]
tech_stack:
  added: []
  patterns: []
key_files:
  created: []
  modified:
    - src-tauri/src/lib.rs
    - src/components/FirstRun.tsx
decisions:
  - "Removed gpu_mode conditional from Small (English) description — both branches now share identical text, so a single string is cleaner"
  - "Parakeet description updated to reflect CUDA + DirectML + CPU support, not NVIDIA-only"
metrics:
  duration: "5 minutes"
  completed: "2026-03-02"
---

# Phase quick-21 Plan 01: Update Model Descriptions Across Settings and First Run Summary

**One-liner:** Corrected model quality labels and GPU support text in both the Rust backend (Settings ModelSelector) and React frontend (FirstRun cards) for accuracy and consistency.

## What Was Done

Updated model description text in two places:

**Backend (`src-tauri/src/lib.rs` — `list_models` function):**
- Large v3 Turbo: `"Best accuracy"` → `"Most accurate"`
- Small (English): removed `gpu_mode` conditional; both branches had converged to the same semantic meaning, replaced with single string `"Lightweight — 190 MB — GPU accelerated when available"`
- Parakeet TDT fp32: `"Full precision — 2.56 GB — requires NVIDIA GPU (ONNX)"` → `"Fast and accurate — 2.56 GB — GPU accelerated (CUDA or DirectML)"`

**Frontend (`src/components/FirstRun.tsx` — `MODELS` array):**
- Large v3 Turbo quality: `'Best accuracy'` → `'Most accurate'`
- Parakeet TDT fp32 quality: `'Full precision'` → `'Fast and accurate'`
- Small (English): already correct (`'Fast and lightweight'`, `'GPU accelerated when available'`), no change

## Verification

- `cargo check` passed with no errors
- `npx tsc --noEmit` passed with no errors
- Grep confirms zero remaining instances of `"Best accuracy"`, `"Full precision"`, or `"requires NVIDIA GPU (ONNX)"` in source files

## Commits

| Task | Description | Commit |
|------|-------------|--------|
| 1 | Backend model descriptions in list_models | a6e2dc3 |
| 2 | FirstRun MODELS quality labels | c9aa853 |

## Deviations from Plan

None — plan executed exactly as written.

## Self-Check: PASSED

- `src-tauri/src/lib.rs` modified: confirmed
- `src/components/FirstRun.tsx` modified: confirmed
- Commit a6e2dc3 exists: confirmed
- Commit c9aa853 exists: confirmed
