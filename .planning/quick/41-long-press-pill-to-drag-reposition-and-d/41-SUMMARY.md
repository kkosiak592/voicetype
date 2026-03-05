---
phase: quick-41
plan: 01
subsystem: pill-overlay
tags: [drag, reposition, long-press, pill, move-mode, settings-persistence, multi-monitor]
dependency_graph:
  requires: []
  provides: [pill-drag-reposition, pill-position-persistence, pill-move-mode]
  affects: [src/Pill.tsx, src/pill.css, src-tauri/src/pill.rs, src-tauri/src/lib.rs]
tech_stack:
  added: [Win32 GetCursorPos]
  patterns: [backend-cursor-polling, fractional-offset-persistence, deferred-event-queue, tauri-ipc-command]
key_files:
  created: []
  modified:
    - src/Pill.tsx
    - src/pill.css
    - src-tauri/src/pill.rs
    - src-tauri/src/lib.rs
decisions:
  - read_settings/write_settings promoted to pub(crate) for cross-module access
  - compute_home_position() extracted as helper for show_pill() and reset_pill_position()
  - Reworked from "drag mode" to "move mode" — distinct visual state with green breathing glow and move icon
  - Frontend mousemove abandoned — DOM events only fire within 178x46 webview, cursor escapes immediately
  - Backend tokio loop polls GetCursorPos every 16ms for smooth cross-monitor tracking
  - Win32 GetCursorPos used instead of Tauri cursor_position() — Tauri returns window-relative coords that break on monitor transitions
  - Position saved as fractional offsets (0.0-1.0) relative to work area, not absolute pixels — works across monitors and resolution changes
  - Deferred hide pattern — pill-hide/pill-result events queued during move mode and flushed on exit
  - Click pill or press hotkey to exit move mode
metrics:
  duration: "~45 minutes"
  completed_date: "2026-03-04"
  tasks_completed: 3
  files_modified: 4
---

# Phase quick-41 Plan 01: Long-Press Pill Drag Reposition Summary

**One-liner:** Long-press move mode with green breathing glow, backend Win32 GetCursorPos polling loop for cross-monitor tracking, fractional-offset position persistence, click/hotkey to exit, and double-click home reset.

## What Was Built

Move-mode repositioning for the pill overlay window with multi-monitor support:

- **600ms long-press** enters "move mode" — green breathing glow with move icon replaces frequency bars
- **Backend Rust loop** polls Win32 `GetCursorPos` every 16ms, calling `set_position()` directly — pill follows cursor smoothly across all monitors
- **Click pill or press hotkey** to exit move mode; position is persisted on exit (not every tick)
- **Fractional offset persistence** — position saved as (0.0-1.0) offsets relative to work area, so placing the pill top-right on monitor A puts it top-right on monitor B
- **Double-click** calls `reset_pill_position`, clears saved position, snaps pill to bottom-center
- **Deferred hide** — pill-hide/pill-result events during move mode are queued and flushed when move mode ends
- **App restart** — `show_pill()` reads saved fractional offsets and applies them to the current cursor's monitor

## All Commits

| # | Commit | Description | Files |
|---|--------|-------------|-------|
| 1 | 84dda35 | Rust IPC commands + show_pill saved position | src-tauri/src/pill.rs, src-tauri/src/lib.rs |
| 2 | 7ada147 | Frontend drag UX + CSS | src/Pill.tsx, src/pill.css |
| 3 | fa5dc52 | Deferred pill hide during drag mode | src/Pill.tsx |
| 4 | 65b7d8f | Rework into move mode with green glow + move icon | src/Pill.tsx, src/pill.css, src-tauri/src/pill.rs, src-tauri/src/lib.rs |
| 5 | e175bd8 | Click pill to exit move mode | src/Pill.tsx |
| 6 | 73eba41 | Move tracking from frontend to backend tokio loop | src-tauri/src/pill.rs, src/Pill.tsx |
| 7 | 5c557b1 | Win32 GetCursorPos for multi-monitor tracking | src-tauri/src/pill.rs |
| 8 | 70df12c | Fractional offset persistence for multi-monitor | src-tauri/src/pill.rs |

## Deviations from Plan

**1. [Rule 1 - Bug] read_settings/write_settings were private**
- **Found during:** Task 1
- **Issue:** `read_settings` and `write_settings` in lib.rs were `fn` (private), not accessible from pill.rs
- **Fix:** Changed both to `pub(crate) fn`
- **Commit:** 84dda35

**2. [Rule 1 - Bug] Pill hide interrupts active drag**
- **Found during:** Post-checkpoint verification
- **Issue:** Hold-to-talk hotkey release during drag fires pill-hide, breaking interaction
- **Fix:** Deferred hide pattern — events queued during move mode, flushed on exit
- **Commit:** fa5dc52

**3. [Rule 3 - Blocking] Purple drag mode replaced with green move mode**
- **Found during:** Post-checkpoint refinement
- **Issue:** Original "drag" metaphor (pointer capture, release to drop) didn't fit the UX — needed a distinct "move mode" with explicit exit
- **Fix:** Reworked into move mode with green breathing glow, move icon, click/hotkey to exit, backend PillMoveActive flag
- **Commit:** 65b7d8f

**4. [Rule 1 - Bug] Frontend mousemove only fires within 178x46 webview**
- **Found during:** Testing move mode
- **Issue:** `document.addEventListener("mousemove")` only fires when cursor is inside the tiny pill webview — cursor escapes immediately and events stop
- **Fix:** Moved tracking to backend tokio loop polling cursor_position() every 16ms
- **Commit:** 73eba41

**5. [Rule 1 - Bug] Tauri cursor_position() returns window-relative coordinates**
- **Found during:** Testing cross-monitor movement
- **Issue:** Tauri's `cursor_position()` returns coordinates relative to the window, breaking when cursor crosses monitor boundaries
- **Fix:** Switched to Win32 `GetCursorPos` for absolute screen coordinates
- **Commit:** 5c557b1

**6. [Rule 2 - Missing critical] Absolute pixel positions don't transfer across monitors**
- **Found during:** Multi-monitor testing
- **Issue:** Saving absolute (x, y) pins the pill to one specific monitor position — placing it on monitor A means it appears on monitor A even when working on monitor B
- **Fix:** Position saved as fractional offsets (0.0-1.0) relative to work area; applied to whichever monitor the cursor is on when pill shows
- **Commit:** 70df12c

## Self-Check: PASSED

All commits verified present in git log. All modified files exist on disk.
