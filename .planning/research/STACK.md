# Stack Research

**Domain:** Windows low-level keyboard hook for modifier-only global hotkey (Ctrl+Win) — v1.2 milestone addendum
**Researched:** 2026-03-02
**Confidence:** HIGH (Win32 API documented by Microsoft; windows crate features verified via win-hotkeys dependency graph; Tauri fix confirmed closed July 2025)

---

## Scope

This is an additive milestone document covering only what changes for v1.2. The existing stack (Tauri 2.0, React/TS, windows v0.58 with `Win32_Graphics_Dxgi`, tauri-plugin-global-shortcut) is validated and not re-researched here.

**Previous stack research (full project):** `.planning/research/STACK.md` as of 2026-02-27.

---

## The Core Problem

`RegisterHotKey` (used by tauri-plugin-global-shortcut under the hood) cannot register modifier-only combinations. It requires a non-modifier virtual key as the trigger. `WH_KEYBOARD_LL` (low-level keyboard hook via `SetWindowsHookExW`) intercepts every key event before the system processes it, enabling modifier-only detection with a custom state machine.

---

## Recommended Stack Changes

### 1. windows crate: Add 3 feature flags (no version bump)

The existing dependency:
```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = ["Win32_Graphics_Dxgi"] }
```

Must become:
```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = [
    "Win32_Graphics_Dxgi",
    "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging",
    "Win32_UI_Input_KeyboardAndMouse",
] }
```

| Added Feature | Provides |
|---------------|----------|
| `Win32_UI_WindowsAndMessaging` | `SetWindowsHookExW`, `CallNextHookEx`, `UnhookWindowsHookEx`, `GetMessageW`, `TranslateMessage`, `DispatchMessageW`, `WH_KEYBOARD_LL`, `KBDLLHOOKSTRUCT`, `WM_KEYDOWN`, `WM_KEYUP`, `MSG` |
| `Win32_UI_Input_KeyboardAndMouse` | `keybd_event`, `KEYEVENTF_KEYUP`, `VK_LWIN`, `VK_RWIN`, `VK_LCONTROL`, `VK_RCONTROL` virtual key constants |
| `Win32_Foundation` | `LRESULT`, `WPARAM`, `LPARAM`, `HHOOK`, `HINSTANCE`, `BOOL` — needed alongside UI features |

**No version bump to 0.59/0.60 required.** win-hotkeys 0.5.1 uses windows 0.60 with these exact same three features — confirming the feature names are stable across 0.58–0.60. The project must NOT bump the windows version without auditing existing `Win32_Graphics_Dxgi` usage for API compatibility breaks.

### 2. Tauri Builder: Add device_event_filter

