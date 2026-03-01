---
phase: quick-8
plan: 01
subsystem: pill-ui
tags: [pill, positioning, multi-monitor, drag-removal]
dependency_graph:
  requires: []
  provides: [pill-bottom-center-positioning, multi-monitor-support]
  affects: [src-tauri/src/pill.rs, src-tauri/src/lib.rs, src/Pill.tsx]
tech_stack:
  added: []
  patterns: [cursor-monitor-detection, work-area-positioning]
key_files:
  created: []
  modified:
    - src-tauri/src/pill.rs
    - src-tauri/src/lib.rs
    - src/Pill.tsx
decisions:
  - work_area() method used (available in Tauri 2.10.2) — returns PhysicalRect excluding taskbar
  - pill::show_pill() called at all 4 recording-start sites replacing raw emit_to
  - 14px margin_bottom above work area bottom edge for visual breathing room
metrics:
  duration: ~10 minutes
  completed_date: "2026-03-01"
  tasks_completed: 2
  tasks_total: 3
  files_modified: 3
---

# Phase quick-8 Plan 01: Fix Pill Position to Bottom-Center Above Taskbar Summary

**One-liner:** Pill now auto-positions to bottom-center of the cursor's monitor work area using Tauri's work_area() API, with all drag/saved-position logic removed.

## What Was Built

Multi-monitor aware pill positioning: a new `show_pill()` Rust function detects which monitor the cursor is on at the moment of hotkey press, computes the bottom-center of that monitor's work area (which excludes the taskbar), positions the pill window there, then emits `pill-show`. All four recording-start sites in `lib.rs` now call `pill::show_pill(&app)` instead of raw `app.emit_to("pill", "pill-show", ())`. The frontend `Pill.tsx` had its position-save/restore and drag handler code entirely removed.

## Tasks Completed

| # | Task | Commit | Status |
|---|------|--------|--------|
| 1 | Add show_pill() with multi-monitor positioning, replace all call sites | 69f28ea | Done |
| 2 | Remove drag logic and fixed-position init from Pill.tsx | 000463d | Done |
| 3 | Human verify pill positioning and multi-monitor behavior | — | Pending checkpoint |

## Key Changes

**src-tauri/src/pill.rs**
- Added `use tauri::Manager;` import
- Added `pub fn show_pill(app: &tauri::AppHandle)` that:
  - Gets cursor position via `pill_window.cursor_position()`
  - Finds which monitor the cursor is on by checking bounds
  - Reads `monitor.work_area()` (Tauri 2.10.2 — available)
  - Computes `x = wa_x + (wa_w - 178) / 2` and `y = wa_y + wa_h - 46 - 14`
  - Calls `pill_window.set_position()` then emits `pill-show`

**src-tauri/src/lib.rs**
- All 4 `app.emit_to("pill", "pill-show", ()).ok()` lines replaced with `pill::show_pill(&app)`
- Removed the saved-position restore block from `setup()` (17 lines deleted)
- Updated log message to `"Pill overlay window configured (focusable=false, no-shadow)"`

**src/Pill.tsx**
- Removed: `PhysicalPosition` import, `plugin-store` import, `useCallback` import
- Removed: `initPosition` useEffect (18 lines)
- Removed: `handleMouseDown`, `handleMouseUp` callbacks
- Removed: `onMouseDown`, `onMouseUp` event handlers and `cursor-grab active:cursor-grabbing` classes from root div

## Deviations from Plan

None — plan executed exactly as written. `work_area()` was confirmed available in Tauri 2.10.2 (no fallback needed).

## Self-Check

### Files verified:
- `src-tauri/src/pill.rs` — contains `pub fn show_pill`
- `src-tauri/src/lib.rs` — 4 occurrences of `pill::show_pill(&app)`, 0 occurrences of raw `pill-show` emit
- `src/Pill.tsx` — no drag handlers, no cursor-grab, no plugin-store import

### Commits verified:
- 69f28ea: feat(quick-8): add show_pill() with multi-monitor positioning, replace all call sites
- 000463d: feat(quick-8): remove drag logic and saved-position init from Pill.tsx

### Build status:
- `cargo check` passes (clean, no warnings in modified files)
- `npx tsc --noEmit` passes (no TypeScript errors)

## Self-Check: PASSED
