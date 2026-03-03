---
status: resolved
trigger: "stuck-win-key-paste-failure: Ctrl+Win hotkey sometimes fails to paste transcribed text at cursor, and sometimes causes keyboard to glitch (Win key stuck)"
created: 2026-03-03T00:00:00Z
updated: 2026-03-03T00:20:00Z
---

## Current Focus

hypothesis: CONFIRMED. Win-first press order: Win-down passed through to OS, combo activates on Ctrl, Win-up suppressed — OS never gets Win-up. Simulated Ctrl+V fires as Win+Ctrl+V. Paste silently fails. Keyboard stuck with Win held.
test: Full code trace of keyboard_hook.rs hook_proc Win-first vs Ctrl-first paths. Confirmed by build success.
expecting: Fix eliminates stuck Win key and paste failure for all press orders.
next_action: Human verification — exercise Ctrl+Win hotkey with both press orders repeatedly.

## Symptoms

expected: After Ctrl+Win hold-to-talk transcription completes, text should always appear at the cursor position in the focused application.
actual: Sometimes transcribed text doesn't appear at cursor even though PowerShell/logs show transcription completed. Sometimes after using the hotkey, the keyboard acts as if Win key is stuck (pressing E opens Explorer, D shows Desktop). Both symptoms are intermittent.
errors: No error messages — inject_text returns Ok(()), pipeline logs success. Silent failures.
reproduction: Use Ctrl+Win hold-to-talk repeatedly. Intermittent — depends on which physical key (Ctrl vs Win) is pressed/released first.
started: Since Phase 15 Ctrl+Win keyboard hook was implemented. The issue is inherent to the design.

## Eliminated

(none yet)

## Evidence

- timestamp: 2026-03-03T00:00:00Z
  checked: keyboard_hook.rs Win keydown handler (lines 292-325)
  found: When Win is pressed first (combo not active), Win keydown falls through to CallNextHookEx at line 324 — OS receives Win-down. When Win is pressed second (Ctrl already held, combo activates), Win keydown is suppressed via LRESULT(1) at line 316.
  implication: The Win-first path delivers Win-down to OS. The Ctrl-first path does not. This asymmetry is the root of the inconsistency.

- timestamp: 2026-03-03T00:00:00Z
  checked: keyboard_hook.rs Win keyup handler (lines 327-346)
  found: Win keyup is ALWAYS suppressed (LRESULT(1) at line 341) when combo_active is true, regardless of which key was pressed first. OS never receives Win-up when Win was pressed first.
  implication: Win-first path: OS gets Win-down but not Win-up → Win key stuck at OS level for duration of session.

- timestamp: 2026-03-03T00:00:00Z
  checked: inject.rs inject_text() lines 36-38
  found: Only simulates Ctrl+V — no Win key release injected before paste. If OS thinks Win is held, this becomes Win+Ctrl+V.
  implication: Even if hook sends no Win-up, inject_text could defensively inject Win-up before Ctrl+V to protect against the stuck state.

- timestamp: 2026-03-03T00:00:00Z
  checked: inject.rs error handling in lines 36-38
  found: enigo.key(Key::Unicode('v'), Click) uses ? to early-return on error. If it errors, enigo.key(Key::Control, Release) is never called. Ctrl remains logically pressed in Enigo's state (though Enigo Drop may handle it).
  implication: Ctrl key can leak on paste error. Minor but fixable alongside the Win key fix.

## Resolution

root_cause: Two-part root cause. Primary: keyboard_hook.rs passes Win-down through to OS when Win is pressed before Ctrl (combo not yet active at Win-down time), then suppresses Win-up when combo_active is true (line 341 / now 382). OS receives Win-down but no Win-up — Win key stuck at OS level. inject_text()'s simulated Ctrl+V then fires as Win+Ctrl+V, which is not a paste shortcut, so text silently does not appear. This also explains the E=Explorer, D=Desktop glitch. Secondary: inject_text() did not defensively release Win before pasting (no safety net), and did not ensure Ctrl is released if V-click fails.
fix: |
  Three changes applied:
  1. keyboard_hook.rs: Added inject_win_up(vk) helper (SendInput with KEYEVENTF_KEYUP). Called it in Win keyup handler immediately before LRESULT(1) suppression. OS now always sees balanced Win down/up regardless of which key was pressed first.
  2. inject.rs: Added release_win_keys() helper using Enigo Key::LWin and Key::RWin Release. Called before Ctrl+V simulation as defensive layer in case hook-level fix races or fails.
  3. inject.rs: Restructured Ctrl+V as Press / store V result / Release / propagate V result — Ctrl is always released even if V-click returns Err.
verification: cargo check passed cleanly. User confirmed working in runtime testing (2026-03-03).
files_changed:
  - src-tauri/src/keyboard_hook.rs
  - src-tauri/src/inject.rs
