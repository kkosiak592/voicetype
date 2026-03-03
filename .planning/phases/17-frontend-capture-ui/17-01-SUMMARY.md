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
status: complete

# Metrics
duration: 30min
completed: 2026-03-03
---

# Phase 17 Plan 01: Frontend Capture UI Summary

**Dual keydown+keyup capture in HotkeyCapture.tsx enabling modifier-only combos (Ctrl+Win) alongside existing standard hotkey capture**

## Performance

- **Duration:** ~2 min
- **Started:** 2026-03-03T14:29:42Z
- **Completed:** 2026-03-03T14:31:34Z
- **Tasks:** 2/2 (Task 2 checkpoint verified)
- **Files modified:** 4

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
2. **Task 2: Human verification** - approved
3. **Fix: HookAvailable builder registration** - `510fd66` (fix)
4. **Fix: comboRef for multi-key release** - `b0dc56b` (fix)
5. **Fix: Hook warning condition + status refresh** - `bef1625` (fix)

## Files Created/Modified
- `src/components/HotkeyCapture.tsx` - Dual keydown+keyup capture, modifierToken helper, MODIFIER_ORDER, heldRef/comboRef, heldDisplay, progressive display render
- `src-tauri/src/lib.rs` - HookAvailable registered on Builder (before webview creation) instead of in setup()
- `src/components/sections/GeneralSection.tsx` - Hook warning only shows for modifier-only hotkeys
- `src/App.tsx` - Re-queries get_hook_status after hotkey rebind

## Decisions Made
- Used `e.code` not `e.ctrlKey`/`e.metaKey` in `modifierToken` — flags reflect post-release state on `keyup`, so `e.code` is the only reliable way to identify which modifier was released
- Read-before-delete pattern on `keyup`: snapshot `tokens` from `heldRef` before calling `heldRef.current.delete(token)` — the snapshot is the pre-release combo
- `MODIFIER_ORDER` assigns `ctrl:0, alt:1, shift:2, meta:3` — consistent with how `normalizeKey` orders them in the standard path (`ctrlKey` first, `metaKey` last)
- `heldRef` cleared on Escape, click-away handler, and `useEffect` return cleanup — prevents stale modifier state if user opens capture, holds Ctrl, clicks away, then opens again

## Deviations from Plan

1. **HookAvailable panic at startup** — webview2 COM init pumps Win32 messages before setup() runs, triggering get_hook_status IPC before manage(). Fixed by registering on Builder, matching CachedGpuMode pattern.
2. **comboRef needed** — heldRef is depleted as keys release one at a time; last keyup only saw the last modifier. Added comboRef to track full session set.
3. **Hook warning false positive** — Warning showed for standard combos. Added modifier-only check + post-rebind status refresh.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All tasks complete, human verification passed
- Phase 17 complete (only 1 plan)
- Phase 18 (release/packaging) is unblocked

## Self-Check: PASSED
- src/components/HotkeyCapture.tsx — FOUND
- .planning/phases/17-frontend-capture-ui/17-01-SUMMARY.md — FOUND
- Commit fd62885 — FOUND

---
*Phase: 17-frontend-capture-ui*
*Completed: 2026-03-03*