A confirmed Tauri 2.0 defect (issue #13919, closed July 2025): when the Tauri window is focused, WH_KEYBOARD_LL hooks fail to capture system keys (Win key, Alt+Tab, Ctrl+Shift+Esc). The fix is one builder call in `lib.rs`:

```rust
tauri::Builder::default()
    .device_event_filter(tauri::DeviceEventFilter::Always)
    // ... existing plugins and setup
```

This adjusts how tao (Tauri's windowing layer) dispatches device events. Without it, the Win key will be swallowed when the Tauri overlay is in focus.

### 3. No new Cargo dependencies

The two additions above (feature flags + one builder call) are the complete stack change. No new crates.

---

## Why Not win-hotkeys

`win-hotkeys` v0.5.1 was the first candidate evaluated. It is disqualified:

1. **Version conflict:** win-hotkeys 0.5.1 depends on `windows = "0.60"`. The project pins `windows = "0.58"`. Cargo cannot resolve two conflicting minor versions of the same crate; one must win, and bumping to 0.60 requires auditing all existing Win32 API calls.

2. **No modifier-only support:** The API signature is `register_hotkey(trigger_key: VKey, modifiers: &[VKey], callback: Fn())`. There is no way to register Ctrl+Win without a third trigger key. This is not a gap that can be worked around — it is a fundamental API constraint.

3. **Unnecessary abstraction:** The custom hook module is ~80–100 lines of Rust using APIs the project already has access to via the `windows` crate. win-hotkeys adds a dependency for functionality that would be a subset of what's needed.

## Why Not rdev

`rdev` has a documented unfixed defect in Tauri 2.0 (tauri-apps/tauri discussion #7752, issue #14770): rdev stops receiving keyboard events when the Tauri application window receives focus. Mouse events continue but keyboard events are dropped. The defect is attributed to the UIPI/focus model interaction and has no confirmed fix as of March 2026.

## Why Not windows-hotkeys (dnlmlr)

`windows-hotkeys` uses `RegisterHotKey` internally (same as tauri-plugin-global-shortcut), not WH_KEYBOARD_LL. It has the identical modifier-only limitation.

---

## Implementation Notes for the Hook Module

### Thread Requirement (Critical)

From the Microsoft LowLevelKeyboardProc spec: "This hook is called in the context of the thread that installed it. The call is made by sending a message to the thread that installed the hook. Therefore, **the thread that installed the hook must have a message loop**."

The hook must be installed on a dedicated `std::thread` that runs `GetMessageW` in a loop. Do NOT install it on the Tauri main thread — Tauri/tao already owns the main thread's message pump, and installing a second `GetMessageW` loop there interferes with the existing pump.

```rust
// Skeleton — actual implementation lives in src-tauri/src/keyboard_hook.rs
std::thread::spawn(move || {
    let hook = unsafe {
        SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), None, 0)
            .expect("SetWindowsHookExW failed")
    };

    let mut msg = MSG::default();
    unsafe {
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        UnhookWindowsHookEx(hook).ok();
    }
});
```

### Modifier-Only State Machine

The hook receives individual KEYDOWN/KEYUP events. For Ctrl+Win, press order is not guaranteed. The state machine must be order-independent:

```
KEYDOWN VK_LCONTROL or VK_RCONTROL → ctrl_down = true
KEYDOWN VK_LWIN or VK_RWIN         → win_down = true
KEYDOWN any other key (while both held) → dirty = true (abort combo)

KEYUP of either modifier → check if both were held and !dirty
  → if yes AND elapsed since first modifier down <= 50ms debounce window: fire trigger
  → clear state

KEYUP releases both → clear all state
```

State is stored in atomics or a `Mutex<HookState>` accessible from the static callback via a thread-local or global `Arc`.

### Win Key Start Menu Suppression

Returning `1` from the hook callback for VK_LWIN/VK_RWIN KEYDOWN suppresses the keydown event. However, Windows activates the Start menu on Win key UP, not DOWN, using an internal state machine that does not depend solely on the hook chain.

The proven suppression technique (used by AutoHotkey's `#MenuMaskKey` mechanism): inject a dummy keystroke via `keybd_event` before returning from the KEYDOWN callback. The injection "breaks" the Win-key-alone sequence that triggers the Start menu.

Safe virtual key to inject: `0xE8` (documented by Microsoft as "unassigned"). Do NOT use `0x07` — on Windows 10 1909+, VK_07 opens the Windows Game Bar.

```rust
// Suppress Start menu: inject a harmless unassigned keystroke on Win KEYDOWN
unsafe {
    keybd_event(0xE8, 0, KEYEVENTF_KEYUP.0 as u32, 0);
}
return LRESULT(1); // consume the Win keydown
```

The Win KEYUP must also be consumed (return 1) to prevent any residual Start menu trigger.

### Callback Timeout Budget

Windows enforces a maximum callback execution time (1 second on Windows 10+). If the hook exceeds this, it is silently removed. The hook callback must:
- Do minimal work inline (only state updates and `keybd_event` injection)
- Communicate to the main app via an `Arc<AtomicBool>` or `std::sync::mpsc::Sender` and return immediately
- Never call async functions or block

### Interaction with tauri-plugin-global-shortcut

The LL hook and `RegisterHotKey` (used by tauri-plugin-global-shortcut) coexist independently on the same machine. However, during frontend hotkey-capture mode, the existing `unregister_hotkey` command (already implemented) should also pause the LL hook to prevent spurious trigger detection while the user is pressing key combinations to configure a new hotkey.

---

## Supporting Libraries Summary

| Library | Change | Reason |
|---------|--------|--------|
| `windows` v0.58 | Add `Win32_Foundation`, `Win32_UI_WindowsAndMessaging`, `Win32_UI_Input_KeyboardAndMouse` features | Win32 APIs for hook installation and key injection |
| `tauri` v2 | Add `.device_event_filter(tauri::DeviceEventFilter::Always)` | Fix system key capture when Tauri window focused (issue #13919) |
| `tauri-plugin-global-shortcut` v2 | No change | Kept as fallback for non-modifier hotkeys; coexists with LL hook |
| All others | No change | Audio, transcription, VAD, clipboard, tray unchanged |

---

## Alternatives Considered

| Recommended | Alternative | Why Not |
|-------------|-------------|---------|
| Direct `windows` crate + custom hook module | `win-hotkeys` v0.5.1 | Requires windows 0.60 (conflicts with 0.58 pin), no modifier-only API (requires a trigger key), ~80 LOC custom implementation is simpler |
| Direct `windows` crate + custom hook module | `rdev` crate | Keyboard events dropped when Tauri window is focused (tauri-apps/tauri #14770, unresolved March 2026) |
| Direct `windows` crate + custom hook module | `windows-hotkeys` (dnlmlr) | Uses RegisterHotKey internally, cannot register modifier-only combos |
| `VK_E8` for Start menu mask injection | `VK_07` | VK_07 opens Windows Game Bar on Windows 10 1909+ |
| State machine with `std::time::Instant` | `tokio::sleep` debounce | Hook callback is synchronous; async cannot be awaited from it; callback has 1-second max budget |
| Dedicated hook thread | Hook on Tauri main thread | Tauri/tao owns the main thread message pump; second GetMessageW loop interferes |

---

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `RegisterHotKey` for Ctrl+Win | Cannot register modifier-only combos | `WH_KEYBOARD_LL` with state machine |
| `win-hotkeys` crate | Version conflict + no modifier-only API | Direct `windows` crate |
| `rdev` crate | Drops keyboard events when Tauri window is focused | Direct `windows` crate |
| `GetAsyncKeyState` inside hook callback | Explicitly documented to be unreliable in LowLevelKeyboardProc (async state not yet updated when callback fires) | Track state manually with AtomicBool flags |
| `VK_07` injection for Start menu mask | Opens Game Bar on Windows 10 1909+ | `VK_E8` (unassigned) |
| Any blocking or async work in hook callback | Windows enforces 1-second max; exceeding it silently removes the hook | Dispatch to channel, return immediately |

---

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| `windows` 0.58 + new features | Tauri 2.x, tauri-plugin-global-shortcut 2.x | Feature flags are additive; no API breaking changes |
| `Win32_UI_WindowsAndMessaging` feature | `Win32_Foundation`, `Win32_UI_Input_KeyboardAndMouse` | These three are always co-used for keyboard hook patterns; stable across windows crate 0.48–0.62 |
| `tauri::DeviceEventFilter::Always` | Tauri 2.0+ | Available in Tauri 2.0; resolves system key capture when window focused |

---

## Sources

- [LowLevelKeyboardProc — Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelkeyboardproc) — Thread message loop requirement, callback timeout (1s on Win10+), return 1 for suppression — HIGH confidence
- [Disabling Shortcut Keys in Games — Microsoft Win32 Docs](https://learn.microsoft.com/en-us/windows/win32/dxtecharts/disabling-shortcut-keys-in-games) — Canonical pattern for SetWindowsHookExW with VK_LWIN/VK_RWIN suppression — HIGH confidence
- [SetWindowsHookExW — Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexw) — API signature and threading semantics — HIGH confidence
- [win-hotkeys Cargo.toml (iholston/win-hotkeys)](https://github.com/iholston/win-hotkeys) — Confirms windows 0.60 with Win32_Foundation + Win32_UI_Input_KeyboardAndMouse + Win32_UI_WindowsAndMessaging; v0.5.1 released May 2025 — HIGH confidence (verified from source)
- [win-hotkeys docs.rs](https://docs.rs/win-hotkeys/latest/win_hotkeys/) — API requires trigger key + modifier array; no modifier-only support — HIGH confidence
- [Tauri issue #13919](https://github.com/tauri-apps/tauri/issues/13919) — WH_KEYBOARD_LL fails to capture system keys when Tauri window focused; fix: `device_event_filter(Always)`; confirmed resolved July 2025 — HIGH confidence
- [How do I access the Windows Low Level Hooks using the Windows rust crate? — Microsoft Q&A](https://learn.microsoft.com/en-us/answers/questions/1530452/how-do-i-access-the-windows-low-level-hooks-using) — Feature flag set for keyboard hooks in windows crate — MEDIUM confidence (community Q&A, consistent with win-hotkeys Cargo.toml)
- [AutoHotkey #MenuMaskKey docs](https://www.autohotkey.com/docs/v1/lib/_MenuMaskKey.htm) — VK_E8 as unassigned mask key for Start menu suppression; VK_07 now reserved for Game Bar — MEDIUM confidence (AHK docs, consistent with Windows VK allocation table)
- [Wispr Flow supported hotkeys](https://docs.wisprflow.ai/articles/2612050838-supported-unsupported-keyboard-hotkey-shortcuts) — Confirms Ctrl+Win is viable on Windows; recommended by Wispr Flow as a primary option — MEDIUM confidence (competitor app documentation)
- [Tauri rdev issue #14770](https://github.com/tauri-apps/tauri/issues/14770) — rdev drops keyboard events when Tauri window focused — HIGH confidence (confirmed open issue)

---

*Stack research for: v1.2 Ctrl+Win modifier-only hotkey via WH_KEYBOARD_LL (Tauri 2.0 Rust app, Windows)*
*Researched: 2026-03-02*
