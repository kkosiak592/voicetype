# Feature Research

**Domain:** Modifier-only hotkey activation for local voice-to-text desktop dictation (Windows)
**Researched:** 2026-03-02
**Confidence:** HIGH (Wispr Flow official docs, Windows Speech Recognition official docs, SuperWhisper docs, Windows API references)

---

## Context: What Is Already Built

This is a subsequent milestone. The existing app (v1.0/v1.1) already has:

- Global hotkey via `tauri-plugin-global-shortcut` (RegisterHotKey API)
- Hold-to-talk mode (hold to record, release to transcribe)
- Toggle mode (tap to start, tap to stop)
- Configurable hotkey in settings UI with capture dialog
- Floating pill overlay, audio visualizer, system tray

**The new milestone (v1.2) adds one capability:** Ctrl+Win modifier-only hotkey activation, implemented via a custom WH_KEYBOARD_LL low-level keyboard hook replacing the RegisterHotKey API.

Research below covers only what is needed for this new feature.

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist for a modifier-only hotkey activation. Missing these = the feature feels broken or incomplete.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Hold-to-talk behavior on modifier-only hotkey | Wispr Flow (the closest competitor on Windows) uses exactly Ctrl+Win as hold-to-talk. Users familiar with Wispr Flow will expect hold = record, release = transcribe | LOW | Behavior is identical to existing hold-to-talk; only the trigger key changes from a standard hotkey to a modifier-only combo |
| Press-order independence (debounce) | On a physical keyboard, pressing Ctrl+Win means one key always lands slightly before the other. The feature must activate regardless of which modifier is pressed first | MEDIUM | ~50ms debounce window: if both keys are down within 50ms, treat as a combo. Without this, users get inconsistent activation |
| Start menu suppression when Ctrl+Win activates recording | Windows shows the Start menu on Win keyup unless something intervenes. If the Start menu pops up every time the user finishes a dictation, the feature is unusable | HIGH | WH_KEYBOARD_LL hook must intercept Win key events and return non-zero to consume them when the combo is active. This is the hardest part of the feature |
| Visual confirmation that hotkey was received | Users must see the pill overlay transition to recording state when they press the combo. Without this, there is no feedback that the modifier combo registered correctly | LOW | No new UI needed; existing pill overlay states (idle → recording → processing) already provide this signal |
| Reliable release detection | Recording must stop exactly when the user releases the key(s). Missed keyup events cause stuck recording state | MEDIUM | WH_KEYBOARD_LL provides WM_KEYDOWN and WM_KEYUP for all keys including modifiers. Must handle both left and right Win/Ctrl variants (VK_LWIN, VK_RWIN, VK_LCONTROL, VK_RCONTROL) |
| Settings UI to select modifier-only combos | Users who want Ctrl+Win must be able to configure it in the existing hotkey capture dialog. Showing only "Ctrl + Win" with no other key is not how the existing capture dialog works | MEDIUM | Capture dialog must accept modifier-only combos as valid input and display them appropriately (e.g., "Ctrl + Win" not "Ctrl + Win + [nothing]") |

---

### Differentiators (Competitive Advantage)

Features that set this implementation apart from competitors and naive approaches.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Double-tap toggle mode on modifier-only hotkey | Wispr Flow documents a "double-tap for hands-free" mode. If two rapid taps of the modifier combo are detected, activate toggle mode instead of hold-to-talk. Eliminates need to hold keys during long dictations | HIGH | Requires timing logic on top of the debounce window. Two taps within ~300ms = toggle mode entry. Complexity is high; defer unless there is explicit user demand |
| Left vs. right modifier distinction | Allow binding specifically to LCtrl+LWin vs. RCtrl+RWin. Power users who bind other tools to right-side modifiers would benefit | LOW | WH_KEYBOARD_LL gives VK_LCONTROL vs. VK_RCONTROL and VK_LWIN vs. VK_RWIN. Simple to implement once hook is in place; adds flexibility with minimal code |
| Graceful fallback to RegisterHotKey on hook failure | If the WH_KEYBOARD_LL hook cannot be installed (e.g., antivirus interference, permission issues), the app falls back to the RegisterHotKey API. Users never see a broken hotkey | MEDIUM | Hook installation returns NULL on failure. Detection is straightforward. Fallback path means modifier-only combos are unavailable but standard hotkeys still work. Must surface this to user in settings |
| Persist hook across lock/unlock and session changes | Users who lock their workstation (Win+L) and return expect the hotkey to keep working. WH_KEYBOARD_LL hooks are not automatically re-registered after session events in all configurations | MEDIUM | Listen for WM_WTSSESSION_CHANGE or WTS_SESSION_UNLOCK via WTSRegisterSessionNotification; re-install hook on return from lock |

