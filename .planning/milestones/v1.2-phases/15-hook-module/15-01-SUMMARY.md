---
phase: 15-hook-module
plan: "01"
subsystem: infra
tags: [windows, win32, keyboard-hook, wh_keyboard_ll, mpsc, rust, tauri]

# Dependency graph
requires: []
provides:
  - WH_KEYBOARD_LL hook thread lifecycle (install/run/shutdown)
  - HookHandle with uninstall() and Drop safety net
  - HOOK_TX OnceLock SyncSender<HookEvent> for zero-allocation hook callback
  - hook_proc skeleton with LLKHF_INJECTED guard
  - Dispatcher thread skeleton for HookEvent routing
  - DeviceEventFilter::Always on Tauri Builder (fix tauri#13919)
  - Windows crate feature flags for Win32 hook APIs
affects:
  - 15-hook-module/15-02 (state machine fills hook_proc body)
  - 15-hook-module/15-03 (wires install() into app setup, dispatcher to handle_shortcut)

# Tech tracking
tech-stack:
  added:
    - windows crate features: Win32_Foundation, Win32_UI_WindowsAndMessaging, Win32_UI_Input_KeyboardAndMouse, Win32_System_Threading
  patterns:
    - Dedicated OS thread (std::thread, not tokio) for WH_KEYBOARD_LL — required for Win32 message pump
    - OnceLock for once-set global state shared between install() and hook_proc
    - Bounded mpsc::sync_channel(32) for hook_proc-to-dispatcher communication (never blocks in callback)
    - Drop impl on handle types as cleanup safety net

key-files:
  created:
    - src-tauri/src/keyboard_hook.rs
  modified:
    - src-tauri/Cargo.toml
    - src-tauri/src/lib.rs

key-decisions:
  - "std::thread::spawn used for hook thread — tokio task cannot host WH_KEYBOARD_LL (requires stable OS thread with Win32 message pump)"
  - "hmod=None in SetWindowsHookExW — required when dwThreadId=0 (global hook); using HMODULE from GetModuleHandle is incorrect and can cause hook removal"
  - "LLKHF_INJECTED guard in hook_proc prevents infinite loop when Plan 02 injects synthetic VK_E8 keystrokes"

patterns-established:
  - "Hook thread pattern: spawn thread, store thread_id in AtomicU32, run GetMessageW loop, UnhookWindowsHookEx on WM_QUIT"
  - "Sub-millisecond hook_proc constraint: no allocation, no Mutex, no async — only AtomicBool/AtomicU32 and mpsc::try_send"

requirements-completed: [HOOK-01, HOOK-02, HOOK-03, HOOK-04]

# Metrics
duration: 15min
completed: 2026-03-02
---

# Phase 15 Plan 01: Hook Module Infrastructure Summary

**WH_KEYBOARD_LL hook thread with GetMessageW loop, bounded mpsc channel, HookHandle shutdown protocol, and LLKHF_INJECTED guard — compilable skeleton ready for Plan 02 state machine**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-03-02T22:28:00Z
- **Completed:** 2026-03-02T22:43:48Z
- **Tasks:** 2
- **Files modified:** 3 (Cargo.toml, lib.rs, keyboard_hook.rs)

## Accomplishments

- Keyboard hook module created with full install/uninstall lifecycle and clean shutdown path (PostThreadMessageW WM_QUIT -> GetMessageW returns 0 -> UnhookWindowsHookEx)
- hook_proc skeleton with LLKHF_INJECTED guard (prevents infinite loop from Plan 02's VK_E8 injection) and sub-millisecond CallNextHookEx passthrough
- Tauri DeviceEventFilter::Always applied to Builder (required fix for tauri#13919 — without it WH_KEYBOARD_LL does not fire when Tauri window has focus)

## Task Commits

Each task was committed atomically:

1. **Task 1: Add Windows crate feature flags and DeviceEventFilter fix** - `ab2a97b` (chore)
2. **Task 2: Create keyboard_hook.rs with hook thread lifecycle and channel bridge** - `83870d2` (feat)

## Files Created/Modified

- `src-tauri/src/keyboard_hook.rs` - Hook thread, HookHandle, hook_proc skeleton, HookEvent enum, OnceLock channel, dispatcher thread
- `src-tauri/Cargo.toml` - Added Win32_Foundation, Win32_UI_WindowsAndMessaging, Win32_UI_Input_KeyboardAndMouse, Win32_System_Threading feature flags
- `src-tauri/src/lib.rs` - DeviceEventFilter::Always on Builder, `#[cfg(windows)] mod keyboard_hook` declaration

## Decisions Made

- Used `std::thread::spawn` with named threads ("keyboard-hook", "hook-dispatcher") — tokio tasks cannot host WH_KEYBOARD_LL since the hook requires a stable OS thread with a Win32 message pump.
- `hmod=None` in `SetWindowsHookExW` when `dwThreadId=0` — this is the correct calling convention for global hooks; passing `HMODULE` from `GetModuleHandle` is incorrect and causes silent hook removal on some Windows versions.
- `LLKHF_INJECTED` check in hook_proc ensures Plan 02's synthetic VK_E8 key injection (for Start menu suppression) does not recurse through the hook.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 02 can now fill in the hook_proc body with the modifier state machine (Win + Win alone detection, debounce, Start menu suppression via VK_E8 injection)
- Plan 03 can wire `install()` into the Tauri app setup and connect the dispatcher to `handle_shortcut()`
- `cargo check` passes cleanly — all Win32 imports resolve, no errors

---
*Phase: 15-hook-module*
*Completed: 2026-03-02*
