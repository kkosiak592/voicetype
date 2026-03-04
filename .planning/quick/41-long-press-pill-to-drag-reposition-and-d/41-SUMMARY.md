---
phase: quick-41
plan: 01
subsystem: pill-overlay
tags: [drag, reposition, long-press, pill, settings-persistence]
dependency_graph:
  requires: []
  provides: [pill-drag-reposition, pill-position-persistence]
  affects: [src/Pill.tsx, src/pill.css, src-tauri/src/pill.rs, src-tauri/src/lib.rs]
tech_stack:
  added: []
  patterns: [pointer-capture-drag, tauri-ipc-command, settings-persistence, deferred-event-queue]
key_files:
  created: []
  modified:
    - src/Pill.tsx
    - src/pill.css
    - src-tauri/src/pill.rs
    - src-tauri/src/lib.rs
decisions:
  - read_settings/write_settings promoted to pub(crate) — pill.rs is in same crate and needs settings access
  - compute_home_position() extracted as helper called by both show_pill() and reset_pill_position()
  - Pointer capture used for drag so mouse events continue even when cursor leaves the pill window
  - screenX/screenY - half pill dims for centering under cursor (no need to query window position)
  - 16ms throttle on set_pill_position invoke to cap at ~60fps
  - Deferred hide pattern — pill-hide/pill-result events queued during drag and flushed on pointerup
metrics:
  duration: "~15 minutes"
  completed_date: "2026-03-04"
  tasks_completed: 3
  files_modified: 4
---

# Phase quick-41 Plan 01: Long-Press Pill Drag Reposition Summary

**One-liner:** iPhone-style long-press-to-drag pill repositioning with purple glow, screenX/screenY centering, settings.json persistence, double-click home reset, and deferred hide during drag.

## What Was Built

Long-press drag repositioning for the pill overlay window:

- **600ms long-press** on the pill enters drag mode (purple glow + scale animation)
- **Dragging** moves the pill window in real time via `set_pill_position` IPC (throttled to 16ms)
- **Release** exits drag mode; position is already persisted to `settings.json`
- **Double-click** calls `reset_pill_position`, which clears the saved position and snaps pill to bottom-center
- **App restart** — `show_pill()` now reads `pill_position` from settings and uses it if present
- **Deferred hide** — pill-hide/pill-result events during active drag are queued and flushed when the drag ends, preventing interrupted repositioning when hold-to-talk hotkey is released mid-drag

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add IPC commands and update show_pill | 84dda35 | src-tauri/src/pill.rs, src-tauri/src/lib.rs |
| 2 | Add long-press drag to Pill.tsx + CSS | 7ada147 | src/Pill.tsx, src/pill.css |
| 3 | Human verification + deferred hide fix | fa5dc52 | src/Pill.tsx |

## Deviations from Plan

**1. [Rule 1 - Bug] read_settings/write_settings were private**
- **Found during:** Task 1
- **Issue:** `read_settings` and `write_settings` in lib.rs were `fn` (private), not accessible from pill.rs
- **Fix:** Changed both to `pub(crate) fn`
- **Files modified:** src-tauri/src/lib.rs
- **Commit:** 84dda35

**2. [Rule 1 - Bug] Pill hide interrupts active drag**
- **Found during:** Post-checkpoint verification
- **Issue:** If the hold-to-talk hotkey is released while the user is mid-drag, pill-hide/pill-result fires and hides the pill immediately, breaking the drag interaction
- **Fix:** Deferred hide pattern — hide events during active drag are queued and flushed when pointerup fires
- **Files modified:** src/Pill.tsx
- **Commit:** fa5dc52

## Self-Check: PASSED

All files exist and all commits verified:
- 84dda35: feat(quick-41): add set_pill_position and reset_pill_position IPC commands
- 7ada147: feat(quick-41): add long-press drag interaction and double-click reset to pill
- fa5dc52: fix(quick-41): defer pill hide during drag mode
