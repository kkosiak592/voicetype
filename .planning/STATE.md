---
gsd_state_version: 1.0
milestone: v1.4
milestone_name: Per-App Settings
status: ready_to_plan
stopped_at: Roadmap created for v1.4
last_updated: "2026-03-07"
last_activity: 2026-03-07 -- Roadmap created for v1.4 Per-App Settings
progress:
  total_phases: 4
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-07)

**Core value:** Voice dictation must feel instant -- sub-1500ms from end-of-speech to text appearing at the cursor, with zero internet dependency.
**Current focus:** Phase 23 - Foreground Detection Backend

## Current Position

Phase: 23 (1 of 4 in v1.4)
Plan: 0 of ? in current phase
Status: Ready to plan
Last activity: 2026-03-07 -- Roadmap created for v1.4 Per-App Settings

Progress: [░░░░░░░░░░] 0%

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

### Pending Todos

None.

### Blockers/Concerns

- Do NOT remove clipboard verification retry loop or 150ms pre-paste delay -- they serve orthogonal purposes
- UWP EnumChildWindows callback pattern in windows crate needs verification during Phase 23 planning
- Three-state toggle UX (cycling vs segmented control) needs decision during Phase 25 planning

## Session Continuity

Last session: 2026-03-07
Stopped at: Roadmap created for v1.4
Resume file: None
