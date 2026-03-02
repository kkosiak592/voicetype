---
phase: 15-hook-module
plan: "03"
subsystem: infra
tags: [rust, windows-api, keyboard-hook, wh-keyboard-ll, tauri, global-shortcut, handle-hotkey-event]

# Dependency graph
requires:
  - phase: 15-01
    provides: keyboard_hook.rs skeleton with HookHandle, install(), hook_proc stub, HOOK_TX
  - phase: 15-02
    provides: ModifierState machine, inject_mask_key, hook_proc dispatching Pressed/Released via HOOK_TX

provides:
  - handle_hotkey_event(app, pressed: bool) as pub(crate) — shared hotkey entry point for both code paths
  - handle_shortcut() thinned to delegate to handle_hotkey_event
  - dispatch_hook_event() wired to crate::handle_hotkey_event (Pressed/Released)
  - is_hook_hotkey() helper: returns true for "ctrl+win" modifier-only combos
  - HookHandleState managed state (Mutex<Option<HookHandle>>) registered on Builder
  - Conditional hook install in setup(): WH_KEYBOARD_LL for ctrl+win, global-shortcut for standard combos
  - Default hotkey changed from "ctrl+shift+space" to "ctrl+win" (fresh installs only)
  - Hook cleanup in tray quit handler before app.exit(0)

affects: [15-04, 16]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "handle_hotkey_event(pressed: bool) as shared entry point — avoids constructing private ShortcutEvent type"
    - "Conditional hotkey routing: is_hook_hotkey() gates WH_KEYBOARD_LL vs global-shortcut"
    - "HookHandleState managed state for cross-boundary hook lifecycle management"
    - "Explicit hook uninstall in tray quit handler — belt-and-suspenders on top of HookHandle::Drop"

key-files:
  created: []
  modified:
    - src-tauri/src/lib.rs
    - src-tauri/src/keyboard_hook.rs
    - src-tauri/src/tray.rs

key-decisions:
  - "handle_hotkey_event(pressed: bool) avoids constructing private ShortcutEvent — both code paths converge on bool"
  - "Tauri v2 Builder.run() takes only Context, no RunEvent callback — hook cleanup placed in tray quit handler instead"
  - "Default hotkey changed to ctrl+win — only affects fresh installs; existing users keep saved hotkey from settings.json"
  - "HookHandleState registered on Builder (not setup) — available for cleanup regardless of setup() completion"
  - "global-shortcut plugin still registered for non-hook hotkeys — no removal, conditional skip only"

patterns-established:
  - "Shared hotkey entry point pattern: pub(crate) fn handle_hotkey_event(app, pressed) centralizes recording logic"
  - "Conditional plugin registration: bool flag (hook_active) gates global-shortcut plugin install"

requirements-completed: [INT-01]

# Metrics
duration: 8min
completed: 2026-03-02
---

# Phase 15 Plan 03: Hook Integration Summary

**End-to-end Ctrl+Win hold-to-talk wiring: handle_hotkey_event extracted as shared entry point, dispatch_hook_event connected, conditional hook install in setup(), default hotkey changed to ctrl+win, HookHandleState managed state, clean shutdown via tray quit handler**

## Performance

- **Duration:** 8 min
- **Started:** 2026-03-02T22:52:38Z
- **Completed:** 2026-03-02T23:00:33Z
- **Tasks:** 1 (of 2 — Task 2 is human-verify checkpoint)
- **Files modified:** 3

## Accomplishments

