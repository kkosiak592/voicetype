---
phase: 16-rebind-and-coexistence
plan: "01"
subsystem: hotkey
tags: [rust, tauri, keyboard-hook, global-shortcut, routing, windows]

requires:
  - phase: 15-hook-module
    provides: keyboard_hook::install/uninstall, HookHandle, dispatch_hook_event

provides:
  - is_modifier_only routing predicate (single source of truth for hook vs plugin dispatch)
  - Routed rebind_hotkey/unregister_hotkey/register_hotkey IPC commands
  - Startup routing with hook-failure fallback and settings.json persistence
  - HookAvailable managed state (Arc<AtomicBool>)
  - get_hook_status IPC command
  - PipelineState::current() read accessor

affects: [17-frontend-settings, 18-distribution]

tech-stack:
  added: []
  patterns:
    - is_modifier_only as single routing predicate — no duplicate modifier lists
    - HookHandleState(Mutex<Option<HookHandle>>) for managed hook lifecycle
    - HookAvailable(Arc<AtomicBool>) for cross-command availability status
    - Global-shortcut plugin always registered (with or without shortcuts) for runtime rebind support

key-files:
  created: []
  modified:
    - src-tauri/src/lib.rs
    - src-tauri/src/pipeline.rs

key-decisions:
  - "is_modifier_only predicate replaces hardcoded is_hook_hotkey — now handles any modifier-only combo, not just ctrl+win"
  - "Global-shortcut plugin registered even when hook is active (no shortcuts) — enables runtime rebind to standard combos without restart"
  - "rebind_hotkey attempts hook install on demand when HookAvailable is false and modifier-only combo requested — avoids startup probe for non-modifier hotkeys"
  - "Hook-failure fallback at startup persists ctrl+shift+space to settings.json — frontend reads from settings.json for displayed hotkey"
  - "rebind_hotkey checks PipelineState::current() before switching backends — prevents mid-recording backend swap"

patterns-established:
  - "Rule: All hotkey routing goes through is_modifier_only — never check key string directly outside this predicate"
  - "Rule: HookHandleState.take() + handle.uninstall() pattern for stopping hook (Drop is safety net only)"

requirements-completed: [INT-02, INT-03]

duration: 5min
completed: 2026-03-03
---

# Phase 16 Plan 01: Rebind and Coexistence — Backend Routing Summary

**is_modifier_only routing predicate wired into all three hotkey IPC commands and startup, with hook-failure fallback, HookAvailable managed state, and get_hook_status IPC**

## Performance

- **Duration:** 5 min
- **Started:** 2026-03-03T13:55:06Z
- **Completed:** 2026-03-03T13:59:42Z
- **Tasks:** 3
- **Files modified:** 2

## Accomplishments

- Added `PipelineState::current()` read accessor enabling mid-recording backend-switch guard in rebind_hotkey
- Replaced hardcoded `is_hook_hotkey("ctrl+win")` with `is_modifier_only()` — general predicate matching any modifier-only combo against a token list
- Routed all three IPC commands (`rebind_hotkey`, `unregister_hotkey`, `register_hotkey`) through `is_modifier_only()` with correct hook lifecycle (install/uninstall via HookHandleState)
- Added `HookAvailable(Arc<AtomicBool>)` managed state and `get_hook_status` IPC command
- Reworked startup routing: hook-failure fallback persists `ctrl+shift+space` to settings.json; global-shortcut plugin always registered for runtime rebind support

## Task Commits

Each task was committed atomically:

1. **Task 1: Add PipelineState::current(), is_modifier_only, HookAvailable, get_hook_status, startup routing** - `26a32fa` (feat)
2. **Task 2: Route rebind_hotkey, unregister_hotkey, register_hotkey via is_modifier_only** - `b898541` (feat)

Note: Task 3 (startup routing) was implemented as part of Task 1's commit since it was a blocking dependency (the old `is_hook_hotkey` call in setup() had to be replaced at the same time `is_modifier_only` was introduced).

## Files Created/Modified

- `src-tauri/src/lib.rs` - `is_modifier_only` predicate, `HookAvailable` struct, `get_hook_status` IPC, routed hotkey commands, startup routing with hook-failure fallback
- `src-tauri/src/pipeline.rs` - `PipelineState::current()` read accessor

## Decisions Made

- `is_modifier_only` uses a token list (`ctrl`, `alt`, `shift`, `meta`, `win`, `super`) rather than exact-matching "ctrl+win" — makes the predicate extensible to future modifier-only combos without code changes
- Global-shortcut plugin registered with no shortcuts on the hook-success path — avoids a startup panic if the user later calls `rebind_hotkey` to switch to a standard combo (plugin must be initialized before `GlobalShortcutExt` trait methods work)
- `rebind_hotkey` attempts hook install on-demand when `HookAvailable` is false and a modifier-only combo is requested — this means hook availability is only probed when actually needed, not eagerly at startup for all users
- Task 3 merged into Task 1 commit because the `is_hook_hotkey` reference in setup() would not compile without `is_modifier_only` being present

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Merged startup routing (Task 3) into Task 1 commit**
- **Found during:** Task 1 (adding is_modifier_only)
- **Issue:** The existing `is_hook_hotkey` reference in setup() would not compile after removing the old function. The cargo check step for Task 1 required the startup routing to be updated simultaneously.
- **Fix:** Implemented startup routing replacement as part of Task 1's implementation pass, committed together. Task 3 was then verified as already complete.
- **Files modified:** src-tauri/src/lib.rs
- **Verification:** cargo check passed after Task 1 commit
- **Committed in:** 26a32fa (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (blocking — compile dependency between Task 1 and Task 3)
**Impact on plan:** No scope change. Tasks 1 and 3 were naturally atomic from the compiler's perspective.

## Issues Encountered

None beyond the compile-dependency deviation documented above.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Backend routing complete: `rebind_hotkey`, `unregister_hotkey`, `register_hotkey` all route correctly
- `get_hook_status` IPC ready for frontend to query and display hook-failure warnings
- Phase 17 (frontend settings) can now wire `get_hook_status` into the UI and present modifier-only combo selection only when hook is available
- No blockers

---
*Phase: 16-rebind-and-coexistence*
*Completed: 2026-03-03*
