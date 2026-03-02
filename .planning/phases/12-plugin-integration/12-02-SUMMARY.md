---
phase: 12-plugin-integration
plan: "02"
subsystem: ui
tags: [tauri, updater, react, typescript, tray, plugin-updater, plugin-process]

# Dependency graph
requires:
  - phase: 12-01
    provides: tauri-plugin-updater and tauri-plugin-process registered in Rust backend, check_for_update IPC command, capabilities/permissions configured
provides:
  - useUpdater React hook with full update lifecycle (check, download via JS plugin API, cancel, restart, dismiss, periodic 4h checks)
  - UpdateBanner component with states: checking, available, downloading (progress), ready, error
  - GeneralSection version display (getVersion from @tauri-apps/api/app) and Check for Updates button
  - Tray menu update indicator (Update Available item above Settings)
  - set_update_available and is_pipeline_active Tauri commands
affects: [13-ci-cd, future-release-workflow]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "useUpdater hook stores Update object in ref across re-renders for downloadAndInstall call"
    - "Tray menu rebuilt via set_menu() on update indicator change (Tauri 2 menus are immutable after creation)"
    - "JS plugin API for download — download continues after component unmount but ref is lost; banner resets to available on reopen (known limitation)"
    - "is_pipeline_active polls LevelStreamActive AtomicBool to defer relaunch until dictation is idle"

key-files:
  created:
    - src/lib/updater.ts
    - src/components/UpdateBanner.tsx
  modified:
    - src/App.tsx
    - src/components/sections/GeneralSection.tsx
    - src-tauri/src/tray.rs
    - src-tauri/src/lib.rs
    - src-tauri/src/updater.rs
    - src-tauri/tauri.conf.json

key-decisions:
  - "JS plugin API handles download/install (not Rust IPC) — enables progress callbacks and simpler frontend control"
  - "cancelDownload sets a cancelled ref flag; actual download continues (plugin has no cancellation API) — UI just stops showing progress"
  - "restartLater is valid: downloadAndInstall already wrote update to disk; next app launch applies it automatically"
  - "Updater config belongs under top-level plugins key in tauri.conf.json (Tauri v2 format), not under app section"
  - "Mid-download close limitation acknowledged: banner resets to available state on reopen since JS Update ref is destroyed on unmount"

patterns-established:
  - "UpdateBanner: conditional rendering with null return for idle/dismissed — no DOM impact when not needed"
  - "tray::set_tray_update_indicator: creates new Menu and calls set_menu() to update tray dynamically"

requirements-completed: [UPD-03, UPD-04, UPD-05, UPD-06]

# Metrics
duration: ~45min
completed: 2026-03-02
---

# Phase 12 Plan 02: Plugin Integration — Update UI Summary

**UpdateBanner component with download progress and restart flow, useUpdater hook with 4-hour periodic checks, tray indicator, and GeneralSection version display all wired via @tauri-apps/plugin-updater JS API**

## Performance

- **Duration:** ~45 min
- **Started:** 2026-03-02T19:43:43Z
- **Completed:** 2026-03-02T20:30:00Z
- **Tasks:** 3 (including human-verify checkpoint)
- **Files modified:** 8

## Accomplishments

- Full update UI lifecycle implemented: checking banner, available notification with release notes, download progress bar, restart/later controls, error with retry
- useUpdater hook manages Update object ref, periodic 4-hour checks, 4-second launch delay, tray indicator invocation, and pipeline-active guard before relaunch
- Tray menu dynamically gains "Update Available" item when update is found (rebuilt via set_menu() since Tauri 2 menus are immutable)
- GeneralSection shows live version from @tauri-apps/api/app getVersion() and a Check for Updates button with status-aware label
- Post-checkpoint fixes applied: updater config moved to Tauri v2 plugins key, unused Manager import removed

## Task Commits

Each task was committed atomically:

1. **Task 1: Create updater state module and UpdateBanner component** - `9198c7c` (feat)
2. **Task 2: Integrate updater into App.tsx, GeneralSection, and tray menu** - `f79def4` (feat)
3. **Task 3: Human verification checkpoint — approved** - checkpoint (no code commit)
4. **Post-checkpoint fixes: tauri.conf.json plugin config + updater.rs import cleanup** - `ee29ac2` (fix)

## Files Created/Modified

- `src/lib/updater.ts` - useUpdater hook: check/download/cancel/restart/dismiss, periodic timer, tray invoke
- `src/components/UpdateBanner.tsx` - Status-driven banner with progress bar, action buttons, dark mode
- `src/App.tsx` - UpdateBanner integration above main content, updater props passed to GeneralSection
- `src/components/sections/GeneralSection.tsx` - Version display via getVersion(), Check for Updates button with status labels
- `src-tauri/src/tray.rs` - set_tray_update_indicator function, update_available menu event handler
- `src-tauri/src/lib.rs` - set_update_available and is_pipeline_active commands registered
- `src-tauri/src/updater.rs` - Removed unused use tauri::Manager import
- `src-tauri/tauri.conf.json` - Moved updater config from app section to top-level plugins (Tauri v2 format)

## Decisions Made

- Used JS plugin API (check().downloadAndInstall()) rather than a Rust download command — enables progress callbacks in the frontend without custom channel piping
- cancelDownload works by setting a cancelled ref and ignoring the resolved promise; the underlying download continues (plugin has no cancellation API)
- restartLater does not require additional code — downloadAndInstall already writes the update to disk; the Tauri updater applies it on next launch automatically
- Tray menus in Tauri 2 must be rebuilt from scratch on each change; set_tray_update_indicator creates a new Menu with/without the update item and calls set_menu()

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Moved updater config from Tauri v1 app section to Tauri v2 plugins section**
- **Found during:** Post-checkpoint (orchestrator fix)
- **Issue:** tauri.conf.json had `updater` nested under `app.security` (Tauri v1 format); Tauri 2 expects it under top-level `plugins` key
- **Fix:** Moved the updater block (pubkey + endpoints) to `plugins.updater`; removed deprecated `active: true` field
- **Files modified:** src-tauri/tauri.conf.json
- **Verification:** Config structure matches Tauri 2 plugin configuration format
- **Committed in:** ee29ac2 (post-checkpoint fix)

**2. [Rule 1 - Bug] Removed unused use tauri::Manager import from updater.rs**
- **Found during:** Post-checkpoint (orchestrator fix)
- **Issue:** Import was unused after Task 2 implementation; Rust compiler would emit dead-code warning
- **Fix:** Removed the import line
- **Files modified:** src-tauri/src/updater.rs
- **Verification:** File compiles without unused import warning
- **Committed in:** ee29ac2 (post-checkpoint fix)

---

**Total deviations:** 2 auto-fixed (2 bugs — both config/cleanup corrections)
**Impact on plan:** Both fixes necessary for correct Tauri 2 plugin configuration and clean compilation. No scope creep.

## Issues Encountered

- UpdateBanner mid-download state is lost when the settings window closes (JS Update ref is destroyed on unmount). On reopen, the banner resets to 'available' state. This is a known plugin API limitation explicitly acknowledged in the plan — no workaround without a persistent Rust download handle.

## User Setup Required

None - no external service configuration required for this plan. The updater endpoint in tauri.conf.json already points to the GitHub releases URL configured in Phase 11.

## Next Phase Readiness

- Update UI fully functional; will exercise the real update flow once Phase 13 (CI/CD) publishes the first GitHub Release with a newer version
- No blockers for Phase 13

---
*Phase: 12-plugin-integration*
*Completed: 2026-03-02*
