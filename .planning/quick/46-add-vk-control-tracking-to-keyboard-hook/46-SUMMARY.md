---
phase: quick-46
plan: 01
subsystem: keyboard-hook
tags: [keyboard-hook, windows, hotkey, ctrl, vk-control]
dependency_graph:
  requires: []
  provides: [vk-control-tracking]
  affects: [keyboard_hook.rs]
tech_stack:
  added: []
  patterns: [generic-modifier-tracking]
key_files:
  modified:
    - src-tauri/src/keyboard_hook.rs
decisions:
  - "VK_CONTROL added to same condition as VK_LCONTROL/VK_RCONTROL -- matches existing Shift and Alt pattern"
metrics:
  duration_seconds: 64
  completed: "2026-03-05T21:44:22Z"
  tasks_completed: 1
  tasks_total: 1
---

# Quick Task 46: Add VK_CONTROL Tracking to Keyboard Hook Summary

Added generic VK_CONTROL to Ctrl keydown/keyup conditions in keyboard_hook.rs, fixing Ctrl+Win combo detection in Outlook and Office apps that emit VK_CONTROL instead of VK_LCONTROL/VK_RCONTROL.

## Commits

| Task | Commit | Description |
|------|--------|-------------|
| 1 | 9d5bed0 | Add VK_CONTROL to Ctrl keydown and keyup conditions |

## Changes Made

### Task 1: Add VK_CONTROL to Ctrl keydown and keyup conditions

Two-line change in `src-tauri/src/keyboard_hook.rs`:

- **Line 272 (Ctrl keydown):** Added `|| vk == VK_CONTROL` to the existing `VK_LCONTROL || VK_RCONTROL` condition
- **Line 299 (Ctrl keyup):** Added `|| vk == VK_CONTROL` to the existing `VK_LCONTROL || VK_RCONTROL` condition

All three modifier families now consistently track generic + left + right variants:
- Shift: `VK_LSHIFT || VK_RSHIFT || VK_SHIFT`
- Alt: `VK_LMENU || VK_RMENU || VK_MENU`
- Ctrl: `VK_LCONTROL || VK_RCONTROL || VK_CONTROL`

## Verification

- `cargo check` passes with no errors
- `VK_CONTROL` confirmed present in both Ctrl tracking blocks
- Pattern consistency verified across all three modifier families

## Deviations from Plan

None -- plan executed exactly as written.
