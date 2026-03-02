---
phase: 01-foundation
plan: 02
subsystem: ui
tags: [tauri, rust, global-shortcut, hotkey, windows, keyboard]

# Dependency graph
requires:
  - phase: 01-01
    provides: Tauri 2.0 app shell that compiles and runs on Windows
provides:
  - Global hotkey Ctrl+Shift+Space registered system-wide via tauri-plugin-global-shortcut
  - hotkey-triggered event emitted to frontend on keypress
  - rebind_hotkey Tauri command for dynamic hotkey re-registration
affects: [01-03, 04-overlay, 06-ux]

# Tech tracking
tech-stack:
  added:
    - tauri-plugin-global-shortcut 2.3.1 (target cfg not android/ios)
    - global-hotkey 0.7.0 (transitive dep)
    - "@tauri-apps/plugin-global-shortcut (npm)"
  patterns:
    - Global shortcut plugin registered in setup() via app.handle().plugin() with #[cfg(desktop)] guard, not at builder level
    - with_shortcuts() on Builder registers initial hotkeys; with_handler() fires for all registered shortcuts
    - on_shortcut() on GlobalShortcutExt provides per-shortcut handler for dynamic registration
    - rebind_hotkey command pattern - unregister old, register new with same handler using GlobalShortcutExt
    - use tauri::Emitter required to call app.emit() in shortcut handler callbacks

key-files:
  created:
    - src-tauri/capabilities/desktop.json
  modified:
    - src-tauri/src/lib.rs
    - src-tauri/Cargo.toml
    - src-tauri/Cargo.lock

key-decisions:
  - "Global shortcut plugin registered in setup() not at builder level — plan calls for app.handle().plugin() inside setup() with #[cfg(desktop)]"
  - "desktop.json capability targets windows: [settings] not [main] — settings is the only window in this app"
  - "on_shortcut() exists in tauri-plugin-global-shortcut 2.3.1 — plan research note confirmed it is available"
  - "use tauri::Emitter must be imported for app.emit() in shortcut handler closures — same requirement as established in 01-01"

patterns-established:
  - "Pattern: Plugin in setup() — global-shortcut registered via app.handle().plugin() inside setup(), not in builder chain"
  - "Pattern: rebind_hotkey — Tauri command takes old/new strings, calls gs.unregister(old) then gs.on_shortcut(new, handler)"

requirements-completed: [CORE-01]

# Metrics
duration: 5min
completed: 2026-02-27
---

# Phase 1 Plan 02: Global Hotkey Summary

**System-wide Ctrl+Shift+Space hotkey via tauri-plugin-global-shortcut 2.3.1, emitting hotkey-triggered event without stealing focus, with rebind_hotkey command for Plan 01-03**

## Performance

- **Duration:** 5 min
- **Started:** 2026-02-27T15:49:18Z
- **Completed:** 2026-02-27T15:54:49Z
- **Tasks:** 1
- **Files modified:** 4

## Accomplishments

- tauri-plugin-global-shortcut 2.3.1 added to Cargo.toml under desktop target
- Plugin registered in setup() with `#[cfg(desktop)]` guard via `app.handle().plugin()`
- Ctrl+Shift+Space registered with handler that prints "Hotkey triggered" and emits "hotkey-triggered" event
- `rebind_hotkey` Tauri command added — unregisters old combo, registers new via `on_shortcut()`
- Desktop capability file created with correct global-shortcut permissions
- Build succeeds with `cargo build` in dev profile

## Task Commits

Each task was committed atomically:

1. **Task 1: Add global-shortcut plugin and register default hotkey** - `e12b13d` (feat)

## Files Created/Modified

