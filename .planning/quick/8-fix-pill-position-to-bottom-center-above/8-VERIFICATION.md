---
phase: quick-8
verified: 2026-03-01T00:00:00Z
status: human_needed
score: 3/4 must-haves verified automatically
human_verification:
  - test: "Press hotkey and observe pill position on primary monitor"
    expected: "Pill appears horizontally centered just above taskbar (14px margin), not at a random saved position"
    why_human: "Cannot verify screen coordinates without running the app"
  - test: "Attempt to drag the pill while recording is active"
    expected: "Pill does not move — no grab cursor, no repositioning"
    why_human: "No-drag behavior requires interaction testing"
  - test: "On a multi-monitor setup, move cursor to second monitor and press hotkey"
    expected: "Pill appears on the second monitor, bottom-center above its taskbar"
    why_human: "Multi-monitor routing is runtime behavior, cannot verify from source alone"
---

# Phase quick-8: Fix Pill Position to Bottom-Center Verification Report

**Phase Goal:** Fix pill position to bottom-center above the taskbar, remove dragging, and add multi-monitor support
**Verified:** 2026-03-01
**Status:** human_needed (all automated checks pass; runtime behavior requires human testing)
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Pill appears horizontally centered on the monitor where the cursor currently is | ? HUMAN NEEDED | `show_pill()` computes `x = wa_x + (wa_w - 178) / 2` against cursor's monitor work area — correct formula; runtime not verified |
| 2 | Pill sits just above the taskbar (bottom of monitor work area) | ? HUMAN NEEDED | `y = wa_y + wa_h - 46 - 14` with `work_area()` excluding taskbar — correct formula; runtime not verified |
| 3 | Pill cannot be dragged or repositioned by the user | VERIFIED | `Pill.tsx` has no `onMouseDown`, `onMouseUp`, `handleMouseDown`, `handleMouseUp`, `cursor-grab`, or `active:cursor-grabbing` — grep returned 0 hits |
| 4 | On a multi-monitor setup, pressing the hotkey on monitor 2 shows the pill on monitor 2 | ? HUMAN NEEDED | `available_monitors()` + cursor bounds check selects correct monitor — logic correct; multi-monitor runtime not testable from source |

**Score:** 1/4 truths fully verifiable from source alone (truth 3); 3/4 require human runtime verification. All automated-verifiable aspects pass.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src-tauri/src/pill.rs` | `show_pill()` that positions pill on cursor monitor and emits pill-show | VERIFIED | `pub fn show_pill(app: &tauri::AppHandle)` present at line 33; uses `cursor_position()`, `available_monitors()`, `work_area()`, `set_position()`, then `emit_to("pill", "pill-show", ())` |
| `src-tauri/src/lib.rs` | All pill-show calls replaced with `pill::show_pill`, saved position restore removed | VERIFIED | 4 occurrences of `pill::show_pill(&app)` at lines 176, 201, 1049, 1074; 0 occurrences of raw `emit_to("pill", "pill-show", ())`; no `pill-position` references; no `Restore saved pill position` block |
| `src/Pill.tsx` | No drag logic, no saved position, no cursor-grab styling | VERIFIED | No `PhysicalPosition` import, no `plugin-store` import, no `useCallback`, no `initPosition`, no `handleMouseDown`, no `handleMouseUp`, no `cursor-grab`, no `onMouseDown`, no `onMouseUp` |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src-tauri/src/lib.rs` | `src-tauri/src/pill.rs` | `pill::show_pill` called at all 4 recording-start sites | VERIFIED | Lines 176, 201, 1049, 1074 all call `pill::show_pill(&app)` — zero raw `pill-show` emissions remain |
| `src-tauri/src/pill.rs` | Tauri monitor API | `cursor_position` + `available_monitors` + `work_area` | VERIFIED | All three API calls present at lines 36, 45, and 57/59; `work_area().clone()` used correctly; position computed from `work_area.position` and `work_area.size` |

### Requirements Coverage

| Requirement | Description | Status | Evidence |
|-------------|-------------|--------|----------|
| PILL-POS-01 | Pill positions to bottom-center of cursor's monitor | VERIFIED (logic) / HUMAN (runtime) | `show_pill()` implements monitor detection and bottom-center formula |
| PILL-POS-02 | Pill is not draggable / no saved position | VERIFIED | All drag code removed from `Pill.tsx`; all `pill-position` store access removed from both `lib.rs` and `Pill.tsx` |

### Anti-Patterns Found

None. No TODO/FIXME/HACK/PLACEHOLDER comments in modified files. No empty return stubs. No console.log-only handlers.

**Note on `pill-rainbow-border`:** `Pill.tsx` line 92 still contains `${displayState === "recording" ? "pill-rainbow-border" : ""}`. The plan explicitly states this is out of scope for quick-8 and should not be removed here. This is not a gap for this task.

### Human Verification Required

#### 1. Pill bottom-center positioning on primary monitor

**Test:** Build and run with `cargo tauri dev --features whisper`. Press the recording hotkey.
**Expected:** Pill appears horizontally centered at the bottom of the primary monitor, visually just above the taskbar (14px gap).
**Why human:** Screen pixel coordinates require the running app to confirm.

#### 2. No dragging possible

**Test:** While the pill is visible (recording state), attempt to click and drag it.
**Expected:** Pill does not move. No grab cursor appears on hover or drag attempt.
**Why human:** Mouse interaction behavior requires runtime testing.

#### 3. Multi-monitor routing

**Test:** Move the mouse cursor to a secondary monitor, then press the hotkey.
**Expected:** Pill appears on the secondary monitor, bottom-center above that monitor's taskbar.
**Why human:** Multi-monitor behavior requires a multi-monitor environment and runtime observation.

### Gaps Summary

No gaps in the implementation. All source-verifiable requirements pass:

- `pub fn show_pill()` is fully implemented in `pill.rs` with correct monitor detection, work area computation, and position formula.
- All 4 `emit_to("pill", "pill-show", ())` call sites in `lib.rs` are replaced with `pill::show_pill(&app)`.
- The saved-position restore block is fully removed from `lib.rs` `setup()`.
- `Pill.tsx` contains no drag logic, no position-save/restore, and no related imports.
- No `pill-position` references remain anywhere in the codebase.

The three human-needed items are runtime behavior verifications, not implementation gaps. The logic is correct as written.

---

_Verified: 2026-03-01_
_Verifier: Claude (gsd-verifier)_
