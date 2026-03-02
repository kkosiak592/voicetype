# Pitfalls Research

**Domain:** WH_KEYBOARD_LL modifier-only hotkey integration into existing Tauri 2.0 desktop app (v1.2 Keyboard Hook milestone)
**Researched:** 2026-03-02
**Confidence:** HIGH (critical pitfalls verified against official Microsoft Win32 docs and confirmed Tauri GitHub issues; moderate pitfalls from AutoHotkey community validated patterns and Rust FFI docs)

---

## Critical Pitfalls

### Pitfall 1: Hook Callbacks Never Fire When Tauri Window Is Focused

**What goes wrong:**
The WH_KEYBOARD_LL hook installs successfully (SetWindowsHookExW returns a valid HHOOK), but when the VoiceType window has focus, the hook callback receives no keyboard events — not even Win key or Ctrl presses. The hook appears dead. The problem disappears when any other window has focus.

**Why it happens:**
Tauri 2.0 uses the `tao` windowing library, which on Windows sets a `DeviceEventFilter` that defaults to `Unfocused` — meaning the app's input processing pipeline filters out device events when its window is not the receiver. This was confirmed in tauri-apps/tauri#13919 and tauri-apps/tauri#14770. Importantly, WH_KEYBOARD_LL does not inject into another process — it is called in the context of the thread that installed it via an inter-thread message. If that thread's message pump is blocked or its event filter is consuming events, callbacks are silently swallowed.

This is particularly acute for VoiceType because:
- The hook thread is installed inside the same Tauri process
- The Tauri main window is the primary UI (settings panel) — users may have it focused when configuring the app
- Even if settings panel is not normally in focus, the first run and onboarding phases will have it focused

**How to avoid:**
Set `device_event_filter` in the Tauri AppBuilder to allow device events even when focused:

```rust
// In setup() or via AppBuilder before build()
.device_event_filter(tauri::DeviceEventFilter::Always)
```

This must be applied before installing the WH_KEYBOARD_LL hook. Install the hook on a dedicated thread that runs its own `GetMessage` / `DispatchMessage` loop — do not install on the Tauri main thread. The hook thread must be separate from the Tauri async runtime threads.

**Warning signs:**
- Hook callback fires correctly for 30 seconds (while settings window is not focused), then suddenly stops responding when user opens the settings panel
- App logs show hook installed successfully but no `WM_KEYDOWN` events appear when VoiceType window is foreground
- Hotkey works globally but fails to trigger when VoiceType is the active window

**Phase to address:** Hook installation phase (Phase 1 of v1.2). Verify before implementing any other hook logic — this is the foundation all other features rest on.

---

### Pitfall 2: Start Menu Opens on Win Key Release Despite Hook Blocking the Event

**What goes wrong:**
The hook correctly detects the Ctrl+Win combination and returns a non-zero value to suppress the events. The VoiceType hotkey fires. But after releasing the keys, the Start menu opens anyway — the WM_KEYUP for VK_LWIN was not blocked or was blocked too late.

**Why it happens:**
Windows handles the Start menu activation on the WM_KEYUP event for VK_LWIN/VK_RWIN, not WM_KEYDOWN. The sequence the OS uses: if Win key is pressed and released with no other keys in between, the Start menu activation is queued. Simply blocking the WM_KEYDOWN is not sufficient. There are two sub-problems:

1. **Direct suppression is insufficient alone**: Returning non-zero from the hook for WM_KEYUP of VK_WIN suppresses the event to the target window, but the Windows shell has already registered interest in the Win key at a level that observes the hook output.