- `src-tauri/src/lib.rs` - Added `use tauri::Emitter`, `rebind_hotkey` command, global-shortcut plugin registration in setup() with handler
- `src-tauri/Cargo.toml` - Added `tauri-plugin-global-shortcut = "2"` under `[target.'cfg(not(android/ios))'.dependencies]`
- `src-tauri/Cargo.lock` - Updated with global-shortcut 2.3.1 and transitive deps
- `src-tauri/capabilities/desktop.json` - Created with global-shortcut permissions targeting settings window

## Decisions Made

- Plugin registered inside `setup()` via `app.handle().plugin()` with `#[cfg(desktop)]`, matching the plan's exact pattern
- `desktop.json` targets `windows: ["settings"]` (the only window in this app) — the CLI auto-generated `windows: ["main"]` which was wrong and corrected
- `on_shortcut()` confirmed available in the plugin — plan's note to verify was accurate; it maps directly to `register_internal()` with a per-shortcut handler

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Added missing `use tauri::Emitter` import**
- **Found during:** Task 1 (cargo build)
- **Issue:** `app.emit()` not found — `Emitter` trait not in scope. Same issue as 01-01 but now in handler closures too.
- **Fix:** Added `use tauri::{Emitter, Manager}` at top of lib.rs
- **Files modified:** src-tauri/src/lib.rs
- **Verification:** Compiler error resolved; `cargo build` succeeded
- **Committed in:** e12b13d (Task 1 commit)

**2. [Rule 1 - Bug] Fixed desktop.json windows list — changed "main" to "settings"**
- **Found during:** Task 1 (capability file review after `cargo tauri add`)
- **Issue:** CLI auto-generated `"windows": ["main"]` but this app has no "main" window — only "settings"
- **Fix:** Updated desktop.json to `"windows": ["settings"]`
- **Files modified:** src-tauri/capabilities/desktop.json
- **Verification:** Correct window label matches tauri.conf.json
- **Committed in:** e12b13d (Task 1 commit)

**3. [Rule 1 - Bug] Moved global-shortcut plugin from builder chain to setup() — CLI placed it first**
- **Found during:** Task 1 (reviewing lib.rs after `cargo tauri add`)
- **Issue:** CLI inserted `.plugin(tauri_plugin_global_shortcut::Builder::new().build())` at the top of the builder chain without the `#[cfg(desktop)]` guard and before single-instance
- **Fix:** Removed from builder chain; re-implemented in setup() via `app.handle().plugin()` with `#[cfg(desktop)]`, `with_shortcuts()`, and `with_handler()` as plan specified
- **Files modified:** src-tauri/src/lib.rs
- **Verification:** Plugin registered in correct location; single-instance remains first in builder chain
- **Committed in:** e12b13d (Task 1 commit)

---

**Total deviations:** 3 auto-fixed (2 bugs, 1 incorrect auto-generation from CLI)
**Impact on plan:** All fixes necessary for correctness. No scope creep.

## Issues Encountered

- `cargo tauri add global-shortcut` auto-generated lib.rs insertion was incorrect in three ways: wrong position (builder chain vs setup), no `#[cfg(desktop)]` guard, and no handler — required complete replacement of auto-generated code with plan-specified pattern
- `cargo tauri` not available as a cargo subcommand (no cargo-tauri.exe in .cargo/bin) — used `node_modules/.bin/tauri` instead

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Global hotkey is registered and compiled; `rebind_hotkey` command is ready for Plan 01-03 to wire to the settings UI
- Runtime behavior (focus not stolen, event reaches frontend) cannot be verified via automated build — requires manual `cargo tauri dev` test per plan verification steps
- Plan 01-03 (settings UI) can proceed: `rebind_hotkey` command exists and is registered in the invoke handler

---
*Phase: 01-foundation*
*Completed: 2026-02-27*

## Self-Check: PASSED

- src-tauri/src/lib.rs: FOUND
- src-tauri/Cargo.toml: FOUND
- src-tauri/capabilities/desktop.json: FOUND
- .planning/phases/01-foundation/01-02-SUMMARY.md: FOUND
- Commit e12b13d: FOUND
