---
status: awaiting_human_verify
trigger: "pill move mode broken — pill doesn't follow mouse cursor, exit may not work"
created: 2026-03-04T00:00:00Z
updated: 2026-03-04T00:01:00Z
---

## Current Focus

hypothesis: CONFIRMED — document mousemove only fires when the cursor is over the 178x46 pill webview window.
test: replaced frontend mousemove approach with a backend Rust async polling loop
expecting: pill now follows cursor globally; hotkey/click exit still works
next_action: awaiting human verification

## Symptoms

expected: Long-press (600ms) enters move mode with green glow. Pill follows mouse cursor globally and smoothly wherever it goes. Clicking the pill OR pressing the hotkey exits move mode and hides the pill.
actual: Long-press enters move mode (green glow appears) but pill does NOT follow the mouse cursor. Exit via click/hotkey may also not work.
errors: No error messages — behavior just doesn't work as expected.
reproduction: Run `npm run tauri dev`, trigger recording so pill appears, long-press pill for 600ms to enter move mode. Try moving mouse — pill stays in place.
started: Just implemented. Never worked correctly.

## Eliminated

- hypothesis: backend PillMoveActive flag not set / IPC not registered
  evidence: enter_pill_move_mode command is registered in lib.rs (line 1732), PillMoveActive is managed (line 1695), command stores true to flag. Hotkey exit correctly reads flag and emits pill-exit-move. Backend is wired correctly.
  timestamp: 2026-03-04T00:00:00Z

- hypothesis: click-to-exit timing bug in pointerDown handler
  evidence: inMoveModeRef is a ref (not state), set synchronously in the setTimeout callback before setDisplayState. No async gap — the ref will be true by the time any pointerDown fires. Exit path is correct.
  timestamp: 2026-03-04T00:00:00Z

## Evidence

- timestamp: 2026-03-04T00:00:00Z
  checked: tauri.conf.json pill window definition
  found: pill window is 178x46px, transparent, decorations=false
  implication: the entire webview is 178x46px — document mousemove only fires when the OS cursor is physically over that rectangle

- timestamp: 2026-03-04T00:00:00Z
  checked: Pill.tsx handleGlobalMouseMove and useEffect
  found: uses document.addEventListener("mousemove") — this is a DOM event that fires only when the cursor is within the webview bounds
  implication: the moment the user moves the mouse even 1px outside the 178x46 pill, all mousemove events stop. The pill never moves, so the cursor is never back inside. Complete deadlock.

- timestamp: 2026-03-04T00:00:00Z
  checked: pill.rs set_pill_position
  found: correctly calls pill_window.set_position(). Also writes to settings.json on every call — this is a performance issue during drag but not the root cause.
  implication: the backend move mechanism works, it's just never being called because frontend never gets the events

- timestamp: 2026-03-04T00:00:00Z
  checked: pill.rs enter_pill_move_mode / exit_pill_move_mode
  found: these just toggle PillMoveActive flag. No cursor tracking loop exists in backend.
  implication: need to add a backend polling loop that runs while PillMoveActive=true

## Resolution

root_cause: document.addEventListener("mousemove") in a 178x46 Tauri webview only fires events when the OS cursor is physically over that tiny window. As soon as the user moves the mouse to drag the pill, the cursor exits the webview and events stop completely. The pill never follows because it never receives the cursor position.

fix: Replaced frontend mousemove approach with a backend Rust async polling loop in enter_pill_move_mode. The loop calls cursor_position() every 16ms via tokio::time::sleep and repositions the pill window. The loop exits when PillMoveActive is cleared by exit_pill_move_mode. Position is persisted via outer_position() when move mode exits instead of on every movement. Frontend handleGlobalMouseMove callback and its useEffect removed.

verification: awaiting human confirmation

files_changed:
  - src-tauri/src/pill.rs — enter_pill_move_mode now spawns tracking loop; exit_pill_move_mode persists final position; set_pill_position no longer writes settings
  - src/Pill.tsx — removed handleGlobalMouseMove useCallback, its useEffect, and lastInvokeRef
