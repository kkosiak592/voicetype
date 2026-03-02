---
phase: 12-plugin-integration
plan: "01"
subsystem: infra
tags: [tauri, updater, auto-update, github-releases, rust, plugin]

# Dependency graph
requires:
  - phase: 11-signing-repo-setup
    provides: Ed25519 pubkey and updater endpoint already set in tauri.conf.json
provides:
  - tauri-plugin-updater registered in Rust backend with check_for_update IPC command
  - tauri-plugin-process registered for app restart after update install
  - Frontend npm packages @tauri-apps/plugin-updater and @tauri-apps/plugin-process installed
  - updater:default and process:allow-restart/exit permissions in capabilities
affects:
  - 12-02-update-ui

# Tech tracking
tech-stack:
  added:
    - tauri-plugin-updater = "2" (Rust crate)
    - tauri-plugin-process = "2" (Rust crate)
    - "@tauri-apps/plugin-updater" (npm)
    - "@tauri-apps/plugin-process" (npm)
  patterns:
    - tauri-plugin-updater registered in setup() via app.handle().plugin() (needs app handle, same as global-shortcut)
    - tauri-plugin-process registered on Builder chain (no app handle needed)
    - Thin Rust check_for_update command returns UpdateInfo metadata; actual download/install done by JS API

key-files:
  created:
    - src-tauri/src/updater.rs
  modified:
    - src-tauri/Cargo.toml
    - src-tauri/src/lib.rs
    - src-tauri/capabilities/default.json
    - package.json
    - package-lock.json

key-decisions:
  - "tauri-plugin-updater registered in setup() not on Builder — requires app handle to read updater config from tauri.conf.json"
  - "Rust check_for_update command is check-only; download/install handled by JS plugin API (check().downloadAndInstall())"
  - "tauri.conf.json updater config (pubkey + endpoint) left untouched — already set in Phase 11"

patterns-established:
  - "Plugin registration split: setup()-requiring plugins go in setup(), stateless plugins on Builder chain"

requirements-completed: [UPD-02, UPD-07, REL-02]

# Metrics
duration: 10min
completed: 2026-03-02
---

# Phase 12 Plan 01: Plugin Integration — Updater Backend Summary

**tauri-plugin-updater and tauri-plugin-process wired into the Rust backend with check_for_update IPC command, capabilities permissions, and frontend npm packages installed**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-03-02T19:33:00Z
- **Completed:** 2026-03-02T19:43:43Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- Added tauri-plugin-updater and tauri-plugin-process Rust crates to Cargo.toml
- Installed @tauri-apps/plugin-updater and @tauri-apps/plugin-process npm packages
- Created updater.rs with check_for_update command that returns UpdateInfo (available, version, body)
- Registered tauri-plugin-process on Builder chain and tauri-plugin-updater inside setup() via app.handle().plugin()
- Added updater::check_for_update to invoke_handler
- Added updater:default, process:allow-restart, and process:allow-exit to capabilities/default.json

## Task Commits

Each task was committed atomically:

1. **Task 1: Add Rust and npm dependencies for updater and process plugins** - `cda2bc1` (feat)
2. **Task 2: Register plugins in Rust backend and create updater module** - `7a05e1d` (feat)

**Plan metadata:** (docs commit to follow)

## Files Created/Modified

- `src-tauri/src/updater.rs` - New module with check_for_update Tauri command returning UpdateInfo struct
- `src-tauri/Cargo.toml` - Added tauri-plugin-updater = "2" and tauri-plugin-process = "2"
- `src-tauri/src/lib.rs` - Added mod updater, tauri_plugin_process::init() on Builder, tauri_plugin_updater in setup(), updater::check_for_update in invoke_handler
- `src-tauri/capabilities/default.json` - Added updater:default, process:allow-restart, process:allow-exit permissions
- `package.json` - Added @tauri-apps/plugin-updater and @tauri-apps/plugin-process dependencies
- `package-lock.json` - Updated lockfile

## Decisions Made

- tauri-plugin-updater must be registered in setup() because it reads updater configuration (pubkey, endpoint) from the app handle, which is only available after app initialization — same pattern as tauri-plugin-global-shortcut.
- The Rust check_for_update command is intentionally check-only (returns version metadata). The actual download, progress tracking, and relaunch will be done by the JS plugin API in Plan 02 frontend code.
- tauri.conf.json was not modified — the updater endpoint and pubkey were already configured in Phase 11.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Backend updater foundation is complete. Plan 02 can now implement the frontend update notification UI and call check_for_update via IPC, then use the JS plugin API for download/install/relaunch.
- The app needs to be compiled (cargo build) to verify the Rust code compiles cleanly — this will happen as part of Plan 02 development.

---
*Phase: 12-plugin-integration*
*Completed: 2026-03-02*
