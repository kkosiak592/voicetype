---
phase: 06-vocabulary-settings
plan: 01
subsystem: api
tags: [rust, tauri, regex, whisper, corrections, profiles, vocabulary]

# Dependency graph
requires:
  - phase: 02-whisper-integration
    provides: transcribe_audio function and WhisperContext that now accepts initial_prompt
  - phase: 03-pipeline-integration
    provides: pipeline.rs text formatting step where corrections are inserted
provides:
  - corrections.rs: CorrectionsEngine with HashMap-backed regex word-boundary matching
  - profiles.rs: Profile struct with structural_engineering and general built-in profiles
  - Pipeline corrections + ALL CAPS applied after transcription, before inject
  - Whisper initial_prompt per profile with correct set_no_context toggle
  - Five Tauri commands: get_profiles, set_active_profile, get_corrections, save_corrections, set_all_caps
affects:
  - 06-02: settings UI that calls the five new Tauri commands

# Tech tracking
tech-stack:
  added: [regex = "1"]
  patterns:
    - "Corrections engine: HashMap<String,String> -> Vec<(Regex, String)> compiled rules, apply() iterates sequentially"
    - "Profile state: Mutex<Profile> in ActiveProfile managed state, swapped atomically on profile switch"
    - "Settings persistence: flat JSON keys corrections.{profile_id} and profiles.{profile_id}.all_caps"

key-files:
  created:
    - src-tauri/src/corrections.rs
    - src-tauri/src/profiles.rs
    - src-tauri/src/corrections_tests.rs
  modified:
    - src-tauri/Cargo.toml
    - src-tauri/src/lib.rs
    - src-tauri/src/pipeline.rs
    - src-tauri/src/transcribe.rs

key-decisions:
  - "regex crate used for word-boundary matching (?i)\\b{escaped_from}\\b — regex::escape prevents metachar injection"
  - "set_no_context(false) when initial_prompt non-empty — CRITICAL: no_context=true suppresses the prompt in whisper"
  - "Corrections applied sequentially (Vec<Rule>) not in parallel — acceptable for v1 dictionary size"
  - "User corrections stored at settings.json key corrections.{profile_id} and merged at startup/profile-switch"
  - "ProfileInfo (id, name, is_active) returned by get_profiles — corrections dict excluded for efficiency"

patterns-established:
  - "Managed state swap: lock guard, replace inner value, unlock — used for both ActiveProfile and CorrectionsState"
  - "Corrections pipeline position: after trim_start, before format!(trailing space) — matches CONTEXT.md locked decision"
  - "initial_prompt read before spawn_blocking — AppHandle is not Send so state must be cloned before move"

requirements-completed: [VOC-01, VOC-02, VOC-03, VOC-04, VOC-05, VOC-06]

# Metrics
duration: 6min
completed: 2026-02-28
---

# Phase 6 Plan 01: Vocabulary Settings Backend Summary

**HashMap-backed regex corrections engine and vocabulary profile system in Rust with Whisper initial_prompt bias, ALL CAPS flag, and five Tauri commands for profile/corrections management**

## Performance

- **Duration:** 6 min
- **Started:** 2026-03-01T02:47:55Z
- **Completed:** 2026-03-01T02:53:55Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments
- Created corrections.rs: CorrectionsEngine applies whole-word case-insensitive replacements using `(?i)\b...\b` regex patterns compiled from HashMap at construction time
- Created profiles.rs: Profile struct with two hard-coded profiles — Structural Engineering (13 corrections, engineering initial_prompt) and General (empty prompt, no corrections)
- Pipeline integration: corrections + ALL CAPS applied between trim_start and inject, whisper receives initial_prompt from active profile
- Whisper transcribe_audio updated with initial_prompt parameter and correct set_no_context toggle
- Five Tauri commands registered: get_profiles, set_active_profile, get_corrections, save_corrections, set_all_caps
- 8 behavior tests written and passing (TDD)

## Task Commits

Each task was committed atomically:

1. **Task 1: Create corrections engine and profile system modules** - `3fcffd7` (feat)
2. **Task 2: Integrate corrections and profiles into pipeline and transcription** - `58531c9` (feat)

**Plan metadata:** (docs commit follows)

_Note: Task 1 used TDD — test file created first (RED), modules created to pass tests (GREEN)_

## Files Created/Modified
- `src-tauri/src/corrections.rs` - CorrectionsEngine (from_map/apply) + CorrectionsState managed state wrapper
- `src-tauri/src/profiles.rs` - Profile struct, structural_engineering_profile, general_profile, get_all_profiles, ActiveProfile managed state
- `src-tauri/src/corrections_tests.rs` - 8 behavior tests covering word-boundary matching, multi-word phrases, profile fields, ALL CAPS
- `src-tauri/Cargo.toml` - Added regex = "1" dependency
- `src-tauri/src/lib.rs` - Module declarations, 5 new Tauri commands, setup() managed state registration, helper readers, updated transcribe call sites
- `src-tauri/src/pipeline.rs` - initial_prompt read, corrections apply, ALL CAPS, updated tray tooltip
- `src-tauri/src/transcribe.rs` - Added initial_prompt parameter, conditional set_no_context toggling

## Decisions Made
- Used `regex::escape` to safely embed user-provided correction keys in regex patterns — prevents metacharacter injection
- set_no_context(false) when initial_prompt is non-empty: whisper's no_context=true flag silently suppresses initial_prompt (discovered in RESEARCH.md Open Question 3)
- Sequential rule application (Vec<Rule>) is fine for v1 dictionary size — HashMap iteration order is non-deterministic but the v1 dictionary has no conflicting rules
- ProfileInfo struct excludes corrections dict from get_profiles response — UI only needs id/name/active for the profile selector

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None — cargo check and all 8 tests passed on first attempt.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Corrections engine and profiles backend fully wired and tested
- All five Tauri commands available for the settings UI to consume
- Profile switching, corrections editing, and ALL CAPS toggle all persist to settings.json
- Ready for Phase 6 Plan 02: Settings UI (vocabulary/profile sections)

---
*Phase: 06-vocabulary-settings*
*Completed: 2026-02-28*
