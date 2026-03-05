---
phase: quick-41
verified: 2026-03-04T00:00:00Z
status: human_needed
score: 5/5 must-haves verified
re_verification: false
human_verification:
  - test: "Long-press pill for ~600ms and observe visual feedback"
    expected: "Purple glow and slight scale animation appear on the pill after 600ms hold"
    why_human: "CSS animation and timing behavior cannot be verified programmatically; requires live UI interaction"
  - test: "Drag pill to a new position while in drag mode"
    expected: "Pill window moves in real time following the cursor, centered on cursor position"
    why_human: "Real-time window movement via IPC requires live runtime verification; cannot be statically analyzed"
  - test: "Release pointer after dragging"
    expected: "Pill stays at new position; position is persisted to settings.json"
    why_human: "File I/O side effect during drag needs runtime confirmation"
  - test: "Restart the app and trigger recording"
    expected: "Pill appears at the last dragged position, not bottom-center"
    why_human: "Cross-restart persistence requires running the app"
  - test: "Double-click the pill"
    expected: "Pill immediately snaps back to bottom-center home position"
    why_human: "Window repositioning on double-click requires live runtime verification"
  - test: "Release hold-to-talk hotkey while mid-drag"
    expected: "Pill does not hide during the drag; hides only after pointer is released"
    why_human: "Deferred hide timing behavior requires combined hotkey + drag interaction to verify"
---

# Phase quick-41: Long-Press Pill Drag Reposition Verification Report

**Phase Goal:** Long-press pill to drag reposition and double-click to reset home. iPhone-style long-press (~600ms) enters drag mode with visual feedback (glow/scale), user can drag pill to any screen position, double-click snaps pill back to default bottom-center. Persist custom position across app restarts.
**Verified:** 2026-03-04
**Status:** human_needed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Long-pressing the pill for ~600ms enters drag mode (visual glow + slight scale) | VERIFIED | `handlePointerDown` in Pill.tsx starts a 600ms `setTimeout` that sets `dragState = "ready"` and calls `setPointerCapture`. CSS `.pill-drag-ready` class applied when `dragState === "ready"`, with `pill-drag-ready` keyframe animation (scale 1.04) and purple glow border/box-shadow in pill.css lines 117-127. |
| 2 | While in drag mode, moving the mouse repositions the pill window in real time | VERIFIED | `handlePointerMove` in Pill.tsx calls `invoke("set_pill_position", { x, y })` using `e.screenX - 89` / `e.screenY - 23` for centering, throttled to 16ms. Applies in both `"ready"` and `"dragging"` drag states. |
| 3 | Releasing the mouse exits drag mode and persists the new position to settings.json | VERIFIED | `handlePointerUp` sets `dragState = "idle"`. Position was already persisted on each `set_pill_position` call in `pill.rs` lines 124-126: reads settings, writes `json["pill_position"] = {x, y}`, calls `write_settings`. |
| 4 | Double-clicking the pill resets it to the default bottom-center home position | VERIFIED | `handleDoubleClick` in Pill.tsx calls `invoke("reset_pill_position")`. `reset_pill_position` in pill.rs removes `pill_position` from settings and calls `compute_home_position()` to reposition window. |
| 5 | When show_pill is called, if a saved position exists it is used instead of recomputing bottom-center | VERIFIED | `show_pill()` in pill.rs lines 83-106 reads settings, checks `json["pill_position"]["x"]` and `["y"]` as i64, uses saved coords if present, falls back to `compute_home_position()` otherwise. |
| 6 | Custom position survives app restart | VERIFIED (pending runtime) | Position is written to `settings.json` via `write_settings` on every `set_pill_position` call. On next launch `show_pill` reads the same file. The persistence mechanism is structurally correct; runtime confirmation needed. |

