---
phase: quick-19
plan: 01
subsystem: model-management
tags: [parakeet, int8-removal, download, frontend, backend]
dependency_graph:
  requires: []
  provides: [fp32-only-parakeet-codebase]
  affects: [download.rs, lib.rs, transcribe_parakeet.rs, FirstRun.tsx, ModelSelector.tsx, ModelSection.tsx, App.tsx]
tech_stack:
  added: []
  patterns: [fp32-only-model-path]
key_files:
  modified:
    - src-tauri/src/download.rs
    - src-tauri/src/lib.rs
    - src-tauri/src/transcribe_parakeet.rs
    - src/components/FirstRun.tsx
    - src/components/ModelSelector.tsx
    - src/components/sections/ModelSection.tsx
    - src/App.tsx
decisions:
  - "[quick-19]: parakeet_model_dir and parakeet_model_exists deleted; resolve_parakeet_dir always returns fp32 dir"
  - "[quick-19]: All Parakeet defaults (read_saved_parakeet_model, read_saved_parakeet_model_startup) now return parakeet-tdt-v2-fp32"
  - "[quick-19]: ModelSelector simplified to single isParakeet variable (fp32 only), removed isParakeetInt8 variant logic"
  - "[quick-19]: FirstRun now shows 3 GPU model cards (lg:grid-cols-3) instead of 4"
metrics:
  duration: "~10 minutes"
  completed: "2026-03-02T16:17:32Z"
  tasks_completed: 2
  files_modified: 7
---

# Phase quick-19 Plan 01: Remove Parakeet int8 Model Summary

Removed the Parakeet TDT int8 model variant (`parakeet-tdt-v2`) from the entire codebase — backend download infrastructure, model listing, frontend model cards, and all fallback defaults — leaving only the fp32 variant (`parakeet-tdt-v2-fp32`).

## Tasks Completed

| Task | Name | Commit | Key Changes |
|------|------|--------|-------------|
| 1 | Remove int8 Parakeet from Rust backend | e63ad1f | Deleted PARAKEET_FILES, download_parakeet_model command, parakeet_model_dir/exists functions; removed int8 ModelInfo from list_models; changed all defaults to fp32 |
| 2 | Remove int8 Parakeet from frontend | 0ef9a61 | Removed int8 model card from FirstRun; simplified ModelSelector to fp32-only; deleted handleParakeetDownload from ModelSection; updated App.tsx engine reconciliation |

## What Was Built

- **download.rs**: Deleted `PARAKEET_FILES` const, `parakeet_model_dir()`, `parakeet_model_exists()`, and the `download_parakeet_model` Tauri command. Renamed `parakeet_download_url` doc comment to remove int8 reference.
- **lib.rs**: All four `"parakeet-tdt-v2"` default fallback strings changed to `"parakeet-tdt-v2-fp32"`. `resolve_parakeet_dir` simplified to always return `parakeet_fp32_model_dir()`. Int8 `ModelInfo` block removed from `list_models`. `parakeet_exists` check removed from `check_first_run` `needs_setup` condition. `download_parakeet_model` unregistered from invoke_handler.
- **transcribe_parakeet.rs**: Doc comment updated from "int8 variant" to "fp32 variant".
- **FirstRun.tsx**: Removed int8 model entry from MODELS array. Changed grid to `lg:grid-cols-3`. Removed `download_parakeet_model` branch from `handleDownload`. Simplified engine activation condition to fp32 only.
- **ModelSelector.tsx**: Removed `onParakeetDownload`, `parakeetDownloading`, `parakeetPercent`, `parakeetError` props from interface and signature. Replaced `isParakeetInt8`/`isParakeetFp32` pair with single `isParakeet`. Simplified all download state resolution to direct fp32 references.
- **ModelSection.tsx**: Deleted `handleParakeetDownload` function. Removed `parakeetDownloading`, `parakeetPercent`, `parakeetError` state declarations. Removed int8 props from `<ModelSelector>` JSX.
- **App.tsx**: Engine reconciliation updated to use `'parakeet-tdt-v2-fp32'` in all three checks.

## Verification Results

1. `cargo check` — passes clean (no errors, only pre-existing esaxx-rs warning)
2. `npx tsc --noEmit` — passes clean (no errors)
3. Grep for `'parakeet-tdt-v2'` (without fp32) in `src/` and `src-tauri/src/` — zero matches
4. Grep for `download_parakeet_model` in lib.rs invoke_handler — zero matches
5. Grep for `parakeet_model_dir()` (without fp32) — zero matches

## Deviations from Plan

None — plan executed exactly as written.

## Self-Check: PASSED

- e63ad1f exists: confirmed
- 0ef9a61 exists: confirmed
- All 7 modified files verified clean of bare `parakeet-tdt-v2` string literals