- `handle_hotkey_event(app: &AppHandle, pressed: bool)` extracted as `pub(crate)` — both the global-shortcut path and WH_KEYBOARD_LL path converge here, avoiding construction of the private `ShortcutEvent` type
- `handle_shortcut()` reduced to 3-line wrapper matching on `ShortcutState::Pressed/Released`
- `dispatch_hook_event()` in `keyboard_hook.rs` now calls `crate::handle_hotkey_event(app, true/false)`
- `is_hook_hotkey()` helper added: case-insensitive match on "ctrl+win"
- `HookHandleState(Mutex<Option<HookHandle>>)` registered on Builder before `setup()`
- `setup()` conditionally installs WH_KEYBOARD_LL hook when `is_hook_hotkey(&hotkey)` returns true; stores handle in `HookHandleState`
- Global-shortcut plugin registration gated behind `!hook_active` — still registered for standard combos
- Default hotkey changed from `"ctrl+shift+space"` to `"ctrl+win"` (fresh install default only)
- Tray quit handler (`"quit"` menu item) explicitly calls `handle.uninstall()` before `app.exit(0)`
- `cargo check` and `cargo build` both pass

## Task Commits

Each task was committed atomically:

1. **Task 1: Wire dispatcher to handle_shortcut and setup() to install hook conditionally** - `b94f7b3` (feat)

**Plan metadata:** (docs commit — see below)

## Files Created/Modified

- `src-tauri/src/lib.rs` — Extracted `handle_hotkey_event`, thinned `handle_shortcut`, added `is_hook_hotkey`, `HookHandleState`, changed default hotkey, added conditional hook install in setup()
- `src-tauri/src/keyboard_hook.rs` — Updated `dispatch_hook_event` to call `crate::handle_hotkey_event`
- `src-tauri/src/tray.rs` — Added explicit hook uninstall in "quit" menu handler

## Decisions Made

- **handle_hotkey_event(pressed: bool)** rather than constructing a synthetic ShortcutEvent: `ShortcutEvent` fields are not public in tauri-plugin-global-shortcut, so constructing it from the hook dispatcher is not possible. A simple `bool` parameter is sufficient and creates a cleaner internal API.
- **Tauri v2 `.run()` only accepts Context, not a RunEvent callback**: The plan's Part E suggested using `RunEvent::Exit` but this API does not exist in Tauri v2 (`Builder::run` signature: `fn run(self, context: Context<R>) -> Result<()>`). Instead, cleanup was placed in the tray "quit" handler where `app.exit(0)` is called — the only code path for graceful shutdown. `HookHandle::Drop` remains as the safety net for crashes.
- **HookHandleState on Builder, not setup()**: Ensures the state is available for cleanup in the tray handler even if the app starts partially and setup() doesn't complete fully.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Tauri v2 RunEvent callback does not exist in Builder::run()**
- **Found during:** Task 1 (Part E — Hook cleanup on app exit)
- **Issue:** Plan specified `.run(context, |app_handle, event| { ... })` but Tauri v2's `Builder::run()` takes only one argument (the context). The two-argument callback form is a Tauri v1 API that was removed.
- **Fix:** Moved hook cleanup to tray quit handler (`"quit"` match arm in `tray.rs`) which is the only graceful shutdown path. `HookHandle::Drop` serves as safety net for all other exit paths.
- **Files modified:** `src-tauri/src/tray.rs`, removed incorrect `.run()` callback from `src-tauri/src/lib.rs`
- **Verification:** `cargo check` passes; cleanup code compiles; `HookHandle::Drop` still present as fallback
- **Committed in:** `b94f7b3` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 bug — API version mismatch)
**Impact on plan:** Required fix. Cleanup is equivalent in behavior: tray quit is the only graceful path; Drop handles all others.

## Issues Encountered

None beyond the Tauri v2 API deviation above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Full pipeline wired: Ctrl+Win hold via WH_KEYBOARD_LL → `dispatch_hook_event` → `handle_hotkey_event` → recording → transcription → inject
- Human verification (Task 2) pending: 7 test scenarios covering hold-to-talk, reversed key order, Start menu suppression, shutdown, rapid activation, exact match, Tauri window focus
- No blockers

## Self-Check: PASSED

- FOUND: src-tauri/src/keyboard_hook.rs
- FOUND: src-tauri/src/lib.rs
- FOUND: src-tauri/src/tray.rs
- FOUND: .planning/phases/15-hook-module/15-03-SUMMARY.md
- FOUND commit: b94f7b3

---
*Phase: 15-hook-module*
*Completed: 2026-03-02*
