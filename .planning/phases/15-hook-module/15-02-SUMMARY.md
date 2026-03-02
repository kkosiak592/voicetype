---
phase: 15-hook-module
plan: "02"
subsystem: infra
tags: [rust, windows-api, keyboard-hook, atomics, wh-keyboard-ll, sendInput, vk-e8]

# Dependency graph
requires:
  - phase: 15-01
    provides: keyboard_hook.rs skeleton with HOOK_TX, HookHandle, install(), hook_proc stub

provides:
  - ModifierState struct with 6 atomic fields (ctrl_held, win_held, shift_held, alt_held, combo_active, first_key_time)
  - hook_proc state machine detecting Ctrl+Win in either order within 50ms
  - inject_mask_key() sending VK_E8 via SendInput for Start menu suppression
  - reset_state() for cleanup on hook thread exit
  - Fully functional hook_proc dispatching Pressed/Released events via HOOK_TX

affects: [15-03, 15-04]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "AtomicBool/AtomicU32 with Relaxed ordering for single-thread hook callback state"
    - "wrapping_sub for u32 GetTickCount debounce to handle 49-day wraparound"
    - "LRESULT(1) return to suppress Win key events (prevents Start menu)"
    - "VK_E8 mask-key injection via SendInput between Win-down and Win-up"
    - "LLKHF_INJECTED guard prevents infinite recursion from synthetic key injection"
    - "try_send over send in hook callback — channel-full events are dropped with warn log"

key-files:
  created: []
  modified:
    - src-tauri/src/keyboard_hook.rs

key-decisions:
  - "Tasks 1 and 2 implemented in single write — inject_mask_key is called inline from hook_proc, atomically correct"
  - "Shift/Alt keydown returns CallNextHookEx immediately after state update — does not fall through to combo logic"
  - "Repeated Win keydown during active combo (held-key repetition) suppressed with inject+LRESULT(1)"

patterns-established:
  - "Win key suppression pattern: inject VK_E8 then return LRESULT(1) on KEYDOWN, return LRESULT(1) on KEYUP"
  - "Debounce pattern: store first_key_time when first modifier pressed alone, wrapping_sub on second modifier"
  - "Exact-match pattern: shift_held && alt_held checked before activating combo"

requirements-completed: [MOD-01, MOD-02, MOD-03, MOD-04, MOD-05]

# Metrics
duration: 2min
completed: 2026-03-02
---

# Phase 15 Plan 02: Hook State Machine Summary

**Ctrl+Win modifier state machine in hook_proc with 50ms wrapping_sub debounce, exact-match enforcement (no Shift/Alt), and VK_E8 SendInput injection for Start menu suppression**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-02T22:46:34Z
- **Completed:** 2026-03-02T22:48:32Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments

- ModifierState with 6 atomic fields (ctrl_held, win_held, shift_held, alt_held, combo_active, first_key_time) stored as static STATE
- hook_proc state machine handles all 10 relevant virtual keys: VK_LCONTROL, VK_RCONTROL, VK_LWIN, VK_RWIN, VK_SHIFT, VK_LSHIFT, VK_RSHIFT, VK_MENU, VK_LMENU, VK_RMENU
- 50ms debounce using `wrapping_sub` for safe u32 GetTickCount wraparound (49-day clock rollover)
- Exact-match enforcement: shift_held or alt_held blocks combo activation (Ctrl+Win+Shift does not fire)
- Win key suppression via LRESULT(1) on KEYDOWN and KEYUP when combo active — prevents Start menu (MOD-04)
- Win alone passes through CallNextHookEx — Start menu works normally when no combo (MOD-05)
- inject_mask_key() sends VK_E8 (0xE8) via SendInput between Win-down and Win-up to break Start menu sequence
- LLKHF_INJECTED guard in hook_proc prevents infinite recursion from synthetic VK_E8 injection
- reset_state() called on hook thread exit, clears all atomics
- try_send used throughout — channel-full events dropped with warn log, hook callback never blocks

## Task Commits

Each task was committed atomically:

1. **Tasks 1+2: Modifier state machine + VK_E8 injection (implemented together)** - `3252ed9` (feat)

**Plan metadata:** (docs commit — see below)

## Files Created/Modified

- `src-tauri/src/keyboard_hook.rs` - Full modifier state machine replacing Plan 01 TODO stub; added ModifierState, inject_mask_key(), reset_state(), complete hook_proc implementation

## Decisions Made

- Tasks 1 and 2 were implemented in a single write because inject_mask_key() is called inline from hook_proc — separating them would have required a second cargo check cycle with an intermediate broken state. The combined implementation is the minimal correct unit.
- Shift and Alt key handlers return CallNextHookEx immediately after storing state, preventing any fall-through to combo logic for those keys.
- Repeated Win keydown during active combo (OS key-repeat while Win is held) is suppressed with inject_mask_key() + LRESULT(1) to prevent Start menu from triggering mid-recording.

## Deviations from Plan

None - plan executed exactly as written. Both tasks implemented as specified; the combined commit reflects that inject_mask_key() and the state machine are a single logical unit, not a sequencing deviation.

## Issues Encountered

None. `cargo check` passed on first attempt with no errors (11 pre-existing unused-function warnings from Plan 01 stubs persist, expected until Plan 03).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- hook_proc now sends Pressed/Released through HOOK_TX when Ctrl+Win is held/released
- Plan 03 (dispatcher wiring) can connect dispatch_hook_event() to handle_shortcut()
- Start menu suppression logic is in place and ready for empirical Windows 11 validation (Research Flag: KEYDOWN-only vs KEYDOWN+KEYUP for VK_E8 injection)
- No blockers

## Self-Check: PASSED

- FOUND: src-tauri/src/keyboard_hook.rs
- FOUND: .planning/phases/15-hook-module/15-02-SUMMARY.md
- FOUND commit: 3252ed9

---
*Phase: 15-hook-module*
*Completed: 2026-03-02*
