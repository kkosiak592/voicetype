---
phase: 06-vocabulary-settings
plan: 03
subsystem: ui
tags: [react, tauri, typescript, tailwind, settings, sidebar, profiles, dictionary, microphone, model]

# Dependency graph
requires:
  - phase: 06-vocabulary-settings plan 01
    provides: five Tauri commands for profiles/corrections (get_profiles, set_active_profile, get_corrections, save_corrections, set_all_caps)
  - phase: 06-vocabulary-settings plan 02
    provides: four Tauri commands for mic/model (list_input_devices, set_microphone, list_models, set_model)
provides:
  - settings panel with sidebar navigation (General, Profiles, Model, Microphone, Appearance)
  - ProfileSwitcher.tsx: radio-card component for two built-in profiles
  - DictionaryEditor.tsx: inline From/To table with add/delete and auto-save on blur
  - ModelSelector.tsx: vertical model list with recommended badge and download status
  - ProfilesSection.tsx: profile switching, ALL CAPS toggle, corrections dictionary wired to Tauri commands
  - ModelSection.tsx: model selection with runtime reload wired to Tauri commands
  - MicrophoneSection.tsx: device dropdown wired to list_input_devices and set_microphone
  - App.tsx: sidebar-nav layout shell with no-scroll content pane
  - store.ts extended with activeProfile, selectedMic, selectedModel fields
affects:
  - 06.1-fix-duplicate-tray-icons: settings window dimensions now 720x500

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Sidebar-nav layout: flex h-screen sidebar (w-44) + flex-1 overflow-hidden content pane"
    - "Radio-card pattern: border-2 rounded-lg, indigo-500/indigo-50 selected, gray-200/white unselected"
    - "DictionaryEditor: Record<string,string> -> Row[] conversion on mount, rowsToRecord on blur triggers save"
    - "Section isolation: each section component owns its own useEffect + state + invoke calls"
    - "Loading skeletons: animate-pulse rounded-lg bg-gray-100 while async data loads"

key-files:
  created:
    - src/components/Sidebar.tsx
    - src/components/ProfileSwitcher.tsx
    - src/components/DictionaryEditor.tsx
    - src/components/ModelSelector.tsx
    - src/components/sections/GeneralSection.tsx
    - src/components/sections/AppearanceSection.tsx
    - src/components/sections/ProfilesSection.tsx
    - src/components/sections/ModelSection.tsx
    - src/components/sections/MicrophoneSection.tsx
  modified:
    - src/App.tsx
    - src/lib/store.ts
    - src-tauri/tauri.conf.json

key-decisions:
  - "Stub approach: Task 1 created stubs for Task 2 sections to allow build verification after Task 1 before full implementation"
  - "Profile descriptions hard-coded in ProfileSwitcher PROFILE_DESCRIPTIONS map — ProfileInfo only carries id/name/active from backend"
  - "DictionaryEditor saves on blur (not on each keystroke) — prevents excessive invoke calls while typing"
  - "ModelSelector tracks loadingId locally — allows per-card loading indicator without parent state change"
  - "MicrophoneSection shows 'System Default' first option from list_input_devices response (backend includes it)"

patterns-established:
  - "Section component pattern: each section owns its own data fetching (useEffect + invoke) and local state"
  - "Store sync pattern: invoke Tauri command first, then persist to store.ts for UI consistency"
  - "Sidebar active state: indigo-50 bg + indigo-600 text (light), indigo-950 bg + indigo-400 text (dark)"

requirements-completed: [SET-01]

# Metrics
duration: 6min
completed: 2026-03-01
---

# Phase 6 Plan 03: Settings UI Rebuild Summary

**Sidebar-navigated settings panel with ProfileSwitcher, DictionaryEditor, ModelSelector, and MicrophoneSection components wiring all Phase 6 backend Tauri commands into the React frontend**

## Performance

- **Duration:** 6 min
- **Started:** 2026-03-01T03:05:33Z
- **Completed:** 2026-03-01T03:11:30Z
- **Tasks:** 2
- **Files modified:** 12

## Accomplishments
- Rebuilt App.tsx from flat scrollable column to flex sidebar + content pane layout (720x500, no scroll)
- Created Sidebar.tsx with five nav items and indigo active state matching established design patterns
- Created ProfileSwitcher.tsx matching RecordingModeToggle radio-card pattern exactly
- Created DictionaryEditor.tsx with inline From/To two-column table, add/delete rows, and auto-save on blur
- Created ModelSelector.tsx with per-card loading indicator, recommended badge, and download status
- Created ProfilesSection, ModelSection, MicrophoneSection fully wired to Plan 01 and Plan 02 Tauri commands
- Extended store.ts with activeProfile, selectedMic, selectedModel fields and DEFAULTS

## Task Commits

Each task was committed atomically:

1. **Task 1: Update window config, store, sidebar layout, and section shell components** - `70ca534` (feat)
2. **Task 2: Create ProfileSwitcher, DictionaryEditor, ProfilesSection, ModelSelector, ModelSection, and MicrophoneSection** - `1d7b85e` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `src-tauri/tauri.conf.json` - Settings window changed from 480x400 to 720x500
- `src/lib/store.ts` - AppSettings extended with activeProfile, selectedMic, selectedModel fields
- `src/App.tsx` - Rebuilt as flex sidebar + content pane, five section conditionals, new state fields
- `src/components/Sidebar.tsx` - Nav sidebar with five items, indigo active state, exported SectionId type
- `src/components/ProfileSwitcher.tsx` - Radio-card profile selection calling set_active_profile
- `src/components/DictionaryEditor.tsx` - From/To table with add/delete and auto-save on blur calling onChange
- `src/components/ModelSelector.tsx` - Vertical model list with recommended badge, disabled state for not-downloaded
- `src/components/sections/GeneralSection.tsx` - Hotkey and recording mode extracted from old App.tsx
- `src/components/sections/AppearanceSection.tsx` - Theme and autostart extracted from old App.tsx
- `src/components/sections/ProfilesSection.tsx` - Profile switcher + ALL CAPS toggle + corrections dictionary
- `src/components/sections/ModelSection.tsx` - list_models + set_model wired, store persistence
- `src/components/sections/MicrophoneSection.tsx` - list_input_devices + set_microphone wired, store persistence

## Decisions Made
- Stub approach for Task 1/2 split: Task 1 created minimal stubs for sections not yet implemented so that build verification could confirm App.tsx compiled correctly before Task 2 replaced them with full implementations
- Profile descriptions hard-coded in ProfileSwitcher via PROFILE_DESCRIPTIONS map — backend ProfileInfo only carries id/name/active, not display descriptions
- DictionaryEditor saves on blur rather than on every keystroke to prevent excessive invoke('save_corrections') calls
- ModelSelector tracks loadingId locally (not in parent) — each card shows its own loading state independently
- Section components own their own useEffect data fetching rather than having App.tsx coordinate all data loading — simpler, section-isolated

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None — build passed on first attempt after both tasks.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Full settings UI functional: sidebar navigation, profiles, corrections dictionary, model selection, microphone selection
- All Phase 6 backend commands (9 total across Plans 01 and 02) now wired to UI
- Ready for Phase 06.1: Fix duplicate tray icons and replace square icon with proper app icon

## Self-Check: PASSED

All 9 created files found on disk. Both task commits (70ca534, 1d7b85e) verified in git log.

---
*Phase: 06-vocabulary-settings*
*Completed: 2026-03-01*