---

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Capture and suppress all Win key combos (Win+L, Win+D, Win+Tab, etc.) | Users might assume the hook needs to consume all Win key events to work | Suppressing Win+L, Win+D, Win+Tab, Win+E, etc. breaks core Windows navigation. Users will lose access to lock screen, desktop show/hide, Task View, Explorer | Only consume Win key events when Ctrl is simultaneously held AND the app is waiting for the hotkey. All other Win combos pass through unmodified |
| Modifier-only hotkey with no visual feedback delay tolerance | "It should activate instantly" — users want zero delay | A debounce window is required for press-order independence. Without 50ms debounce, pressing Ctrl slightly before Win means the hook sees Win-up without Ctrl-down, producing no activation. Perceived delay at 50ms is imperceptible | Keep debounce window at 50ms. This is below human perception threshold for activation latency |
| Registry-based Win key disable | Simplest way to stop the Start menu (HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced\DisabledHotkeys or HKLM\SYSTEM\CurrentControlSet\Control\Keyboard Layout\Scancode Map) | Disables Win key globally and permanently until registry is restored. If the app crashes before restoring, the user is stuck with a broken Win key. Also requires elevated privileges for some registry paths | Use WH_KEYBOARD_LL hook to consume events selectively and only while the app is running. Hook is automatically removed on process exit |
| Global Win key blocking via Group Policy | Seems authoritative and reliable | Requires domain-joined machine or GPO access. Not available to most personal Windows users. Also disables all Win key shortcuts globally, not just Start menu | WH_KEYBOARD_LL hook is self-contained, requires no elevated privileges, and is automatically scoped to app lifetime |
| Storing hotkey as modifier-only in the existing RegisterHotKey path | Reuse existing code | RegisterHotKey API does not support modifier-only hotkeys. Passing only MOD_CONTROL | MOD_WIN with no virtual key code returns an error. This is a hard Windows API limitation that is why the milestone switches to WH_KEYBOARD_LL | Replace RegisterHotKey with WH_KEYBOARD_LL for all hotkeys, not just modifier-only ones. Unified code path |

---

## Feature Dependencies

```
[Ctrl+Win modifier-only hotkey activation]
    └──requires──> [WH_KEYBOARD_LL keyboard hook module]
                       └──requires──> [SetWindowsHookEx in Rust (via winapi or windows crate)]
                       └──requires──> [Hook runs on dedicated thread with message loop]
                       └──requires──> [Tauri command bridge: hook → frontend events]

[Start menu suppression]
    └──requires──> [WH_KEYBOARD_LL keyboard hook]  <-- cannot suppress without consuming the event at hook level
    └──requires──> [State tracking: combo active / not active]

[Press-order independence (debounce)]
    └──requires──> [WH_KEYBOARD_LL keyboard hook]
    └──requires──> [Timestamp tracking per key event in hook callback]

[Modifier-only combo capture in settings UI]
    └──requires──> [Frontend hotkey capture dialog modification]
    └──requires──> [Existing settings panel]  <-- already built in v1.0
    └──requires──> [Hotkey serialization format that can represent modifier-only combos]

[Fallback to RegisterHotKey on hook failure]
    └──requires──> [WH_KEYBOARD_LL hook installation attempt]
    └──requires──> [Existing RegisterHotKey path]  <-- already built in v1.0; keep as fallback
    └──requires──> [Settings UI indicator showing which hotkey backend is active]

[Hook persistence across lock/unlock]
    └──requires──> [WH_KEYBOARD_LL hook module]
    └──requires──> [WTSRegisterSessionNotification or equivalent session event listener]
```