**Score:** 5/5 truths verified (automated); runtime confirmation required for truths 2, 3, 5, 6.

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/Pill.tsx` | Long-press detection, drag mode state, double-click reset, IPC calls | VERIFIED | DragState type, `longPressTimerRef`, `handlePointerDown` (600ms timer), `handlePointerMove` (invoke throttled), `handlePointerUp` (flush deferred hide), `handleDoubleClick` (reset_pill_position), all pointer handlers wired to root div. |
| `src/pill.css` | drag-mode glow animation keyframe | VERIFIED | `@keyframes pill-drag-ready` (scale 1.04) at lines 117-120, `.pill-drag-ready` class with purple glow at lines 122-127, `.pill-dragging` class with stronger glow at lines 129-133. |
| `src-tauri/src/pill.rs` | show_pill reads saved pill_position; set_pill_position command moves window + persists; reset_pill_position clears saved + recenters | VERIFIED | All three behaviors implemented. `compute_home_position()` extracted as helper (lines 33-75). `show_pill()` reads settings and branches on saved position (lines 83-106). `set_pill_position` command at lines 118-128. `reset_pill_position` command at lines 131-153. |
| `src-tauri/src/lib.rs` | Registers set_pill_position and reset_pill_position as Tauri invoke handlers | VERIFIED | Both `pill::set_pill_position` and `pill::reset_pill_position` present in `generate_handler!` macro at lines 1707-1708. `read_settings` and `write_settings` are `pub(crate)` (lines 211, 218). |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/Pill.tsx` | `src-tauri/src/lib.rs` | `invoke('set_pill_position', {x, y})` and `invoke('reset_pill_position')` | WIRED | Both invoke calls present in Pill.tsx (lines 137, 148, 193). Both commands registered in invoke_handler in lib.rs (lines 1707-1708). `invoke` imported from `@tauri-apps/api/core` at Pill.tsx line 3. |
| `src-tauri/src/pill.rs show_pill()` | `settings.json pill_position key` | `read_settings -> json["pill_position"]` | WIRED | `show_pill()` calls `crate::read_settings(app)` and accesses `json["pill_position"]["x"]` and `json["pill_position"]["y"]` (pill.rs lines 83-89). `set_pill_position` writes `json["pill_position"] = serde_json::json!({"x": x, "y": y})` (pill.rs line 125). |

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|---------|
| QUICK-41 | 41-PLAN.md | Long-press pill drag reposition, double-click reset, position persistence | SATISFIED | All six observable truths verified with code evidence. Three commits (84dda35, 7ada147, fa5dc52) confirmed in git log. |

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| None | — | — | — | No stubs, TODOs, empty returns, or placeholder patterns found in any modified file. |

---

### Human Verification Required

**1. Long-press visual feedback**

**Test:** Start the app, trigger recording so the pill is visible, hold the pointer down on the pill for approximately 600ms without moving.
**Expected:** Purple glow border and scale animation (1.04x) appear on the pill after 600ms. Cursor changes to `grab`.
**Why human:** CSS animation timing and visual appearance cannot be confirmed by static analysis.

**2. Real-time drag repositioning**

**Test:** After entering drag mode (step 1), move the mouse to a different area of the screen.
**Expected:** The pill window follows the cursor in real time, remaining centered under the cursor (screenX-89, screenY-23 offset).
**Why human:** Window movement via IPC requires a running Tauri instance to confirm.

**3. Position persistence on release**

**Test:** Drag the pill to a new position and release the pointer button.
**Expected:** The pill stays at the new position. Open `settings.json` (in app data dir) to confirm `pill_position` key was written with the new coordinates.
**Why human:** File I/O confirmation requires runtime execution.

**4. Cross-restart position persistence**

**Test:** After saving a custom position (step 3), close the app entirely and relaunch it. Trigger recording.
**Expected:** Pill appears at the previously dragged position, not at the default bottom-center.
**Why human:** Requires restarting the app and observing startup behavior.

**5. Double-click home reset**

**Test:** While pill is visible (any state), double-click on it.
**Expected:** Pill immediately repositions to bottom-center of the current monitor's work area. Check `settings.json` to confirm `pill_position` key was removed.
**Why human:** Window repositioning and file mutation require live runtime verification.

**6. Deferred hide during drag (regression guard)**

**Test:** Start a hold-to-talk recording session. While the pill is visible, enter drag mode (600ms long-press) and then release the hotkey while still holding the pointer.
**Expected:** Pill does NOT hide while the pointer is still pressed. Only after releasing the pointer does the pill perform its exit animation.
**Why human:** Requires coordinating hotkey release timing with active pointer hold; not statically verifiable.

---

### Gaps Summary

No gaps found. All automated verifications passed:

- `src/Pill.tsx`: Fully implemented drag state machine with pointer event handlers, 600ms long-press timer, 16ms throttled IPC calls, deferred hide pattern, and double-click reset.
- `src/pill.css`: Both `.pill-drag-ready` and `.pill-dragging` CSS classes exist with the specified purple glow and keyframe animation.
- `src-tauri/src/pill.rs`: `compute_home_position()` helper extracted and used by both `show_pill()` and `reset_pill_position()`. `set_pill_position` and `reset_pill_position` commands fully implemented with settings persistence.
- `src-tauri/src/lib.rs`: Both commands registered in `generate_handler!`. `read_settings`/`write_settings` promoted to `pub(crate)`.
- All three commits (84dda35, 7ada147, fa5dc52) confirmed in git log.

Pending items are exclusively runtime/visual behaviors that require human verification per the plan's checkpoint task.

---

_Verified: 2026-03-04_
_Verifier: Claude (gsd-verifier)_
