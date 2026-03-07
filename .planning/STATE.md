---
gsd_state_version: 1.0
milestone: v1.4
milestone_name: Per-App Settings
status: executing
stopped_at: Completed 25-01-PLAN.md
last_updated: "2026-03-07T18:13:41Z"
last_activity: 2026-03-07 -- Completed 25-01 App Rules UI
progress:
  total_phases: 4
  completed_phases: 3
  total_plans: 4
  completed_plans: 4
  percent: 100
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-07)

**Core value:** Voice dictation must feel instant -- sub-1500ms from end-of-speech to text appearing at the cursor, with zero internet dependency.
**Current focus:** Phase 25 - App Rules UI

## Current Position

Phase: 25 (3 of 4 in v1.4)
Plan: 1 of 1 in current phase (COMPLETE)
Status: Phase 25 Complete
Last activity: 2026-03-07 -- Completed 25-01 App Rules UI

Progress: [██████████] 100%

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- [v1.4]: Detection at pipeline.rs line 395 (before ALL CAPS application), not in inject.rs
- [v1.4]: AppOverrides as separate managed state from ActiveProfile (read-only resolution)
- [v1.4]: PROCESS_QUERY_LIMITED_INFORMATION for elevated process safety
- [v1.4]: Three-state toggle via Option<bool> (None=inherit, Some(true)=ON, Some(false)=OFF)
- [v1.4]: Case-normalize exe names at every boundary
- [v1.4]: CreateToolhelp32Snapshot for process enumeration (Win32_System_Diagnostics_ToolHelp feature flag)
- [23-01]: EnumChildWindows in windows 0.58 takes HWND directly, not Option<HWND>
- [23-01]: #![allow(dead_code)] on foreground.rs until pipeline integration in 23-02
- [23-02]: Added #[cfg(windows)] to command function definitions (not just invoke_handler entries)
- [23-02]: Removed #![allow(dead_code)] from foreground.rs (module now integrated)
- [24-01]: Override resolution as pure function for unit testability without Win32 dependencies
- [24-01]: Lock ordering: ActiveProfile dropped before AppRulesState acquired to prevent deadlocks
- [25-01]: Custom dropdown (not native select) for color-coded three-state ALL CAPS control
- [25-01]: Inline button state machine for detect flow (no modal/toast)

### Pending Todos

None.

### Blockers/Concerns

- Do NOT remove clipboard verification retry loop or 150ms pre-paste delay -- they serve orthogonal purposes
- UWP EnumChildWindows callback pattern verified in 23-01 (takes HWND directly, not Option<HWND>)
- Three-state toggle UX resolved in 25-01: custom dropdown with Inherit/Force ON/Force OFF

## Session Continuity

Last session: 2026-03-07T18:13:41Z
Stopped at: Completed 25-01-PLAN.md
Resume file: .planning/phases/25-app-rules-ui/25-01-SUMMARY.md