### Dependency Notes

- **WH_KEYBOARD_LL is the foundational dependency for everything in this milestone.** All other features in v1.2 depend on having a working low-level keyboard hook. The hook must be implemented first before any other v1.2 work proceeds.
- **Start menu suppression depends on hook state tracking.** The hook callback must know whether the Ctrl+Win combo is currently active to decide whether to consume the Win key event. This requires shared state between keydown and keyup handlers within the hook.
- **The existing RegisterHotKey path (tauri-plugin-global-shortcut) must be kept as a fallback**, not removed. Users with hook installation failures still need hotkey functionality. This means the app will have two hotkey backends simultaneously; the settings UI must reflect which one is active.
- **Modifier-only combo capture in the UI has no dependency on the hook itself.** The frontend capture dialog change is independent of the Rust hook implementation and can be developed in parallel.
- **Hook thread isolation is required.** WH_KEYBOARD_LL callbacks must complete in under the system hook timeout (~200ms on Windows 10/11). Any work beyond state tracking and event emission must be offloaded to the Tauri command thread. Blocking in the hook callback causes Windows to silently unhook the application.

---

## MVP Definition

### Launch With (v1.2)

This is the complete feature set for the milestone — there is no MVP subset because the features are deeply interdependent.

- [ ] WH_KEYBOARD_LL hook module in Rust — foundation for everything else
- [ ] Ctrl+Win combo detection with 50ms debounce for press-order independence — core activation behavior
- [ ] Start menu suppression when Ctrl+Win combo is active — required for usability; without this the feature is unusable
- [ ] Hold-to-talk behavior on Ctrl+Win (hold to record, release to transcribe) — consistent with existing hold-to-talk UX
- [ ] Frontend hotkey capture dialog supports modifier-only combos — users must be able to configure Ctrl+Win in settings
- [ ] Fallback to RegisterHotKey API if hook installation fails — prevents complete hotkey breakage on hook failure

### Add After Validation (v1.2.x)

- [ ] Hook persistence across Win+L lock/unlock — add if users report hotkey stops working after screen lock
- [ ] Left vs. right modifier distinction in binding — add if users request it; low complexity once hook is in place
- [ ] Settings UI indicator for which hotkey backend is active (hook vs. RegisterHotKey fallback) — add if fallback path is triggered in the wild

### Future Consideration (v2+)

- [ ] Double-tap modifier combo for toggle mode entry — high complexity, unclear demand; defer until users request it explicitly
- [ ] Additional modifier-only combos (e.g., double-Ctrl, Shift+Win) — defer; Ctrl+Win covers the stated use case

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| WH_KEYBOARD_LL hook module | HIGH | HIGH | P1 |
| Ctrl+Win combo detection with debounce | HIGH | MEDIUM | P1 |
| Start menu suppression | HIGH | HIGH | P1 |
| Hold-to-talk on modifier-only combo | HIGH | LOW | P1 |
| Frontend modifier-only combo capture | HIGH | MEDIUM | P1 |
| Fallback to RegisterHotKey on hook failure | MEDIUM | MEDIUM | P1 |
| Hook persistence across lock/unlock | MEDIUM | MEDIUM | P2 |
| Left vs. right modifier distinction | LOW | LOW | P2 |
| Double-tap toggle mode on modifier combo | MEDIUM | HIGH | P3 |

**Priority key:**
- P1: Required for v1.2 to ship
- P2: Add after v1.2 if reported as a gap
- P3: Future consideration only

---

## Competitor Feature Analysis

How competing tools handle modifier-only hotkey activation:

