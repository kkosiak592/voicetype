---
phase: quick-15
plan: "01"
subsystem: backend
tags: [whisper, performance, model-loading, early-return]
dependency_graph:
  requires: []
  provides: [same-model-short-circuit-in-set_model]
  affects: [set_model, read_settings]
tech_stack:
  added: []
  patterns: [early-return guard, settings.json as source of truth for current model]
key_files:
  modified:
    - src-tauri/src/lib.rs
decisions:
  - "settings.json whisper_model_id used as source of truth for current model (not WhisperStateMutex Option check) — WhisperStateMutex is always Some after startup so Option check would not distinguish which model is loaded"
  - "Brace-scoped block for the json borrow prevents borrow interference with the rest of set_model()"
metrics:
  duration: "~5 minutes"
  completed: "2026-03-01"
  tasks_completed: 1
  files_modified: 1
---

# Phase quick-15 Plan 01: Skip Redundant Whisper Model Reload Summary

Early-return guard in set_model() that skips the multi-second WhisperContext disk reload when the requested model_id already matches the persisted whisper_model_id in settings.json.

## What Was Built

Added a brace-scoped early-return block in `set_model()` (src-tauri/src/lib.rs) immediately after the `model_path.exists()` check and before the `spawn_blocking` call. The guard reads `whisper_model_id` from settings.json via `read_settings()` and returns `Ok(())` with an info log if the value matches the requested `model_id`. Different model selections fall through to the existing full reload path unchanged.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add same-model early-return guard to set_model() | c5e7126 | src-tauri/src/lib.rs |

## Deviations from Plan

None — plan executed exactly as written.

## Key Decisions

- **settings.json as source of truth:** `WhisperStateMutex` is always `Some` after startup (unlike `ParakeetStateMutex` which can be `None`), so checking the mutex Option would not reveal which model is currently loaded. `whisper_model_id` in settings.json is written on every successful `set_model()` call and on startup, making it the correct source of truth.
- **Brace-scoped block:** Wrapping the `read_settings` call in `{}` ensures the `json` value is dropped before the `move` closure in `spawn_blocking`, preventing borrow checker conflicts.

## Self-Check: PASSED

- src-tauri/src/lib.rs modified with early-return guard at line 924-933
- Commit c5e7126 exists and verified
- `cargo check --features whisper` finished with no errors