2. **The masking technique is required**: AutoHotkey and other hook tools suppress Start menu activation by injecting a synthetic key event (VK 0xE8, which is "unassigned" per Microsoft's virtual key table) on Win key press. This "masks" the Win key as a combo-key in the OS's internal tracking state. Without the mask key, the OS concludes the Win key was pressed alone and opens the Start menu.

On Windows 11, this is more sensitive than Windows 10: the AutoHotkey community confirmed that Windows 11 changed Win key handling and the masking technique requires sending the unassigned key on both down and up transitions in some configurations.

**How to avoid:**
In the hook callback, when Ctrl+Win is recognized as the active combo:
1. Return non-zero to suppress the Win key KEYDOWN event.
2. Simultaneously send a synthetic key event for VK 0xE8 (KEYDOWN + KEYUP) via `SendInput` with `LLKHF_INJECTED` — but check the `LLKHF_INJECTED` flag in your own callback to skip re-processing your own synthetic events (see Pitfall 6).
3. Also return non-zero for VK_LWIN KEYUP.

Critical: Do not use VK 0x07 as the mask key — on Windows 10+ it opens the Xbox Game Bar.

```rust
// In hook callback, detect own synthetic events to avoid recursion:
let flags = kbdll.flags;
if flags & LLKHF_INJECTED != 0 {
    return CallNextHookEx(ptr::null_mut(), code, wparam, lparam);
}
```

**Warning signs:**
- Start menu opens briefly then closes (partial suppression — KEYDOWN blocked but not KEYUP)
- Start menu reliably opens after every successful Ctrl+Win hotkey trigger
- Behavior differs on Windows 10 vs Windows 11 test machines

**Phase to address:** Phase 1 (hook implementation) — the mask-key strategy must be designed in from the start, not bolted on after testing reveals Start menu leakage.

---

### Pitfall 3: Hook Is Silently Removed by Windows After Timeout — App Doesn't Know

**What goes wrong:**
The hook works normally for hours, then stops responding with no error. The app continues running normally (no crash, no log error), but the Ctrl+Win hotkey no longer fires. The user must restart VoiceType to restore the hotkey. This happens intermittently — more often on machines with antivirus, heavy CPU load, or slow machines.

**Why it happens:**
From the official `LowLevelKeyboardProc` documentation (verified): "The hook procedure should process a message in less time than the data entry specified in the `LowLevelHooksTimeout` value in `HKEY_CURRENT_USER\Control Panel\Desktop`." If the callback exceeds this timeout — default 300ms, capped at 1000ms on Windows 10 version 1709+ — the system passes the event to the next hook. If it times out 11 times cumulatively, **the hook is silently removed with no notification to the app**. There is no callback, no event, no return value — the HHOOK handle becomes invalid and all subsequent keystrokes bypass the callback.

In this app, the hook callback runs on the hook thread. Any blocking operation (acquiring a Mutex, sending across a channel that is full, calling into the Tauri command system) directly inside the callback can cause timeout. Given the existing Arc<Mutex<...>> patterns in lib.rs, accidentally holding a lock in the callback path is a realistic mistake.

**How to avoid:**
- The hook callback must do ONLY: read the KBDLLHOOKSTRUCT, set an AtomicBool or send to an unbounded channel (non-blocking), and return immediately. Do NOT lock mutexes, do NOT call into Tauri commands, do NOT allocate on the heap inside the callback.
- Worker logic (debounce timer, state machine evaluation) runs on a separate thread that reads from the channel.
- Implement a health-check mechanism: a periodic timer (e.g., every 5 seconds) attempts a synthetic key event and verifies the hook callback fires. If the hook is dead, reinstall it.

```rust
// The entire callback should be near this simple:
unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let kb = *(lparam as *const KBDLLHOOKSTRUCT);
        let _ = HOOK_SENDER.send(HookEvent { vk: kb.vkCode, flags: kb.flags, wp: wparam });
    }
    CallNextHookEx(ptr::null_mut(), code, wparam, lparam)
}
```

**Warning signs:**
- Hotkey stops working without any restart or settings change
- Issue reproduces more reliably when antivirus is scanning (CPU spikes) or system is under heavy load
- Adding a `log::debug!()` call inside the hook callback causes the hook to die faster (disk I/O in callback)

**Phase to address:** Phase 1 (hook architecture). The channel-based non-blocking callback design must be the initial design — it cannot be refactored in after discovering timeouts.

---

### Pitfall 4: Rust Panic Inside `extern "system"` Hook Callback Causes Undefined Behavior

**What goes wrong:**
A Rust panic occurs inside the hook callback (e.g., a slice index out of bounds, an unwrap() on a None, or an assertion). The process exhibits undefined behavior — in the best case it aborts immediately; in the worst case it silently corrupts memory and the app continues running in a broken state. The specific undefined behavior is platform-dependent.

**Why it happens:**
From the Rust Nomicon and RFC 2945: a Rust panic that unwinds through an `extern "C"` (or `extern "system"`) FFI boundary is undefined behavior per the Rust specification. The Windows `LowLevelKeyboardProc` callback is called by the OS from C code — it is precisely an FFI boundary. Even if `panic = "abort"` is set in Cargo.toml (which would catch this for the common case), it is not set for the existing project (which uses default `panic = "unwind"`). An unwinding panic crossing this boundary may corrupt the hook chain for the entire system, not just this app.

**How to avoid:**
Wrap the entire callback body in `std::panic::catch_unwind`:

```rust
unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let result = std::panic::catch_unwind(|| {
        if code >= 0 {
            let kb = *(lparam as *const KBDLLHOOKSTRUCT);
            let _ = HOOK_SENDER.try_send(HookEvent { vk: kb.vkCode, flags: kb.flags, wp: wparam });
        }
        CallNextHookEx(ptr::null_mut(), code, wparam, lparam)
    });
    match result {
        Ok(r) => r,
        Err(_) => CallNextHookEx(ptr::null_mut(), code, wparam, lparam), // safe fallback
    }
}
```

Alternatively, keep the callback so minimal (just one AtomicStore and one return) that no panic is possible.

Never use `.unwrap()`, `.expect()`, index operations, or any fallible operation inside the hook callback body.

**Warning signs:**
- App crashes with no Rust backtrace during hotkey use (OS-level crash rather than Rust panic)
- The crash happens intermittently and is hard to reproduce under debugger
- Crash only occurs when the keyboard is used rapidly or in specific key sequences

**Phase to address:** Phase 1 (hook implementation). Code review gate: the hook callback must be reviewed for any panic-capable operations before merging.

---

### Pitfall 5: Coexistence of tauri-plugin-global-shortcut (RegisterHotKey) and WH_KEYBOARD_LL Causes Double-Firing or Deadlock

**What goes wrong:**
The existing app uses `tauri-plugin-global-shortcut` (which wraps the `global-hotkey` crate, which uses Win32 `RegisterHotKey`). The new v1.2 milestone adds a WH_KEYBOARD_LL hook in the same process. These two systems both observe keyboard events at the global level. Depending on implementation, two failure modes appear:

1. **Double-firing**: A standard hotkey (e.g., Ctrl+F9) registered via RegisterHotKey also passes through the WH_KEYBOARD_LL hook. If the hook logic is not explicitly scoped to only handle Ctrl+Win combinations, it may also trigger on Ctrl+F9, causing double-execution of the hotkey handler.

2. **Cross-thread message loop interference**: Both systems install into the same process's message loop. If the hook thread and the RegisterHotKey thread share state (Arc<Mutex<...>>) and both try to acquire the same lock in response to the same keypress event, a deadlock can occur.

**Why it happens:**
`RegisterHotKey` works through the WM_HOTKEY message, which is delivered to the thread that registered it. `WH_KEYBOARD_LL` operates at a lower level and fires before WM_HOTKEY is generated. They are independent mechanisms that both observe the same keystrokes. The existing `tauri-plugin-global-shortcut` state machine in lib.rs (handle_shortcut, rebind_hotkey, etc.) was not designed to coexist with a parallel low-level hook. During the transition phase, both systems will be active simultaneously (fallback path if hook fails to install).

**How to avoid:**
- In the WH_KEYBOARD_LL callback, immediately check if the key combination involves VK_LWIN or VK_RWIN. Only process combinations that include a Win modifier — pass everything else through via `CallNextHookEx` with no state changes. This scopes the hook to only the Ctrl+Win use case.
- Define a clear state ownership boundary: the WH_KEYBOARD_LL handler owns the `is_recording` AtomicBool when the hook is active; the RegisterHotKey handler owns it when hook is not installed. Never let both read-modify-write the same AtomicBool concurrently.
- During the transition (hook active), unregister the overlapping standard hotkey from RegisterHotKey to avoid double-registration.
- Lock acquisition order must be consistent across both callback paths to prevent deadlock.

**Warning signs:**
- Transcription starts twice in rapid succession after a single hotkey press
- App deadlocks under specific timing conditions when both a standard key and Ctrl+Win are pressed close together
- Toggling the hotkey in settings while the hook is active causes a hang

**Phase to address:** Phase 1 (hook installation) and Phase 2 (fallback/coexistence logic). The boundary between the two systems must be documented as an invariant.

---

## Moderate Pitfalls

### Pitfall 6: Synthetic Key Injection (SendInput for Start Menu Suppression) Recursively Re-Enters the Hook

**What goes wrong:**
The hook callback calls `SendInput` to inject the VK 0xE8 mask key. The hook itself is called again for this injected event (because WH_KEYBOARD_LL is called for all keyboard input including synthetic). This causes infinite recursion — the hook callback calls SendInput, which triggers the hook, which calls SendInput again — until the stack overflows or the timeout fires.

**Why it happens:**
WH_KEYBOARD_LL receives both physical and synthetic keyboard events. Any call to `SendInput`, `keybd_event`, or `PostMessage` with key input from within the hook callback will re-enter the callback. The official docs confirm the hook fires for `keybd_event`-originated input.

**How to avoid:**
Check the `LLKHF_INJECTED` flag (bit 4 of `KBDLLHOOKSTRUCT.flags`) at the start of every callback invocation. If set, the event is synthetic — pass it through without any processing or re-injection:

```rust
if kb.flags & 0x10 != 0 { // LLKHF_INJECTED
    return CallNextHookEx(ptr::null_mut(), code, wparam, lparam);
}
```

This is the standard guard used by AutoHotkey and all hook-based tools.

**Warning signs:**
- Stack overflow crash shortly after first hotkey press
- Exponential CPU spike when hotkey is pressed (hook firing thousands of times per second)
- App freeze immediately after first successful Ctrl+Win detection

**Phase to address:** Phase 1 (hook implementation). Must be the first check in the callback, before any other logic.

---

### Pitfall 7: Left vs. Right Modifier Ambiguity — VK_CONTROL Fires Instead of VK_LCONTROL

**What goes wrong:**
The hook receives `vkCode == VK_CONTROL (0x11)` instead of `VK_LCONTROL (0xA2)` or `VK_RCONTROL (0xA3)`. The state machine that tracks "is Ctrl currently pressed?" either never detects the press (if it only watches VK_LCONTROL) or incorrectly identifies which Ctrl key was pressed (affects user UX for left-vs-right hotkey preferences).

Similarly, VK_LWIN (0x5B) and VK_RWIN (0x5C) are distinct — the user may want only left-Win to trigger (to not interfere with right-Win+L for lock screen).

**Why it happens:**
Windows keyboards send different virtual key codes depending on the physical key, but older hardware, software keyboard emulators, remote desktop sessions, and the on-screen keyboard may send the generic `VK_CONTROL` rather than the side-specific codes. The KBDLLHOOKSTRUCT provides `scanCode` and the `LLKHF_EXTENDED` flag in `flags` (bit 0), which can be used with `MapVirtualKeyW(scanCode, MAPVK_VSC_TO_VK_EX)` to disambiguate.

**How to avoid:**
Track all three VK codes for each modifier: VK_CONTROL, VK_LCONTROL, and VK_RCONTROL. Treat all three as "Ctrl pressed." For the Win key, track VK_LWIN and VK_RWIN separately. Use the `LLKHF_EXTENDED` flag to distinguish extended (right-side) keys when vkCode is the generic form.

Design the state machine so it does not assume left-only modifiers. If the product decision is "only left Ctrl + left Win", enforce it by also checking the scanCode / extended flag, not just vkCode.

**Warning signs:**
- Ctrl+Win combination detected on desktop PC but not on remote desktop session
- Right-Ctrl + Win triggers the hotkey when it should not (or vice versa)
- On-screen keyboard or accessibility tools cause unintended hotkey firing

**Phase to address:** Phase 1 (hook state machine). The VK code enumeration must cover all three variants from the start.

---

### Pitfall 8: Modifier Key State Desync When App Is Backgrounded Mid-Press

**What goes wrong:**
The user holds Ctrl, switches to another window via Alt+Tab, then releases Ctrl. The hook receives the Ctrl KEYDOWN but misses the KEYUP (or vice versa depending on timing). The internal `ctrl_down: bool` state is now permanently stuck as `true`. Subsequently, pressing any key is misidentified as "Ctrl held" — the hotkey fires unexpectedly or never fires because the combo is perceived as already active.

**Why it happens:**
WH_KEYBOARD_LL is a global hook and does receive events when the app is not focused. However, if the hook thread's message pump is blocked at the moment the KEYUP event arrives (e.g., during a transcription pipeline operation), the event is delivered after the timeout and may be dropped (see Pitfall 3). The `GetAsyncKeyState` function cannot be called from inside the hook callback (the official docs explicitly warn: "the callback function is called before the asynchronous state of the key is updated"). There is also no OS-provided "you missed some key events" notification.

**How to avoid:**
- On WM_HOTKEY or application focus-change events (WM_ACTIVATEAPP with wParam=FALSE), reset all modifier state flags to their actual hardware state using `GetKeyState` called from the main thread (not inside the hook callback).
- Implement a periodic modifier-state reconciliation (every 100ms) that calls `GetKeyState(VK_CONTROL)` and `GetKeyState(VK_LWIN)` on the hook thread and corrects the internal AtomicBool state if it diverges.
- Design the state machine to recover from stuck-modifier state: if `ctrl_down` has been `true` for more than 2 seconds without a corresponding Win key press, reset it.

**Warning signs:**
- After Alt+Tabbing away and back, the hotkey fires on the very next keypress with no modifier held
- Holding Ctrl and quickly Alt+Tabbing causes a "phantom" hotkey trigger
- User reports the app "randomly starts recording" without pressing the hotkey

**Phase to address:** Phase 1 (state machine design) and Phase 2 (focus change integration testing).

---

### Pitfall 9: Hook Thread Has No Message Pump — Hook Callbacks Never Fire

**What goes wrong:**
The hook is installed on a standard Rust `std::thread::spawn` thread. The HHOOK is valid, but keyboard callbacks never fire. No events reach the hook callback at all.

**Why it happens:**
The official `LowLevelKeyboardProc` documentation states explicitly: "the thread that installed the hook must have a message loop." The Windows hook subsystem delivers callbacks to the installing thread by posting a special message to its message queue. If the thread has no Win32 message loop (no `GetMessage` / `PeekMessage` / `DispatchMessage` pump), those messages are never processed and the callback is never invoked. A standard Rust thread (`std::thread::spawn`) has no Win32 message loop. The Tauri main thread has a message loop (WebView2 runs one), but it is not safe to install the hook on it.

**How to avoid:**
The hook thread must run an explicit Win32 message loop using `windows-rs` or `winapi`:

```rust
std::thread::spawn(move || {
    let hhook = unsafe {
        SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), None, 0)
    }.expect("hook install failed");

    // Required: Win32 message loop on this thread
    let mut msg = MSG::default();
    loop {
        match unsafe { GetMessageW(&mut msg, None, 0, 0) } {
            BOOL(0) | BOOL(-1) => break, // WM_QUIT or error
            _ => unsafe {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
    unsafe { UnhookWindowsHookEx(hhook) };
});
```

**Warning signs:**
- `SetWindowsHookExW` returns success but the callback function is never called
- Inserting a `println!` at the top of the callback confirms it is never reached
- Hook works when installed on the Tauri main thread (which has a loop) but not on a spawned thread

**Phase to address:** Phase 1. This is a prerequisite that must be verified with a minimal proof-of-concept before building the full state machine.

---

### Pitfall 10: Windows Defender Escalates False Positive Due to WH_KEYBOARD_LL

**What goes wrong:**
The WH_KEYBOARD_LL addition causes Windows Defender to reclassify the VoiceType binary from "low suspicion" (due to existing `SendInput` for clipboard paste) to "high suspicion" (keyboard snooping + input injection). The binary is quarantined or users receive a SmartScreen warning that was not present in v1.0/v1.1.

**Why it happens:**
The existing app already uses `SendInput` for Ctrl+V injection (clipboard paste), which is a known malware signal. `SetWindowsHookExW` with `WH_KEYBOARD_LL` and `dwThreadId = 0` (global hook) is a primary keylogger API pattern. The combination of global keyboard hook + key injection + running at startup = the exact signature of credential-stealing malware. Microsoft's 2024-2025 expanded keylogger protection in Defender (announced on Windows IT Pro Blog, September 2024) increased the ML classifier sensitivity for these API combinations.

The fact that the hook code is in Rust rather than C/C++ provides no protection — Defender classifies on binary behavior, not source language.

**How to avoid:**
- Code signing with an OV or EV certificate (already recommended in the v1.0 PITFALLS.md) is the primary mitigation. Signed binaries receive substantially higher trust and are far less likely to trigger heuristic ML classifiers.
- Submit the signed v1.2 binary to Microsoft Security Intelligence (MSCI) for review before public distribution.
- Document the behavior in the app's About or README: "VoiceType installs a global keyboard hook to detect Ctrl+Win. This is required for modifier-only hotkey support. The hook does not log or transmit keystrokes."
- For the personal use / friend distribution scenario (no OV cert): document the Defender exclusion path and be aware that the exclusion applies per binary hash — each new build requires the same process.

**Warning signs:**
- V1.0/v1.1 passed Defender silently, but v1.2 triggers a SmartScreen warning on first run
- Windows Event Log shows Defender event 1116 (malware detection) or 1117 (remediation) after install
- VirusTotal scan shows 1-3 engine detections on the new binary (was 0 on v1.1)

**Phase to address:** Distribution testing phase. Run the v1.2 binary through VirusTotal before any distribution — treat any new detection vs. v1.1 as a blocking issue.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Install hook on Tauri main thread instead of dedicated hook thread | Avoids writing a Win32 message loop | Main thread blocked by hook; Tauri UI freezes during any hook processing delay; WM_QUIT handling breaks | Never — dedicated thread is mandatory |
| Skip `LLKHF_INJECTED` guard in callback | Simpler callback code | Infinite recursion when sending mask-key synthetic events; immediate stack overflow | Never |
| Use `GetAsyncKeyState` inside hook callback to verify modifier state | Simpler "confirm key state" logic | Undefined per official docs — async state not updated yet when callback fires; returns stale data | Never |
| Skip hook health-check (let dead hook stay dead) | Less code to write | Hotkey silently stops working; user must restart app | Only acceptable for MVP with a visible "hook is inactive" status indicator |
| Skip debounce, detect exact Ctrl-then-Win ordering | Faster initial implementation | Fails for users who press Win slightly before Ctrl (fast typists, human timing variance); brittle | Only for initial proof-of-concept — must add debounce before beta |
| Keep RegisterHotKey active alongside WH_KEYBOARD_LL permanently | Simpler fallback logic | Double-firing risk on overlapping hotkeys; complex state ownership | Only acceptable if non-overlapping hotkeys are guaranteed (Ctrl+Win through hook, all other combos via RegisterHotKey) |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| Tauri + WH_KEYBOARD_LL | Install hook without setting DeviceEventFilter | Set `DeviceEventFilter::Always` before building the app; install hook on separate thread |
| tauri-plugin-global-shortcut + WH_KEYBOARD_LL | Assume they are independent; let both observe the same keys | Scope WH_KEYBOARD_LL strictly to Win-key combos; unregister the overlapping RegisterHotKey when hook is active |
| `SendInput` from hook callback (mask key) | Call SendInput synchronously inside the callback | Call from the worker thread that receives the hook event channel; never from inside the callback itself |
| Rust `std::thread::spawn` + WH_KEYBOARD_LL | Spawn a thread without a Win32 message loop | The hook thread must run `GetMessage` / `DispatchMessage` loop; this is non-negotiable per Win32 docs |
| `Arc<Mutex<...>>` state shared with hook thread | Lock the mutex inside the hook callback | Use `AtomicBool` for all state communicated from within the callback; move all mutex-guarded operations to the worker thread |
| `UnhookWindowsHookEx` at app shutdown | Call from a different thread than where the hook was installed | Call `UnhookWindowsHookEx` from the hook thread itself, then send WM_QUIT to exit the message loop |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Blocking channel send in hook callback | Hook timeout fires; hook silently removed after 11 timeouts | Use `try_send` on an unbounded channel; never block in callback | Immediately if transcription pipeline is active and channel is full |
| Debounce timer using `thread::sleep` inside hook thread | Sleep blocks message pump; hook stops receiving events during sleep | Run debounce timer on the worker thread, not the hook thread; hook thread must only pump messages | Every debounce window while sleep is active |
| Re-evaluating modifier combo state on every key event including non-modifiers | Unnecessary work; risk of timeout on slow machines | State machine only evaluates when vkCode is in {VK_CONTROL, VK_LCONTROL, VK_RCONTROL, VK_LWIN, VK_RWIN}; all other keys bypass the logic | From first non-modifier keypress |
| Heap allocation in hook callback (Box, Vec, String formatting) | Memory allocator contention; jitter causing timeout | Pre-allocate all structures on the worker thread; callback sends only primitive values (u32 vkCode, u32 flags, usize wparam) via `AtomicPtr` or fixed-size channel | Under memory pressure or with jemalloc contention |

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Logging `dwExtraInfo` from KBDLLHOOKSTRUCT to a log file | Other hook-using apps can embed arbitrary data in `dwExtraInfo`; if logged verbosely, creates a potential data exfiltration channel in log files | Never log raw KBDLLHOOKSTRUCT fields other than vkCode and flags; ignore dwExtraInfo entirely |
| Using a static mutable HHOOK without synchronization | Data race if hook is reinstalled concurrently | Store HHOOK in a Mutex or use a thread-local; the hook thread owns its own HHOOK exclusively |
| Not validating `nCode` parameter before processing | Undefined behavior if nCode < 0 and callback processes the event | First line of callback: `if code < 0 { return CallNextHookEx(...) }` — non-negotiable per Win32 spec |

---

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| No indication that hook is installed and healthy | Users don't know if the Ctrl+Win hotkey is active; press it and nothing happens | Show a hook status indicator in the system tray tooltip ("Hook: Active" vs "Hook: Inactive — using standard hotkey") |
| Ctrl+Win fires while user is typing Ctrl+Z (undo) | Users who type fast hit Ctrl+Win accidentally when reaching for Ctrl+Z; app starts recording unexpectedly | Require both Ctrl and Win to be held for ≥50ms before triggering; do not trigger on sub-50ms press sequences |
| Start menu flashes open for 1-2 frames before suppression | Jarring visual glitch; users think app is broken | The mask-key SendInput must be dispatched before the Win key KEYUP event reaches the shell; timing matters — see Pitfall 2 |
| Frontend hotkey-capture UI still uses old tauri-plugin-global-shortcut capture mode | Settings panel cannot capture "Ctrl+Win" as a hotkey choice — modifier-only combos are not representable in the standard hotkey string format | Build a separate capture mode that explicitly shows modifier-only combos as valid options; store as a distinct format ("Ctrl+Win") separate from standard hotkey strings |
| No fallback notification when hook install fails | App launches silently without a working Ctrl+Win hotkey; user confused | On hook install failure, revert to standard hotkey mode and show a tray notification: "Modifier-only hotkey unavailable. Using standard hotkey [X] instead." |

---

## "Looks Done But Isn't" Checklist

- [ ] **Hook with Tauri window focused:** Open VoiceType settings panel, put it in focus, press Ctrl+Win. Verify the hotkey fires. If it doesn't, `DeviceEventFilter` is not set correctly.
- [ ] **Start menu suppression on Windows 11:** On a Windows 11 machine (different from dev machine if dev is Win10), press Ctrl+Win — verify Start menu does NOT open. Test Win key alone after — verify Start menu DOES open (hotkey not over-suppressing).
- [ ] **Hook recovery after timeout:** Simulate a slow hook by adding a 500ms sleep in the callback (debug build only). Verify the hook-health monitor detects the drop and reinstalls. Remove the sleep before shipping.
- [ ] **Left vs. right modifier coverage:** Test Ctrl+Win with right Ctrl. Test with right Win. Verify behavior is as designed (either both work or only left works, per product decision — not "one works and the other is undefined").
- [ ] **App shutdown with hook active:** Close VoiceType while Ctrl+Win hook is installed. Verify no lingering hook ghost process in Process Monitor (hook should be unregistered in Drop impl).
- [ ] **Coexistence during transition phase:** If the fallback is "use standard hotkey," verify that switching between hook mode and RegisterHotKey mode does not double-register or leave both active simultaneously.
- [ ] **Antivirus scan of v1.2 binary:** Run through VirusTotal before any distribution. Compare detection count against v1.1 baseline. Any new detections are a blocking issue.
- [ ] **Debounce correctness:** Press Win before Ctrl (reversed order, fast) — verify the hotkey still fires. Press Ctrl, hold 1 second, then press Win — verify the hotkey still fires (slow press is valid). Press Ctrl+Win and immediately release Ctrl before Win — verify hotkey fires but does not stay in "active" state.

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| DeviceEventFilter not set (hook dead when Tauri focused) | LOW | Add one line to AppBuilder; rebuild and test |
| Start menu suppression missing on Windows 11 | MEDIUM | Implement mask-key SendInput on the hook worker thread; requires testing on both Win10 and Win11 |
| Hook silently removed (no health check) | MEDIUM | Add a background health-check timer; requires hook reinstall logic and state reset |
| Panic crossing FFI boundary (callback crash) | HIGH | Add `catch_unwind` wrapper around entire callback; requires full regression testing of the hook path |
| Double-firing from coexisting RegisterHotKey + WH_KEYBOARD_LL | MEDIUM | Audit which keys are registered where; unregister the overlapping key from one system |
| Defender flags v1.2 binary | HIGH | Same recovery as v1.0 pitfall: purchase OV cert, sign binary, re-release; short-term: per-machine exclusion |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Hook dead when Tauri window focused (#1) | Phase 1: Hook installation | Test: open settings window, put in focus, press Ctrl+Win — must fire |
| Start menu opens despite suppression (#2) | Phase 1: Hook implementation | Test on Windows 10 AND Windows 11; Win key alone must still work after fix |
| Hook silently removed after timeout (#3) | Phase 1: Callback architecture | Verify callback returns in <5ms; add logging that shows callback return time |
| Panic across FFI boundary (#4) | Phase 1: Code review gate | PR review checklist item: zero fallible operations in callback body |
| RegisterHotKey + WH_KEYBOARD_LL coexistence (#5) | Phase 1 + Phase 2 (fallback) | Test: trigger Ctrl+Win rapidly 20 times; verify exactly 20 recording sessions, not 40 |
| SendInput recursive re-entry (#6) | Phase 1: LLKHF_INJECTED guard | Verify CPU usage does not spike on first Ctrl+Win press |
| Left vs. right modifier ambiguity (#7) | Phase 1: State machine | Test right-Ctrl + Win, left-Ctrl + right-Win, and right-Ctrl + right-Win |
| Modifier state desync on focus loss (#8) | Phase 2: Integration testing | Alt-Tab away mid-Ctrl-hold; verify no phantom trigger on return |
| Missing Win32 message loop (#9) | Phase 1: Thread setup | Verify callback fires in smoke test before any other feature work |
| Defender false positive escalation (#10) | Distribution testing | VirusTotal check of signed v1.2 binary before release |

---

## Sources

- [LowLevelKeyboardProc — Microsoft Docs (verified 2025-07-14)](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelkeyboardproc) — HIGH confidence
- [KBDLLHOOKSTRUCT — Microsoft Docs](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-kbdllhookstruct) — HIGH confidence
- [SetWindowsHookExA — Microsoft Docs](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexa) — HIGH confidence
- [Tauri issue #13919: WH_KEYBOARD_LL not capturing system keys when Tauri window focused](https://github.com/tauri-apps/tauri/issues/13919) — HIGH confidence (confirmed resolved via DeviceEventFilter)
- [Tauri issue #14770: rdev keyboard events break when Tauri window focused](https://github.com/tauri-apps/tauri/issues/14770) — HIGH confidence (same root cause; DeviceEventFilter::Always fix confirmed)
- [AutoHotkey: Prevent Win from opening Start menu — Any Version](https://www.autohotkey.com/boards/viewtopic.php?t=101812) — MEDIUM confidence (community-validated; vkE8 mask key technique)
- [AutoHotkey: Disable left Windows key on Windows 11 with AHK 2](https://www.autohotkey.com/boards/viewtopic.php?t=96593) — MEDIUM confidence (Windows 11 behavioral differences confirmed by community)
- [AutoHotkey MenuMaskKey docs — vkE8 as unassigned mask key](https://autohotkey.com/docs/commands/_MenuMaskKey.htm) — MEDIUM confidence
- [Rust Nomicon: FFI and panics](https://doc.rust-lang.org/nomicon/ffi.html) — HIGH confidence
- [RFC 2945: C-unwind ABI](https://rust-lang.github.io/rfcs/2945-c-unwind-abi.html) — HIGH confidence
- [catch_unwind — Rust std docs](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html) — HIGH confidence
- [Raymond Chen (The Old New Thing): GetKeyState vs GetAsyncKeyState](https://devblogs.microsoft.com/oldnewthing/20041130-00/?p=37173) — HIGH confidence
- [Microsoft: Keylogging malware protection built into Windows (Sep 2024)](https://techcommunity.microsoft.com/blog/windows-itpro-blog/keylogging-malware-protection-built-into-windows/4256289) — HIGH confidence (confirms expanded Defender ML sensitivity)
- [Virtual-Key Codes — Microsoft Docs](https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes) — HIGH confidence (VK_LCONTROL=0xA2, VK_RCONTROL=0xA3, VK_LWIN=0x5B, VK_RWIN=0x5C)
- [SetWindowsHookExW in windows-rs — Microsoft GitHub](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/UI/WindowsAndMessaging/fn.SetWindowsHookExW.html) — HIGH confidence

---
*Pitfalls research for: WH_KEYBOARD_LL modifier-only hotkey integration into Tauri 2.0 (VoiceType v1.2 Keyboard Hook milestone)*
*Researched: 2026-03-02*