| Feature | Wispr Flow (Windows) | Windows Speech Recognition | Dragon NaturallySpeaking | This Project (v1.2) |
|---------|---------------------|---------------------------|--------------------------|---------------------|
| Default activation hotkey | Ctrl+Win (hold-to-talk) | Ctrl+Win (toggle) | Numpad + / - | Configurable; Ctrl+Win as option |
| Hotkey type | Modifier-only combo | Modifier-only combo | Single non-modifier key | Modifier-only OR standard (user choice) |
| Activation model | Hold = record, release = transcribe | Toggle (press once = start, press again = stop) | Toggle (Numpad+ = on, Numpad- = off) | Hold-to-talk (existing behavior, now with modifier-only trigger) |
| Start menu suppressed | Yes | Yes (built into OS) | N/A (no Win key used) | Yes (via WH_KEYBOARD_LL) |
| Key order sensitivity | Not exposed to users (handled internally) | Not exposed | N/A | Handled via 50ms debounce |
| Double-tap toggle mode | Yes (documented) | N/A | N/A | Future consideration |
| Fallback if hotkey unavailable | Not documented | N/A (OS-level) | Alternative hotkeys in settings | Falls back to RegisterHotKey API |
| Configuration UI for modifier-only | Yes (hotkey picker accepts Ctrl+Win) | Not configurable | Hot Keys tab in Options dialog | Settings hotkey capture dialog (modified) |

### Key Insight: Wispr Flow Is the Reference Implementation

Wispr Flow on Windows uses Ctrl+Win as its default hold-to-talk hotkey. This is the closest direct competitor for this feature. The expected UX is:

1. Press and hold Ctrl+Win
2. Hear/see activation signal (Wispr Flow: audio ping + white bars; VoiceType: pill overlay transitions to recording state)
3. Speak
4. Release Ctrl+Win
5. Text is transcribed and injected at cursor

If a user taps instead of holds, Wispr Flow shows a toast notification ("Hold down [key], then speak"). VoiceType should do the same — the existing hold-to-talk mode already handles this.

Windows Speech Recognition also uses Ctrl+Win but as a toggle. Wispr Flow's hold-to-talk model is the better UX for short dictations, which is the primary use case here.

---

## Sources

- [Wispr Flow — Starting Your First Dictation](https://docs.wisprflow.ai/articles/6409258247-starting-your-first-dictation) — Ctrl+Win as default PC hotkey, hold-to-talk model, double-tap for hands-free (HIGH confidence — official docs)
- [Wispr Flow — Improved Hotkey Selection Changelog](https://roadmap.wisprflow.ai/changelog/fire-improved-hotkey-selection) — hotkey UX simplification (MEDIUM confidence)
- [Microsoft Support — Windows Speech Recognition Commands](https://support.microsoft.com/en-us/windows/windows-speech-recognition-commands-9d25ef36-994d-f367-a81a-a326160128c7) — Ctrl+Win as WSR activation toggle (HIGH confidence — official docs)
- [Microsoft Learn — LowLevelKeyboardProc callback](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelkeyboardproc) — WH_KEYBOARD_LL API, KBDLLHOOKSTRUCT, WM_KEYDOWN/WM_KEYUP (HIGH confidence — official API docs)
- [Microsoft Learn — KBDLLHOOKSTRUCT](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-kbdllhookstruct) — vkCode, scanCode, flags including LLKHF_INJECTED (HIGH confidence — official API docs)
- [Microsoft Learn — Blocking Windows Hotkeys in an Application](https://learn.microsoft.com/en-us/answers/questions/1286619/blocking-windows-hotkeys-in-an-application) — hook-based suppression approach (MEDIUM confidence)
- [AutoHotkey Community — Prevent Win from opening Start menu](https://www.autohotkey.com/boards/viewtopic.php?t=101812) — unassigned virtual key technique (vkE8) for Start menu suppression (MEDIUM confidence — community-verified workaround)
- [SuperWhisper Keyboard Shortcuts](https://superwhisper.com/docs/get-started/settings-shortcuts) — single-modifier key support, push-to-talk + toggle dual behavior (MEDIUM confidence — official docs, macOS only)
- [Nuance Dragon — Hot Keys Options Dialog](https://www.nuance.com/products/help/dragon/dragon-for-pc/enx/professionalgroup/main/Content/DialogBoxes/options/options_dialog_hotkeys_tab.htm) — Dragon hotkey configuration approach (MEDIUM confidence — official docs)

---
*Feature research for: Modifier-only hotkey (Ctrl+Win) activation — v1.2 Keyboard Hook milestone*
*Researched: 2026-03-02*
