---
phase: 26-process-dropdown
plan: 01
subsystem: ui
tags: [tauri, react, win32, process-enumeration, dropdown]

requires:
  - phase: 25-app-rules-ui
    provides: "AppRulesSection component with detect flow and three-state toggle"
  - phase: 23-foreground-detect
    provides: "foreground.rs module with DetectedApp, AppRule, AppRulesState"
provides:
  - "list_running_processes Tauri command for enumerating visible-window processes"
  - "Browse Running Apps searchable dropdown in AppRulesSection"
affects: []

tech-stack:
  added: [Win32_System_Diagnostics_ToolHelp]
  patterns: [CreateToolhelp32Snapshot two-phase enumeration, fetch-once dropdown strategy]

key-files:
  created: []
  modified:
    - src-tauri/Cargo.toml
    - src-tauri/src/foreground.rs
    - src-tauri/src/lib.rs
    - src/components/sections/AppRulesSection.tsx

key-decisions:
  - "CreateToolhelp32Snapshot with EnumWindows two-phase enumeration for process listing"
  - "Fetch-once strategy: process list fetched on button click, no auto-refresh"
  - "Secondary/outline button styling for Browse vs primary Detect button"

patterns-established:
  - "Two-phase process enumeration: EnumWindows for visible windows, then snapshot for exe names"
  - "Fetch-once dropdown: load data on open, filter client-side"

requirements-completed: [UI-04]

duration: 12min
completed: 2026-03-07
---

# Phase 26 Plan 01: Process Dropdown Summary

**Browse Running Apps searchable dropdown with Win32 process enumeration via CreateToolhelp32Snapshot and EnumWindows**

## Performance

- **Duration:** 12 min
- **Started:** 2026-03-07T21:25:00Z
- **Completed:** 2026-03-07T21:37:58Z
- **Tasks:** 3
- **Files modified:** 4

## Accomplishments
- Win32 backend enumerating running processes with visible windows using CreateToolhelp32Snapshot + EnumWindows two-phase approach
- Searchable dropdown UI filtering by exe name and window title in real-time
- Already-added processes shown dimmed with "already added" label, non-clickable
- Outside-click dismiss and auto-focus search input

## Task Commits

Each task was committed atomically:

1. **Task 1: Backend -- list_running_processes command** - `86c30c0` (feat)
2. **Task 2: Frontend -- Browse Running Apps dropdown** - `e34b3c3` (feat)
3. **Task 3: Checkpoint -- human-verify** - approved (no commit)

## Files Created/Modified
- `src-tauri/Cargo.toml` - Added Win32_System_Diagnostics_ToolHelp feature flag
- `src-tauri/src/foreground.rs` - RunningProcess struct, list_running_processes function, enum_visible_windows callback
- `src-tauri/src/lib.rs` - list_running_processes Tauri command wrapper and invoke_handler registration
- `src/components/sections/AppRulesSection.tsx` - Browse Running Apps button, searchable dropdown panel, process list with filtering and already-added dimming

## Decisions Made
- CreateToolhelp32Snapshot with EnumWindows two-phase enumeration for process listing
- Fetch-once strategy: process list fetched on button click, no auto-refresh or polling
- Secondary/outline button styling for Browse to differentiate from primary Detect button

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Browse Running Apps feature complete and verified end-to-end
- Process dropdown provides alternative to Detect Active App for adding per-app rules

---
*Phase: 26-process-dropdown*
*Completed: 2026-03-07*
