---
phase: 04-pill-overlay
plan: 01
subsystem: ui
tags: [tauri, react, multi-window, overlay, transparency, drag, vite]

# Dependency graph
requires:
  - phase: 03-core-pipeline
    provides: PipelineState AtomicU8 state machine that drives pill show/hide events in Plan 04-02
provides:
  - Frameless transparent always-on-top pill window (Tauri second window)
  - No-focus-steal guarantee verified against Notepad, VS Code, Chrome
  - Draggable pill with position persistence via tauri-plugin-store
  - Multi-page Vite build supporting pill.html and index.html entries
  - React shell (Pill.tsx) ready for visualizer + state display in Plan 04-02
affects:
  - 04-02 (fills pill with FrequencyBars + pipeline state display)
  - Any phase adding additional overlay windows (multi-window pattern established)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Multi-page Vite build with rollupOptions.input for two HTML entry points
    - set_focusable(false) in Rust setup() for WS_EX_NOACTIVATE no-focus-steal on Windows
    - Toggle focusable true/false around startDragging() to allow drag on unfocusable window
    - std::fs + serde_json for sync position restore in setup() (same pattern as read_saved_hotkey)
    - tauri-plugin-store JS API for async position persistence on mouseup

key-files:
  created:
    - pill.html
    - src/pill-main.tsx
    - src/Pill.tsx
    - src/pill.css
  modified:
    - src-tauri/tauri.conf.json
    - src-tauri/capabilities/default.json
    - src-tauri/src/lib.rs
    - vite.config.ts

key-decisions:
  - "set_focusable(false) blocks startDragging on Windows — must toggle focusable(true) before startDragging(), restore focusable(false) after drop via mouseup"
  - "core:window:allow-set-focusable and core:window:allow-start-dragging must be added to capabilities/default.json for pill window — not documented in plan"
  - "dist/ must be pre-built before npx tauri dev — pill.html uses no devUrl so Tauri serves from dist/; run npx vite build first"
  - "data-tauri-drag-region does not work on unfocusable windows — use startDragging() API exclusively"

patterns-established:
  - "Multi-window pattern: each window gets its own HTML entry, React mount, CSS file, and entry in tauri.conf.json windows array"
  - "No-focus-steal pattern: set_focusable(false) in Rust setup() + toggle around startDragging() in JS"
  - "Position persistence pattern: sync restore via std::fs in Rust setup(), async save via tauri-plugin-store JS on drag end"

requirements-completed: [UI-01, UI-02]

# Metrics
duration: ~2h
completed: 2026-02-28
---

# Phase 4 Plan 01: Pill Overlay Window Infrastructure Summary

**Frameless transparent always-on-top pill window with no-focus-steal guarantee, drag-to-reposition, and position persistence via multi-page Vite + Tauri second window**

## Performance

- **Duration:** ~2h
- **Started:** 2026-02-28
- **Completed:** 2026-02-28
- **Tasks:** 2 (1 auto + 1 human-verify checkpoint)
- **Files modified:** 8

## Accomplishments

- Pill overlay window created as a second Tauri window — frameless, transparent, always-on-top, never stealing focus
- No-focus-steal verified against Notepad, VS Code, and Chrome; pill can be shown/hidden/dragged without losing focus from active app
- Drag-to-reposition working with position persisted to settings.json across app restarts
- Multi-page Vite build established as pattern for future windows

## Task Commits

Each task was committed atomically:

1. **Task 1: Create pill window infrastructure** - `2b5440e` (feat)
2. **Task 2: Fix focusable toggle + capability permissions** - `440dc1f` (fix)

## Files Created/Modified

- `src-tauri/tauri.conf.json` - Added pill window definition (transparent, decorations false, alwaysOnTop, visible false, skipTaskbar)
- `src-tauri/capabilities/default.json` - Added "pill" to windows array; added core:window:allow-set-focusable and core:window:allow-start-dragging
- `src-tauri/src/lib.rs` - Pill post-creation setup: set_focusable(false) + sync position restore from settings.json
- `vite.config.ts` - Multi-page rollupOptions.input with index.html and pill.html entries
- `pill.html` - Pill window HTML entry point with transparent body background
- `src/pill-main.tsx` - React mount for pill window targeting #pill-root
- `src/Pill.tsx` - Pill component with drag handling, position persistence, pill-show/pill-hide event listeners
- `src/pill.css` - Tailwind import + transparent html/body/#pill-root styles

## Decisions Made

- **startDragging() not data-tauri-drag-region:** data-tauri-drag-region breaks on unfocusable windows; startDragging() API is required
- **Focusable toggle around drag:** set_focusable(false) prevents startDragging from working — must temporarily set focusable(true), call startDragging(), restore focusable(false) on mouseup
- **Capabilities must explicitly allow set-focusable and start-dragging:** These are not granted by core:default — both must be added as explicit permissions in default.json
- **Dev mode requires pre-built dist/:** No devUrl configured for pill window; Tauri serves pill.html from dist/ — must run `npx vite build` before `npx tauri dev`

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] set_focusable(false) blocked startDragging**
- **Found during:** Task 2 (human-verify checkpoint — drag behavior verification)
- **Issue:** Calling startDragging() on a window where set_focusable(false) has been applied silently fails on Windows — the drag never initiates
- **Fix:** Toggle set_focusable(true) in handleMouseDown before startDragging(), restore set_focusable(false) in handleMouseUp after position is saved
- **Files modified:** src/Pill.tsx, src-tauri/capabilities/default.json (added allow-set-focusable + allow-start-dragging)
- **Verification:** Drag verified working against Notepad, VS Code, Chrome without any focus steal
- **Committed in:** 440dc1f

---

**Total deviations:** 1 auto-fixed (1 bug — focusable/drag interaction on Windows)
**Impact on plan:** Fix was necessary for drag to function at all. No scope creep — capability additions required for the fix.

## Issues Encountered

- **Capability permissions missing from plan:** core:window:allow-set-focusable and core:window:allow-start-dragging were not listed in the plan's capability changes. Both are required — set-focusable for the Rust API call, start-dragging for the JS API call. Added to default.json as part of the fix commit.
- **Dev mode constraint:** pill.html has no devUrl — the pill window loads from dist/ even in dev mode. Developers must run `npx vite build` before `npx tauri dev` for the pill to render correctly.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Pill overlay container ready for Plan 04-02 to fill with FrequencyBars audio visualizer and pipeline state display
- pill-show / pill-hide event listeners wired and waiting — Plan 04-02 needs to emit these from the backend at recording start/stop
- No blockers

---
*Phase: 04-pill-overlay*
*Completed: 2026-02-28*
