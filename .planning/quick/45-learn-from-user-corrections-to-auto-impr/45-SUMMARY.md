---
phase: quick-45
plan: 01
subsystem: corrections-learning
tags: [corrections, history, learning, adaptive, backend, frontend]
dependency_graph:
  requires: [history, corrections, profiles, pipeline]
  provides: [correction_log, submit_correction IPC, undo_promotion IPC, inline correction editor]
  affects: [corrections dictionary, history entries]
tech_stack:
  added: []
  patterns: [managed-state, IPC command, word-level diff, auto-promotion threshold]
key_files:
  created:
    - src-tauri/src/correction_log.rs
  modified:
    - src-tauri/src/history.rs
    - src-tauri/src/pipeline.rs
    - src-tauri/src/lib.rs
    - src/components/sections/HistorySection.tsx
decisions:
  - "CorrectionLog stores entries in corrections_log.json in app_data_dir — same location pattern as history.json"
  - "promote threshold is 3 — same from/to pair must be submitted 3 times before auto-adding to dictionary"
  - "raw_text only passed to history when defillered != formatted_for_tooltip (corrections actually changed output)"
  - "Word-level diff uses positional comparison — splits by whitespace, compares same-index tokens, skips unmatched tail"
  - "History cards changed from button to div to allow nested buttons (copy + edit) without invalid HTML nesting"
metrics:
  duration_minutes: 15
  tasks_completed: 2
  files_created: 1
  files_modified: 4
  completed_date: "2026-03-05"
---

# Quick Task 45: Learn From User Corrections to Auto-Improve Dictionary — Summary

**One-liner:** Dragon NaturallySpeaking-style adaptive learning — tracks user-corrected word pairs with occurrence counts and auto-promotes corrections to the dictionary at 3 repetitions.

## What Was Built

### Backend: `src-tauri/src/correction_log.rs`

New module providing `CorrectionLog` — a Vec-backed store of `CorrectionEntry { from, to, count }` pairs.

- `load(app)` — reads `corrections_log.json` from app_data_dir, returns empty on missing/corrupt
- `save(app)` — writes JSON to disk
- `record(from, to) -> Option<PromotedCorrection>` — increments count for existing pair (case-insensitive on `from`), inserts new with count=1; returns `Some` only when count transitions from below to at/above threshold (3)
- `remove(from, to)` — removes entry (for undo)
- `CorrectionLogState(Mutex<CorrectionLog>)` managed state
- `PromotedCorrection { from, to }` serializable struct for frontend notification

### Backend: `src-tauri/src/history.rs`

Added `raw_text: Option<String>` field to `HistoryEntry` with `#[serde(default)]` for backward compatibility. Updated `append_history` signature to accept `raw_text: Option<&str>`.

### Backend: `src-tauri/src/pipeline.rs`

After corrections are applied, passes `Some(&defillered)` to `append_history` when the pre-correction text differs from the formatted result. Passes `None` when corrections did not change the output (no raw text needed — no correction editor button shown).

### Backend: `src-tauri/src/lib.rs`

- Added `mod correction_log;`
- Registered `CorrectionLogState` in `setup()` before `build_tray`
- Added `submit_correction(app, from, to)` IPC: calls `record()`, on threshold hit updates `ActiveProfile.corrections`, rebuilds `CorrectionsEngine`, persists to `settings.json`, returns `PromotedCorrection`
- Added `undo_promotion(app, from, to)` IPC: removes from `ActiveProfile.corrections`, rebuilds engine, persists, removes from correction log
- Both commands registered in `invoke_handler`

### Frontend: `src/components/sections/HistorySection.tsx`

- Added `rawText?: string` to `HistoryEntry` TypeScript interface
- Added `PromotedCorrection` interface
- History cards changed from `<button>` to `<div>` to allow nested interactive elements
- Separate copy button replaces the full-card-click copy behavior
- Pencil edit button appears per entry only when `rawText` is present
- Clicking pencil opens inline textarea pre-filled with `rawText`
- "Submit Correction" button runs `extractWordDiffs(rawText, editedText)` — positional word comparison — then calls `invoke('submit_correction', {from, to})` for each diff pair
- If any `submit_correction` returns a non-null `PromotedCorrection`, shows a green notification banner: "Auto-added to dictionary: {from} → {to}" with Undo + dismiss buttons
- "Undo" calls `invoke('undo_promotion', {from, to})` and dismisses the notification
- Notification auto-dismisses after 10 seconds via `setTimeout`
- Styling consistent with existing dark mode patterns

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] History entries changed from `<button>` to `<div>` with separate copy button**
- **Found during:** Task 2
- **Issue:** The original design wrapped each history entry in a `<button>` that triggered copy on click. Nested `<button>` elements inside a `<button>` is invalid HTML and browsers either block clicks or behave inconsistently.
- **Fix:** Changed history card container from `<button>` to `<div>`. Extracted copy action into a dedicated `<button>` within the card. The pencil edit button is a separate `<button>` alongside it.
- **Files modified:** `src/components/sections/HistorySection.tsx`
- **Commit:** 29e3c99

## Self-Check: PASSED

Files exist:
- FOUND: src-tauri/src/correction_log.rs
- FOUND: src-tauri/src/history.rs (modified)
- FOUND: src-tauri/src/pipeline.rs (modified)
- FOUND: src-tauri/src/lib.rs (modified)
- FOUND: src/components/sections/HistorySection.tsx (modified)

Commits exist:
- 12e084a: feat(quick-45): backend correction log, raw_text in history, IPC commands
- 29e3c99: feat(quick-45): inline correction editor in history section

Verification:
- cargo check: PASSED (0 errors)
- npx tsc --noEmit: PASSED (0 errors)
