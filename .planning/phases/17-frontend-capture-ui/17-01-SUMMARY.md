---
phase: 17-frontend-capture-ui
plan: 01
subsystem: ui
tags: [react, typescript, keyboard-capture, hotkey, modifier-only]

# Dependency graph
requires:
  - phase: 16-rebind-and-coexistence
    provides: rebind_hotkey IPC that routes modifier-only combos through the keyboard hook
  - phase: 15-hook-module
    provides: keyboard hook backend with is_modifier_only predicate
provides:
  - HotkeyCapture.tsx with dual keydown+keyup capture supporting modifier-only combos
  - modifierToken() helper for e.code-based modifier detection on keyup
  - MODIFIER_ORDER for deterministic ctrl/alt/shift/meta token ordering
  - Progressive held-modifier display during capture
affects: [18-release, testing]

# Tech tracking
tech-stack:
  added: []
  patterns: [dual-event keyboard capture pattern, read-before-delete on keyup held-set]

key-files:
  created: []
  modified:
    - src/components/HotkeyCapture.tsx

key-decisions:
  - "17-01: modifierToken uses e.code (not e.ctrlKey/e.metaKey flags) on keyup — flags already false for released key"
  - "17-01: read heldRef tokens BEFORE deleting on keyup — pre-delete state is the combo"
  - "17-01: MODIFIER_ORDER sorts tokens deterministically regardless of press order (ctrl < alt < shift < meta)"
  - "17-01: heldRef cleared on all cancel paths (Escape, click-away, useEffect cleanup) to prevent stale modifier state"

patterns-established:
  - "Dual-event capture: keydown adds to heldRef, keyup reads-then-deletes, all-released fires combo"
  - "Progressive display: heldDisplay state driven from sorted heldRef on each keydown"

requirements-completed: [UI-01, UI-02]

# Metrics
duration: 2min
completed: 2026-03-03
---

# Phase 17 Plan 01: Frontend Capture UI Summary

**Dual keydown+keyup capture in HotkeyCapture.tsx enabling modifier-only combos (Ctrl+Win) alongside existing standard hotkey capture**

## Performance

- **Duration:** ~2 min
- **Started:** 2026-03-03T14:29:42Z
- **Completed:** 2026-03-03T14:31:34Z
- **Tasks:** 1/2 (Task 2 is checkpoint:human-verify — awaiting user verification)
- **Files modified:** 1

## Accomplishments
- Added `modifierToken()` helper that maps `e.code` to canonical modifier tokens (`ctrl`, `alt`, `shift`, `meta`) — correctly uses `e.code` not `e.ctrlKey`/`e.metaKey` which are post-release false on `keyup`
- Added `MODIFIER_ORDER` constant ensuring stored combo is deterministic regardless of press order (Win+Ctrl → `ctrl+meta`, Ctrl+Win → `ctrl+meta`)
- Added `heldRef` (useRef Set) tracking held modifiers across keydown/keyup with correct read-before-delete semantics on `keyup`
- Added `heldDisplay` state driving progressive `Ctrl + Win...` feedback during capture
- `keyup` handler emits modifier-only combo when all modifiers released; `keydown` handler clears `heldRef` on non-modifier key press to prevent spurious modifier-only emit
- All cancel paths (Escape, click-away, `useEffect` cleanup) clear `heldRef` and `heldDisplay`
- `normalizeKey` and `formatHotkey` unchanged — standard combo capture and `meta` → `Win` display unaffected

## Task Commits

Each task was committed atomically:

1. **Task 1: Add modifier-only capture via keyup listener with progressive display** - `fd62885` (feat)

**Plan metadata:** pending (Task 2 checkpoint not yet verified)

## Files Created/Modified
- `src/components/HotkeyCapture.tsx` - Dual keydown+keyup capture, modifierToken helper, MODIFIER_ORDER, heldRef, heldDisplay, progressive display render

## Decisions Made
- Used `e.code` not `e.ctrlKey`/`e.metaKey` in `modifierToken` — flags reflect post-release state on `keyup`, so `e.code` is the only reliable way to identify which modifier was released
- Read-before-delete pattern on `keyup`: snapshot `tokens` from `heldRef` before calling `heldRef.current.delete(token)` — the snapshot is the pre-release combo
- `MODIFIER_ORDER` assigns `ctrl:0, alt:1, shift:2, meta:3` — consistent with how `normalizeKey` orders them in the standard path (`ctrlKey` first, `metaKey` last)
- `heldRef` cleared on Escape, click-away handler, and `useEffect` return cleanup — prevents stale modifier state if user opens capture, holds Ctrl, clicks away, then opens again

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Task 2 (checkpoint:human-verify) requires user to run the app and test Ctrl+Win capture, standard combo backward compatibility, cancel paths, and progressive display
- Once verified, phase 17 plan 01 is complete and phase 17 is complete (only 1 plan)
- Phase 18 (release/packaging) is unblocked

## Self-Check: PASSED
- src/components/HotkeyCapture.tsx — FOUND
- .planning/phases/17-frontend-capture-ui/17-01-SUMMARY.md — FOUND
- Commit fd62885 — FOUND

---
*Phase: 17-frontend-capture-ui*
*Completed: 2026-03-03*
