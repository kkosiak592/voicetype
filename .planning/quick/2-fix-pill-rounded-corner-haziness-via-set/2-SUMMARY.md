---
phase: quick-2
plan: "01"
subsystem: pill-overlay
tags: [windows, dwm, shadow, pill, tauri]
dependency_graph:
  requires: []
  provides: [clean-pill-corners]
  affects: [pill-overlay]
tech_stack:
  added: []
  patterns: [tauri-window-api, capability-permissions]
key_files:
  created: []
  modified:
    - src-tauri/capabilities/default.json
    - src-tauri/src/lib.rs
decisions:
  - "set_shadow(false) chosen over DWM composition attributes — Tauri API is cross-platform safe and requires no unsafe Win32 calls"
metrics:
  duration: "~3 min"
  completed_date: "2026-02-28"
---

# Quick Task 2: Fix Pill Rounded Corner Haziness via set_shadow Summary

**One-liner:** Disabled DWM window shadow on pill overlay via `set_shadow(false)` + `allow-set-shadow` permission to eliminate rectangular haze artifact around CSS border-radius corners on Windows 10.

## What Was Done

DWM applies a rectangular drop shadow to transparent undecorated windows that does not respect CSS `border-radius`. This caused a ghostly rectangular haze visible around the pill shape's corners.

Two minimal edits fix this:

1. **`src-tauri/capabilities/default.json`** — Added `"core:window:allow-set-shadow"` to the permissions array after `allow-set-focusable`.
2. **`src-tauri/src/lib.rs`** — Added `let _ = pill_window.set_shadow(false);` immediately after `set_focusable(false)` in the pill window setup block, with a comment referencing tauri#11321.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add set-shadow permission and disable DWM shadow on pill window | 527b0f9 | src-tauri/capabilities/default.json, src-tauri/src/lib.rs |

## Checkpoint Pending

| Task | Name | Status |
|------|------|--------|
| 2 | Verify pill corners are clean | Awaiting human verification |

## Deviations from Plan

None - plan executed exactly as written.

## Key Decisions

- `set_shadow(false)` over Win32 DWM composition API: Tauri's built-in API is the correct abstraction layer, avoids unsafe code, and is the established pattern in this codebase (same as `set_focusable`).

## Self-Check: PASSED

- `src-tauri/capabilities/default.json` contains `allow-set-shadow`: confirmed via grep
- `src-tauri/src/lib.rs` contains `set_shadow(false)`: confirmed via grep
- Commit 527b0f9 exists: confirmed
