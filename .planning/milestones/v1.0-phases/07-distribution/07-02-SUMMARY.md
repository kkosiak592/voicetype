---
phase: 07-distribution
plan: 02
subsystem: ui
tags: [react, tauri, channel, download, first-run, model-selector, autostart]

# Dependency graph
requires:
  - phase: 07-distribution/07-01
    provides: check_first_run, download_model, enable_autostart Tauri commands; Channel<DownloadEvent> protocol; list_models with downloaded flag

provides:
  - FirstRun.tsx: guided first-run setup with GPU detection badge, two model cards, download progress, cancel, retry, enable_autostart on success
  - App.tsx first-run gate: invokes check_first_run on mount; renders FirstRun when needsSetup=true, dismisses to normal settings on completion
  - ModelSelector.tsx download capability: Download button + compact progress bar for non-downloaded models; Channel<DownloadEvent> drives progress
  - ModelSection.tsx refresh: re-invokes list_models after download and auto-selects newly downloaded model
affects: [07-03-installer]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Channel<DownloadEvent> driving React state in both FirstRun and ModelSelector — single pattern for progress reporting"
    - "cancelledRef ref guard pattern: cancel sets ref, Channel handler bails early — prevents stale events updating dismissed UI"
    - "useEffect watching downloadState === 'complete' for side-effecting enable_autostart + onComplete — clean separation from download event handler"

key-files:
  created:
    - src/components/FirstRun.tsx
  modified:
    - src/App.tsx
    - src/components/ModelSelector.tsx
    - src/components/sections/ModelSection.tsx

key-decisions:
  - "ModelSection auto-selects freshly downloaded model via handleModelSelect after list_models refresh — user doesn't need an extra click"
  - "FirstRun uses hardcoded MODELS array rather than list_models — self-contained flow with explicit size/quality/hardware labels not in ModelInfo"
  - "enable_autostart called in useEffect watching downloadState complete, not in the Channel handler — avoids async-in-message-handler complexity"
  - "cancelledRef pattern for cancel: Rust download_model continues in background (overwrites .tmp on retry), frontend ignores events after cancel"

patterns-established:
  - "Tauri Channel<T> in React: create Channel, set .onmessage, pass to invoke — both FirstRun and ModelSelector use this pattern identically"

requirements-completed: [DIST-01, DIST-02]

# Metrics
duration: 3min
completed: 2026-03-01
---

# Phase 07 Plan 02: Distribution Frontend Summary

**React first-run setup flow with GPU detection, two-model download cards using Tauri Channel<DownloadEvent> progress, autostart enablement, and download capability added to settings ModelSelector**

## Performance

- **Duration:** ~3 min
- **Started:** 2026-03-01T14:43:14Z
- **Completed:** 2026-03-01T14:46:11Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- Created FirstRun.tsx: full-page setup flow with GPU detection badge, two model cards (recommended badge on GPU-detected model), real-time download progress bar with cancel/retry, enable_autostart fire-and-forget on success, onComplete() after 1s delay
- Updated App.tsx: invokes check_first_run on mount alongside store loading; gates rendering on needsSetup flag; dismisses FirstRun by clearing needsSetup
- Updated ModelSelector.tsx: non-downloaded models show Download button; Channel<DownloadEvent> drives compact progress bar under card; inline retry on error; onDownloadComplete callback
- Updated ModelSection.tsx: passes onDownloadComplete; refreshes model list via list_models after download; auto-selects newly downloaded model; description updated

## Task Commits

Each task was committed atomically:

1. **Task 1: Create FirstRun.tsx** - `328e8c7` (feat)
2. **Task 2: Wire first-run gate in App.tsx and download in ModelSelector** - `ec2f7bd` (feat)

## Files Created/Modified

- `src/components/FirstRun.tsx` - Guided first-run setup: GPU badge, model cards, download state machine, autostart
- `src/App.tsx` - First-run gate: check_first_run on mount, FirstRun gate before settings, import FirstRun
- `src/components/ModelSelector.tsx` - Download button + Channel progress for non-downloaded models, onDownloadComplete prop
- `src/components/sections/ModelSection.tsx` - Wires onDownloadComplete, refreshes models, auto-selects downloaded model

## Decisions Made

- ModelSection auto-selects freshly downloaded model — avoids requiring user to click after download completes in settings
- FirstRun MODELS array is hardcoded (not from list_models) — needs size, quality, and hardware labels not present in ModelInfo
- enable_autostart fires in a useEffect watching downloadState='complete', not inside the Channel.onmessage handler — cleaner async pattern
- cancelledRef ref pattern allows cancel without needing to abort the Rust command; subsequent Channel events are ignored

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None — TypeScript compilation passed on first attempt for both tasks. Vite build succeeded for both entry points (settings + pill).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- FirstRun gate is live: fresh installs with no model file will see the guided setup flow
- ModelSelector in settings lets users download additional models after initial setup
- Both flows use the same Channel<DownloadEvent> pattern from 07-01 backend
- 07-03 (installer) can proceed; autostart is enabled by default via enable_autostart in FirstRun

---
*Phase: 07-distribution*
*Completed: 2026-03-01*
