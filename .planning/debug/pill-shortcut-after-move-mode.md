---
status: resolved
trigger: "pill-shortcut-after-move-mode: After exiting pill move mode (by releasing mouse), the shortcut key requires 2 presses to reactivate the pill. The first press does nothing."
created: 2026-03-05T00:00:00Z
updated: 2026-03-05T00:00:00Z
---

## Current Focus

hypothesis: CONFIRMED - pipeline stays in RECORDING state after move mode exit via any path (hotkey or click)
test: verified both exit paths converge on exit_pill_move_mode; pipeline reset placed there
expecting: both hotkey-exit and click-exit now reset pipeline to IDLE
next_action: human verification of click-exit path

## Symptoms

expected: After exiting move mode, pressing the shortcut key once should reactivate the pill (start recording)
actual: Need to press the shortcut key twice — first press does nothing, second press works
errors: No error messages
reproduction: Run the app, trigger recording so pill appears, long-press pill for 600ms to enter move mode, drag pill to new position, release mouse button to exit move mode, press shortcut key — nothing happens. Press shortcut key again — pill activates.
started: After quick task 41 (pill move mode feature added)

## Eliminated

- hypothesis: combo_active stale state in keyboard_hook.rs prevents first keypress detection
  evidence: combo_active is properly reset when keys are released; the hook correctly fires HookEvent::Pressed on next combo press regardless of move mode state
  timestamp: 2026-03-05

- hypothesis: PillMoveActive flag not properly cleared
  evidence: flag is cleared in handle_hotkey_event before emitting pill-exit-move; confirmed in code
  timestamp: 2026-03-05

- hypothesis: focus/window state affecting keyboard hook
  evidence: WH_KEYBOARD_LL is a global hook unaffected by window focus; hook fires regardless
  timestamp: 2026-03-05

## Evidence

- timestamp: 2026-03-05
  checked: lib.rs handle_hotkey_event lines 536-543 (move mode exit block)
  found: when pressed=true and PillMoveActive, clears flag, emits pill-exit-move, returns early — no pipeline state is touched
  implication: if pipeline was in RECORDING state when move mode was entered, it remains in RECORDING state after exit

- timestamp: 2026-03-05
  checked: lib.rs handle_hotkey_event lines 627-648 (released/pressed=false branch)
  found: the released handler checks PillMoveActive — if true, returns early (line 630-633); this means Ctrl+Win release during move mode is correctly ignored, but the pipeline is still in RECORDING state
  implication: pipeline.state = RECORDING persists through the entire move mode session

- timestamp: 2026-03-05
  checked: lib.rs handle_hotkey_event lines 546-573 (hold-to-talk start)
  found: pipeline.transition(IDLE, RECORDING) — this CAS fails silently if pipeline is already in RECORDING state
  implication: after exiting move mode, first hotkey press calls transition(IDLE, RECORDING) which fails because pipeline is in RECORDING — nothing happens

- timestamp: 2026-03-05
  checked: full scenario trace
  found: user presses key after move mode exit → transition(IDLE, RECORDING) fails → no recording starts → user releases key → transition(RECORDING, PROCESSING) succeeds → processes audio accumulated since original recording session started (long ago, essentially garbage data)
  implication: "second press works" = the release of the second press finalizes the phantom recording. First press appears to do nothing because the pill is hidden and the recording start fails silently.

- timestamp: 2026-03-05
  checked: click-exit path in Pill.tsx (handlePointerDown when inMoveModeRef.current) and exitMoveMode()
  found: both click-exit and hotkey-exit converge on invoke("exit_pill_move_mode") — click calls it directly via exitMoveMode(), hotkey path triggers it via the pill-exit-move event listener
  implication: pipeline reset must live in exit_pill_move_mode (pill.rs) to cover both paths; first fix (in lib.rs hotkey handler) was redundant — moved to pill.rs and removed from lib.rs

## Resolution

root_cause: When the user enters move mode while recording (hold-to-talk, pill showing), and then exits move mode via the hotkey, handle_hotkey_event's move mode exit block (lines 536-543) returns early without touching the pipeline. The pipeline remains in RECORDING state. On the next hotkey press, pipeline.transition(IDLE, RECORDING) fails (pipeline is already RECORDING), so nothing happens. The user must press+release the key again, at which point the RECORDING->PROCESSING transition fires, processing audio accumulated since the original recording session (including all the silence during dragging).

fix: Moved pipeline reset (RECORDING→IDLE, audio discard) into exit_pill_move_mode in pill.rs — the single convergence point for all move mode exit paths. Removed redundant copy from lib.rs hotkey handler. Both hotkey-exit and click-exit now call exit_pill_move_mode which: (1) clears PillMoveActive, (2) attempts pipeline.transition(RECORDING, IDLE) — stops level stream, flushes and stops audio capture, releases mic (unless always-listen), resets tray, (3) persists final pill position.

verification: pending human confirmation of click-exit path
files_changed:
  - src-tauri/src/pill.rs (exit_pill_move_mode: added pipeline RECORDING->IDLE reset and audio cleanup)
  - src-tauri/src/lib.rs (handle_hotkey_event: removed redundant pipeline reset, now delegated to exit_pill_move_mode)
