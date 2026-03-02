---
phase: 08-add-parakeet-tdt-model-and-optimize-transcription-latency
plan: 03
subsystem: ui
tags: [react, tauri, parakeet, whisper, engine-selector, model-download]

# Dependency graph
requires:
  - phase: 08-01
    provides: download_parakeet_model Tauri command with Channel progress events
  - phase: 08-02
    provides: get_engine/set_engine commands, TranscriptionEngine enum, list_models Parakeet entry

provides:
  - Three-card model selection in FirstRun.tsx (GPU-conditional, GPU = 3 cards, CPU = 1 card)
  - Fastest badge on Parakeet TDT card, Recommended badge on Large v3 Turbo
  - Parakeet download from FirstRun routes to download_parakeet_model command
  - Engine selector (Whisper/Parakeet TDT toggle) in ModelSection.tsx for GPU users
  - Parakeet download section in settings with progress bar
  - Auto-activate Parakeet engine after FirstRun Parakeet download

affects:
  - Future UI phases touching ModelSection, FirstRun, or engine state

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Channel<DownloadEvent> pattern reused in ModelSection for Parakeet download
    - GPU presence detection via model list membership (large-v3-turbo or parakeet-tdt-v2)

key-files:
  created: []
  modified:
    - src/components/FirstRun.tsx
    - src/components/sections/ModelSection.tsx

key-decisions:
  - "GPU presence in ModelSection detected by checking if list_models includes large-v3-turbo or parakeet-tdt-v2 (avoids new Tauri command)"
  - "whisperModels filtered before passing to ModelSelector — Parakeet entry excluded from Whisper model list"
  - "FirstRun max-w-xl widened to max-w-3xl to accommodate 3-column card grid"
  - "set_engine('parakeet') called automatically after Parakeet FirstRun download — engine activation is implicit on model choice"

patterns-established:
  - "GPU detection: check model list for GPU-only model IDs rather than dedicated command"
  - "Engine-agnostic section text: 'transcription model' not 'whisper model'"

requirements-completed: [PKT-01, PKT-02, PKT-03, PKT-04, PKT-05]

# Metrics
duration: 8min
completed: 2026-03-01
---

# Phase 08 Plan 03: Parakeet Frontend Integration Summary

**Three-card GPU model selection in FirstRun and Whisper/Parakeet engine toggle in ModelSection settings, with download support from both locations**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-03-01T17:39:24Z
- **Completed:** 2026-03-01T17:47:00Z
- **Tasks:** 1/2 (Task 2 is human verification checkpoint)
- **Files modified:** 2

## Accomplishments
- FirstRun shows 3 model cards for GPU users (Large v3 Turbo + Parakeet TDT + Small English) and 1 card for CPU users
- Parakeet TDT card has a green "Fastest" badge; Large v3 Turbo keeps indigo "Recommended" badge
- Parakeet download in FirstRun invokes `download_parakeet_model` (not `download_model`) and sets engine to 'parakeet' on completion
- ModelSection settings page shows engine selector for GPU users with Whisper (Accurate) and Parakeet TDT (Fast) toggle buttons
- Parakeet download section with progress bar in settings for users who haven't downloaded it yet
- Parakeet TDT entry excluded from Whisper ModelSelector — engines fully separated

## Task Commits

Each task was committed atomically:

1. **Task 1: Add Parakeet card to FirstRun.tsx and engine selector to ModelSection.tsx** - `9b430b8` (feat)
2. **Task 2: Human verification** - checkpoint (pending)

## Files Created/Modified
- `src/components/FirstRun.tsx` - Added parakeet-tdt-v2 card, gpuOnly filtering, Fastest badge, download routing, set_engine on complete, 3-column GPU grid
- `src/components/sections/ModelSection.tsx` - Added engine selector, Parakeet download section, GPU detection, whisperModels filtering, updated description text

## Decisions Made
- GPU presence in ModelSection detected by checking if `list_models` includes `large-v3-turbo` or `parakeet-tdt-v2` — avoids adding a new `check_first_run` call or separate GPU flag prop
- `whisperModels = models.filter(m => m.id !== 'parakeet-tdt-v2')` passed to ModelSelector — keeps Parakeet out of the Whisper model list
- `max-w-xl` widened to `max-w-3xl` in FirstRun to accommodate 3-column card grid without cramping
- `set_engine('parakeet')` called automatically after Parakeet FirstRun download — implicit engine activation matches user intent

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Removed unused formatMB function from ModelSection**
- **Found during:** Task 1 (TypeScript compilation)
- **Issue:** `formatMB` defined but not called — TS6133 error blocked compilation
- **Fix:** Removed the unused helper (Parakeet download section shows percentage only, not byte counts)
- **Files modified:** src/components/sections/ModelSection.tsx
- **Verification:** `npx tsc --noEmit` returned no errors
- **Committed in:** 9b430b8 (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - unused variable causing TS error)
**Impact on plan:** Trivial fix. No scope change.

## Issues Encountered
None — TypeScript compilation error caught and fixed immediately.

## Next Phase Readiness
- All backend and frontend Parakeet integration complete pending human verification (Task 2)
- Human verification covers: engine selector UI, Parakeet download from settings, engine switch without restart, transcription quality, corrections/ALL CAPS with Parakeet, engine persistence across restart, FirstRun 3-card layout

## Self-Check

- [x] `src/components/FirstRun.tsx` exists and contains `parakeet-tdt-v2`
- [x] `src/components/sections/ModelSection.tsx` exists and contains `set_engine`
- [x] Commit `9b430b8` exists

## Self-Check: PASSED

---
*Phase: 08-add-parakeet-tdt-model-and-optimize-transcription-latency*
*Completed: 2026-03-01*
