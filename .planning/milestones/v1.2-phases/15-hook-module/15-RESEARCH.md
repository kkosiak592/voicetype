# Phase 15: Hook Module - Research

**Researched:** 2026-03-02
**Domain:** Win32 WH_KEYBOARD_LL keyboard hook, modifier state machine, Tauri DeviceEventFilter
**Confidence:** HIGH (Win32 hook APIs from official docs; Tauri fix confirmed by issue thread; VK_E8 mechanism from AHK docs + community; Rust crate APIs from docs.rs)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- Any Ctrl (left or right) + Any Win (left or right) qualifies as the combo
- Exact match only: Ctrl+Win with NO other modifiers held (Shift, Alt). Ctrl+Win+Shift does NOT activate. Prevents conflicts with system shortcuts like Ctrl+Win+D (virtual desktops)
- Ctrl+Win becomes the new default hotkey for fresh v1.2 installs, replacing Ctrl+Shift+Space
- Existing v1.1 users keep their saved hotkey on upgrade

### Claude's Discretion

- Left/right modifier distinction (any combination is acceptable)
- Extra keys pressed during active recording: ignore or cancel
- Release behavior: either key ends recording vs both must release
- Hook auto-install timing: always on startup vs only when modifier-only hotkey is configured
- Upgrade notification for existing users: silent, tray notification, or in-app banner
- Start menu suppression: release order handling, behavior on failed/short activation, Win+other shortcuts when Ctrl not held, behavior when app is paused
- Hook status indication: silent when working vs tray tooltip mention
- Hook failure behavior: silent fallback vs notify-and-fallback
- Settings panel backend indicator: show or hide
- Mid-session hook removal recovery: auto-reinstall, notify, or defer to v2

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| HOOK-01 | App installs WH_KEYBOARD_LL hook on a dedicated thread with Win32 GetMessage loop | SetWindowsHookExW with hmod=NULL and dwThreadId=0 on a std::thread; GetMessage loop pattern documented in official Win32 docs |
| HOOK-02 | Hook callback completes in under 5ms using only AtomicBool writes and non-blocking channel sends | LowLevelHooksTimeout default ≤1000ms but hook will be silently removed on Win7+ if it exceeds timeout; pattern: AtomicBool write + mpsc::try_send is guaranteed sub-millisecond |
| HOOK-03 | Tauri builder applies DeviceEventFilter::Always so hook fires when Tauri window is focused | Confirmed fix for tauri#13919: `.device_event_filter(tauri::DeviceEventFilter::Always)` on Builder before `.build()` |
| HOOK-04 | App cleanly uninstalls hook on shutdown via PostThreadMessageW(WM_QUIT) with no dangling hook | PostThreadMessageW(hook_thread_id, WM_QUIT, 0, 0) terminates GetMessage loop; UnhookWindowsHookEx called inside that thread before exit |
| MOD-01 | Hook detects Ctrl+Win held simultaneously and sends Pressed event to handle_shortcut() | State machine tracks VK_LCONTROL/VK_RCONTROL/VK_LWIN/VK_RWIN; combo fires when second modifier down and first is already held (within debounce window) |
| MOD-02 | Hook detects Ctrl or Win released after combo and sends Released event to handle_shortcut() | WM_KEYUP/WM_SYSKEYUP for any of the four VK codes triggers Released when combo was active |
| MOD-03 | 50ms debounce window allows either key to be pressed first without affecting detection | Combo activates as soon as second key is down if first was pressed within 50ms; use `time` field in KBDLLHOOKSTRUCT (milliseconds) for delta comparison |
| MOD-04 | Start menu is suppressed when Ctrl+Win combo is active via VK_E8 mask injection | VK_E8 (0xE8, unassigned) injected via SendInput on Win keydown when combo is active; LLKHF_INJECTED flag prevents re-entrancy; Win key returned nonzero to suppress |
| MOD-05 | Win key alone still opens Start menu when not part of Ctrl+Win combo | When Ctrl is NOT held at Win keydown, CallNextHookEx normally — no suppression |
| INT-01 | Hold-to-talk works end-to-end with Ctrl+Win (hold to record, release to transcribe) | Bridge via mpsc channel + dispatcher thread that calls handle_shortcut() with synthetic ShortcutEvent |
</phase_requirements>

---

## Summary

Phase 15 installs a `WH_KEYBOARD_LL` global keyboard hook on a dedicated OS thread. This is pure Win32 with no new crate dependencies — the `windows` v0.58 crate already in `Cargo.toml` needs three additional feature flags: `Win32_Foundation`, `Win32_UI_WindowsAndMessaging`, and `Win32_UI_Input_KeyboardAndMouse`. The hook thread must run its own `GetMessage` loop because WH_KEYBOARD_LL callbacks are delivered via the message queue of the installing thread. All work inside the callback must complete sub-millisecond: write `AtomicBool` flags and `try_send` on an mpsc channel, then return immediately. A separate dispatcher thread reads the channel and calls the existing `handle_shortcut()` function.

The critical pitfall for Tauri applications is `DeviceEventFilter`. By default, Tauri's underlying windowing library (tao) ignores device events for unfocused windows on Windows. When the Tauri window has focus, WH_KEYBOARD_LL simply does not receive events — confirmed by tauri#13919 and related issues. The fix is one line added to the Tauri builder: `.device_event_filter(tauri::DeviceEventFilter::Always)` before `.build()`. This must be applied regardless of whether the hook is active.

Start menu suppression uses the VK_E8 mask-key technique: when the Win key goes down and Ctrl is already held (or vice versa with debounce), inject a VK_E8 `SendInput` and return nonzero from the hook to suppress the Win key event. The system sees that another key intervened between Win-down and Win-up, so it does not activate the Start menu. The `LLKHF_INJECTED` flag in `KBDLLHOOKSTRUCT.flags` must be checked at the top of the callback to skip injected events and prevent an infinite loop.

**Primary recommendation:** Implement `keyboard_hook.rs` as a self-contained module exposing `install() -> HookHandle` and `HookHandle::uninstall()`. The hook thread owns the Win32 state; the dispatcher thread owns the Tauri integration. Keep them separate.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `windows` crate | 0.58 (already in Cargo.toml) | Win32 API bindings: SetWindowsHookExW, GetMessage, PostThreadMessageW, SendInput, UnhookWindowsHookEx | Official Microsoft Rust bindings; already a dependency — zero new crates |
| `std::sync::mpsc` | stdlib | Non-blocking channel from hook callback to dispatcher | Already established pattern in this codebase (see Arc<AtomicBool> + try_send elsewhere) |
| `std::sync::atomic::AtomicBool` | stdlib | Cross-thread state flags (hook active, combo state) | Established pattern in codebase for LevelStreamActive |

### New Feature Flags Required
Add to `[target.'cfg(windows)'.dependencies]` windows entry in `Cargo.toml`:
```toml
windows = { version = "0.58", features = [
    "Win32_Graphics_Dxgi",          # already present
    "Win32_Foundation",             # ADD: BOOL, LRESULT, WPARAM, LPARAM, HHOOK
    "Win32_UI_WindowsAndMessaging", # ADD: SetWindowsHookExW, GetMessageW, PostThreadMessageW, WH_KEYBOARD_LL, WM_QUIT, KBDLLHOOKSTRUCT, CallNextHookEx, UnhookWindowsHookEx, HC_ACTION
    "Win32_UI_Input_KeyboardAndMouse", # ADD: SendInput, INPUT, KEYBDINPUT, KEYEVENTF_KEYUP, VK_LCONTROL, VK_RCONTROL, VK_LWIN, VK_RWIN
] }
```

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `tauri-plugin-global-shortcut` | 2 (already present) | Fallback for standard (non-modifier-only) hotkeys | Kept as-is for Phase 16 routing; this phase doesn't remove it |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| raw `windows` crate | `win-hotkeys` crate | win-hotkeys is cleaner API but adds a dependency; windows crate is already present and v0.58 is pinned for ORT compatibility — no new deps is a hard constraint |
| raw `windows` crate | `winapi` crate | winapi is unmaintained; windows-rs is the official successor |
| mpsc channel bridge | Tauri event emit from hook | Hook callback is sync/unsafe; emitting Tauri events requires AppHandle which cannot safely cross into a static callback without careful Arc management — mpsc is simpler and lower risk |

**Installation:** No new packages. Add 3 feature flags to existing `windows = "0.58"` entry.

---

## Architecture Patterns

### Recommended Module Structure
```
src-tauri/src/
├── keyboard_hook.rs     # NEW: WH_KEYBOARD_LL thread, state machine, Start menu suppression
└── lib.rs               # MODIFY: add `mod keyboard_hook;`, spawn hook in setup(), DeviceEventFilter fix, default hotkey change
```

### Pattern 1: Hook Thread with GetMessage Loop

**What:** A dedicated `std::thread::spawn` installs the hook and runs `GetMessage` until `WM_QUIT`. The thread ID is captured at startup and stored in an `Arc<AtomicU32>` (or sent via a oneshot channel) for shutdown use.

**When to use:** Required — WH_KEYBOARD_LL callbacks are delivered to the message queue of the installing thread. If the thread exits or has no message loop, the hook is silently removed by Windows on Win7+.

**Example:**
```rust
// Source: Win32 docs (SetWindowsHookExW + LowLevelKeyboardProc)
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Threading::GetCurrentThreadId;

pub struct HookHandle {
    thread_id: Arc<AtomicU32>,
}

impl HookHandle {
    pub fn uninstall(&self) {
        let tid = self.thread_id.load(Ordering::Relaxed);
        if tid != 0 {
            unsafe {
                PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
    }
}

pub fn install(tx: std::sync::mpsc::SyncSender<HookEvent>) -> HookHandle {
    let thread_id = Arc::new(AtomicU32::new(0));
    let tid_clone = thread_id.clone();

    std::thread::spawn(move || {
        // CRITICAL: Store thread ID before installing hook
        let my_tid = unsafe { GetCurrentThreadId() };
        tid_clone.store(my_tid, Ordering::Relaxed);

        // Install hook — hmod=None and dwThreadId=0 for global scope
        // WH_KEYBOARD_LL: hmod MUST be None when dwThreadId=0 for the calling process
        let hook = unsafe {
            SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), None, 0)
                .expect("SetWindowsHookExW failed")
        };

        // GetMessage loop — required for hook delivery
        let mut msg = MSG::default();
        loop {
            let result = unsafe { GetMessageW(&mut msg, None, 0, 0) };
            match result.0 {
                -1 => break, // Error
                0  => break, // WM_QUIT received
                _  => {
                    unsafe {
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            }
        }

        // Cleanup — called on the hook thread after WM_QUIT
        unsafe { UnhookWindowsHookEx(hook).ok(); }
    });

    HookHandle { thread_id }
}
```

### Pattern 2: Non-Blocking Hook Callback

**What:** The hook callback (`hook_proc`) does the absolute minimum: write an `AtomicBool`, call `try_send` on an mpsc channel, then either return `CallNextHookEx` (pass-through) or return `LRESULT(1)` (suppress). Never allocate, never lock a Mutex, never sleep.

**When to use:** Always. WH_KEYBOARD_LL has a system-enforced timeout (configurable via registry, max 1000ms on Win10 1709+). If the callback exceeds it, the hook is silently removed. The 5ms budget in HOOK-02 is a conservative internal target well within the deadline.

**Example:**
```rust
// Source: LowLevelKeyboardProc docs (learn.microsoft.com)
// SAFETY: hook_proc is called from hook thread; KBDLLHOOKSTRUCT is valid for callback lifetime
unsafe extern "system" fn hook_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if ncode < 0 {
        // Must forward without processing when ncode < 0
        return CallNextHookEx(None, ncode, wparam, lparam);
    }

    let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);

    // CRITICAL: Skip injected events to prevent infinite loop when we inject VK_E8
    // LLKHF_INJECTED = 0x10 (bit 4 of flags)
    if (kb.flags.0 & LLKHF_INJECTED.0) != 0 {
        return CallNextHookEx(None, ncode, wparam, lparam);
    }

    let vk = VIRTUAL_KEY(kb.vkCode as u16);
    let is_down = wparam.0 as u32 == WM_KEYDOWN.0 || wparam.0 as u32 == WM_SYSKEYDOWN.0;
    let is_up   = wparam.0 as u32 == WM_KEYUP.0   || wparam.0 as u32 == WM_SYSKEYUP.0;

    // Update modifier state atomics (non-blocking writes)
    // ... (see state machine pattern below)

    // If suppressing Win key: inject VK_E8, return LRESULT(1)
    // Otherwise: return CallNextHookEx(...)
    CallNextHookEx(None, ncode, wparam, lparam)
}
```

### Pattern 3: Modifier State Machine with 50ms Debounce

**What:** Track four booleans (ctrl_down, win_down, combo_active, first_key_time). When either key goes down: record `kb.time` (milliseconds from KBDLLHOOKSTRUCT). When both keys are down within 50ms, fire Pressed. When either key releases while combo is active, fire Released and clear combo.

**Key decision (Claude's discretion):** "Either key releases" triggers end-of-recording. This matches natural hold-to-talk UX — users release one key before the other. Requiring both to release creates a confusing double-release feel.

**Key decision (Claude's discretion):** Extra keys pressed during active recording are ignored (not cancelled). Recording continues until Ctrl or Win is released. Cancellation is complex UX and not required by any success criterion.

**Debounce state machine (pseudo-code):**
```
State: { ctrl_held: bool, win_held: bool, combo_active: bool, first_key_time: u32 }

On VK_LCTRL/VK_RCTRL KEYDOWN:
    ctrl_held = true
    if win_held && (kb.time - first_key_time <= 50) && !combo_active:
        combo_active = true
        suppress_next_win_up = true
        SEND Pressed
    elif !win_held:
        first_key_time = kb.time  // Ctrl was first key

On VK_LWIN/VK_RWIN KEYDOWN (HC_ACTION):
    if combo_active: suppress (return 1, inject VK_E8)
    win_held = true
    if ctrl_held && (kb.time - first_key_time <= 50) && !combo_active:
        combo_active = true
        suppress_next_win_up = true
        inject VK_E8 via SendInput (breaks Start menu detection)
        SEND Pressed
        return LRESULT(1)  // suppress Win keydown
    elif !ctrl_held:
        first_key_time = kb.time  // Win was first key

On VK_LCTRL/VK_RCTRL KEYUP:
    ctrl_held = false
    if combo_active:
        combo_active = false
        SEND Released

On VK_LWIN/VK_RWIN KEYUP:
    win_held = false
    if combo_active:
        combo_active = false
        SEND Released
        return LRESULT(1)  // suppress Win keyup (prevents Start menu)
    if suppress_next_win_up:
        suppress_next_win_up = false
        return LRESULT(1)
```

**Note:** Exact modifier detection — checking Shift/Alt to enforce "no other modifiers" rule (locked decision). Use `GetKeyState(VK_SHIFT)` and `GetKeyState(VK_MENU)` at combo-fire time, or track them in the hook. If either is held, skip combo activation.

### Pattern 4: VK_E8 Start Menu Suppression

**What:** The Windows Start menu activates when the Win key is pressed and released without any intervening keypress. Injecting VK_E8 (0xE8, officially "unassigned") as a synthetic KEYDOWN via `SendInput` before the Win key completes its down-up cycle prevents this pattern. Return `LRESULT(1)` from the hook for both Win KEYDOWN and Win KEYUP when the combo is active.

**Why VK_E8:** Microsoft documents 0xE8 as "unassigned." VK_07 was previously used but became reserved for Xbox Game Bar on Win10 1909+. VK_E8 has been stable as AutoHotkey's default `MenuMaskKey` across Windows versions.

**Implementation:**
```rust
// Source: AHK MenuMaskKey docs + Win32 SendInput docs
fn inject_vk_e8() {
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0xE8),
                wScan: 0,
                dwFlags: KEYEVENTF_KEYDOWN,  // KEYDOWN only is sufficient
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32); }
}
```

**Windows 11 validation flag (from STATE.md):** The exact timing — KEYDOWN only vs KEYDOWN+KEYUP — requires empirical validation during implementation. KEYDOWN-only is the standard AHK behavior and is the starting point.

### Pattern 5: Tauri DeviceEventFilter Fix

**What:** By default, tao (Tauri's windowing library) sets `DeviceEventFilter::Unfocused`, which prevents device events (including keyboard) from being processed when the Tauri window has focus. This breaks WH_KEYBOARD_LL delivery.

**Fix:** Add `.device_event_filter(tauri::DeviceEventFilter::Always)` to the Tauri Builder chain in `lib.rs::run()`, before `.invoke_handler(...)` or `.setup(...)`.

```rust
// Source: tauri#13919 confirmed fix
let mut builder = tauri::Builder::default()
    .device_event_filter(tauri::DeviceEventFilter::Always)  // ADD THIS
    .plugin(tauri_plugin_single_instance::init(...))
    // ... rest of plugins
```

**Confirmed:** This is the fix for tauri#13919. Without it, Success Criterion 1 (hook fires when Tauri window is focused) cannot pass.

### Pattern 6: Channel Bridge from Hook Thread to Tauri

**What:** The hook callback cannot call `handle_shortcut()` directly because handle_shortcut takes `&tauri::AppHandle` which is not `Send + Sync` in the hook's static context. Instead, use `mpsc::sync_channel` with capacity 0 or small buffer. A dispatcher thread (spawned in setup()) reads the channel and calls `handle_shortcut()`.

```rust
// Dispatcher thread pattern
let (tx, rx) = std::sync::mpsc::sync_channel::<HookEvent>(32);
let app_handle = app.handle().clone();

std::thread::spawn(move || {
    for event in rx {
        match event {
            HookEvent::Pressed  => handle_shortcut(&app_handle, &make_pressed_event()),
            HookEvent::Released => handle_shortcut(&app_handle, &make_released_event()),
        }
    }
});
```

The `tx` end is stored in a `static` or `OnceLock<SyncSender<HookEvent>>` accessible from the hook callback.

**Alternative:** Avoid a static sender by embedding state in a thread-local or using a `OnceLock`. The existing codebase uses `Arc<AtomicBool>` for cross-thread flags — same pattern applies here.

### Anti-Patterns to Avoid

- **Mutex in hook callback:** Any blocking operation risks exceeding the hook timeout. Mutex can block. Only use AtomicBool, AtomicU32, and try_send.
- **Calling handle_shortcut() directly from hook_proc:** AppHandle is not safely passable into a static callback. Use channel bridge.
- **Installing hook on the Tauri main thread:** The main thread runs the Tauri event loop. Blocking it with GetMessage causes deadlock. Use a dedicated std::thread.
- **Using tokio task for the hook thread:** Tokio tasks are async and not guaranteed to stay on the same OS thread. WH_KEYBOARD_LL requires the installing OS thread to have a message loop. Use std::thread::spawn.
- **Calling GetKeyState from hook callback:** The docs explicitly state: "the asynchronous state of the key cannot be determined by calling GetAsyncKeyState from within the callback function." Track modifier state explicitly in the hook callback via the vkCode of each event.
- **Skipping LLKHF_INJECTED check:** If injecting VK_E8 via SendInput without filtering LLKHF_INJECTED, the hook re-fires on the injected event, causing infinite recursion and either a stack overflow or hook timeout removal.
- **Not calling UnhookWindowsHookEx on exit:** Leaving an installed hook causes "dangling hook" — other processes' keyboard input is delayed or dropped (the hook callback address is invalid, the system waits for timeout before proceeding). Confirmed failure mode in HOOK-04 success criterion.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Thread-safe hook state | Custom state struct with Mutex | `AtomicBool` + `AtomicU32` | Hook callback is timing-critical; Mutex blocks; Atomics are lock-free |
| Synthetic key injection | Custom Win32 keybd_event calls | `SendInput` with KEYBDINPUT | `keybd_event` is deprecated; SendInput is the correct modern API and correctly sets LLKHF_INJECTED |
| Start menu suppression | Registry edits or shell hooks | VK_E8 inject + return nonzero | Registry edits require elevation and persist after crash; the hook technique is scoped to app lifetime |
| Shutdown signaling | Custom WM_USER messages | `PostThreadMessageW(WM_QUIT)` | WM_QUIT causes GetMessage to return 0 cleanly, which is the standard Win32 shutdown pattern |

**Key insight:** The hook thread architecture (install → GetMessage → UnhookWindowsHookEx → exit) is a well-understood Win32 pattern. The complexity in this phase is entirely in the state machine logic, not the infrastructure.

---

## Common Pitfalls

### Pitfall 1: Hook Silently Removed on Timeout
**What goes wrong:** The hook callback takes too long. On Windows 7+, the system silently removes the hook without any notification. The app continues to run, appearing functional, but hotkeys stop working.
**Why it happens:** Blocking operations (Mutex, I/O, sleep) inside the callback.
**How to avoid:** HOOK-02 enforces: only AtomicBool writes and non-blocking try_send. Implement and test with a tight linter rule: no blocking calls in hook_proc.
**Warning signs:** Hotkeys work initially then stop after some presses; no errors in logs.

### Pitfall 2: Tauri Window Focus Breaks Hook Delivery
**What goes wrong:** Pressing Ctrl+Win while the VoiceType settings window is focused does nothing. The hook is installed and running, but never receives events when focus is on the Tauri window.
**Why it happens:** tao's default `DeviceEventFilter::Unfocused` prevents device events when the app has focus.
**How to avoid:** `.device_event_filter(tauri::DeviceEventFilter::Always)` on the Tauri Builder. This is HOOK-03 and must be applied before `.build()`.
**Warning signs:** Success Criterion 1 fails immediately in testing.

### Pitfall 3: Infinite Loop from VK_E8 Re-injection
**What goes wrong:** The hook callback injects VK_E8 via SendInput. SendInput generates a new WM_KEYDOWN event. The hook re-fires for the injected event, injects another VK_E8, infinitely.
**Why it happens:** Not checking the LLKHF_INJECTED flag before processing.
**How to avoid:** At the top of hook_proc: if `(kb.flags.0 & LLKHF_INJECTED.0) != 0 { return CallNextHookEx(...) }`. This skips all injected events including our own VK_E8.
**Warning signs:** App freezes or CPU spikes when Win key is pressed; hook timeout removal.

### Pitfall 4: Dangling Hook on App Crash
**What goes wrong:** App crashes without calling UnhookWindowsHookEx. The hook HHOOK handle is invalid, but Windows still tries to call the callback. Subsequent keyboard input on the system is delayed by the timeout period.
**Why it happens:** Panic in Rust unwind path, or SIGTERM from OS, or process killed by debugger.
**How to avoid:** Hook cleanup must happen inside the hook thread after its GetMessage loop exits. The hook thread owns the HHOOK handle and is responsible for cleanup. Additionally, implement a Drop guard on HookHandle that sends WM_QUIT.
**Warning signs:** Keyboard input is slow or delayed after VoiceType exits (including after crash); Event Viewer shows application errors.

### Pitfall 5: Start Menu Suppression Timing (Windows 11)
**What goes wrong:** On Windows 11, the Start menu suppression via VK_E8 may require both KEYDOWN and KEYUP injection, not just KEYDOWN. This is flagged in STATE.md as requiring empirical validation.
**Why it happens:** Windows 11 redesigned the Start menu trigger mechanism; the exact threshold for "intervening keypress" may differ from Windows 10.
**How to avoid:** Start with KEYDOWN-only injection (standard AHK behavior). If Success Criterion 3 fails on Windows 11, add a VK_E8 KEYUP injection immediately after the KEYDOWN. This is the known fallback.
**Warning signs:** Start menu opens when Ctrl+Win is pressed on Windows 11 despite suppression logic.

### Pitfall 6: Exact Modifier Match — Shift/Alt Check
**What goes wrong:** Ctrl+Win+Shift accidentally activates dictation, violating the locked decision (exact match, no other modifiers).
**Why it happens:** State machine only checks Ctrl and Win without tracking Shift/Alt.
**How to avoid:** Use `GetKeyState(VK_SHIFT)` and `GetKeyState(VK_MENU)` at combo-fire time (not GetAsyncKeyState, which is unreliable in the callback). Alternatively, track Shift and Alt state in the hook callback alongside Ctrl and Win. If either is held when the combo would fire, skip activation.
**Warning signs:** Ctrl+Win+D (virtual desktop shortcut) also triggers dictation.

### Pitfall 7: std::thread vs tokio task for Hook Thread
**What goes wrong:** Using `tauri::async_runtime::spawn` or a tokio task for the hook thread. The hook callback is never delivered because tokio tasks can migrate between OS threads.
**Why it happens:** WH_KEYBOARD_LL delivers events to the *OS thread* that called SetWindowsHookExW. Tokio may run the task on a different thread, or no thread at all.
**How to avoid:** Use `std::thread::spawn` only. This is called out explicitly in CONTEXT.md and STATE.md decisions.
**Warning signs:** Hook installs without error but no key events are ever received.

---

## Code Examples

Verified patterns from official sources:

### Win32 Feature Flags for windows 0.58
```toml
# src-tauri/Cargo.toml — modify existing windows dependency
[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = [
    "Win32_Graphics_Dxgi",
    "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging",
    "Win32_UI_Input_KeyboardAndMouse",
] }
```

### Hook Install + GetMessage Loop (Rust, windows 0.58)
```rust
// Source: Win32 docs (SetWindowsHookExW, LowLevelKeyboardProc, GetMessage)
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::System::Threading::GetCurrentThreadId;

// Called from hook_proc — static or OnceLock sender
static HOOK_TX: std::sync::OnceLock<std::sync::mpsc::SyncSender<HookEvent>>
    = std::sync::OnceLock::new();

#[derive(Debug, Clone, Copy)]
pub enum HookEvent { Pressed, Released }

unsafe extern "system" fn hook_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if ncode < 0 {
        return CallNextHookEx(None, ncode, wparam, lparam);
    }
    let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);

    // Skip injected events (our own VK_E8 injection)
    if (kb.flags.0 & LLKHF_INJECTED.0) != 0 {
        return CallNextHookEx(None, ncode, wparam, lparam);
    }

    // ... state machine logic (see Pattern 3) ...

    CallNextHookEx(None, ncode, wparam, lparam)
}
```

### Tauri Builder DeviceEventFilter Fix
```rust
// Source: tauri#13919 confirmed fix
// src-tauri/src/lib.rs — add to run() function
let mut builder = tauri::Builder::default()
    .device_event_filter(tauri::DeviceEventFilter::Always)  // HOOK-03
    .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        // ...
    }))
    // ... rest unchanged
```

### Default Hotkey Change
```rust
// Source: CONTEXT.md locked decision
// src-tauri/src/lib.rs — setup() closure, line ~1430
let hotkey = read_saved_hotkey(app)
    .unwrap_or_else(|| "ctrl+win".to_owned());  // was "ctrl+shift+space"
```

### PostThreadMessageW Shutdown
```rust
// Source: Win32 docs (PostThreadMessageW)
pub fn shutdown(hook_thread_id: u32) {
    unsafe {
        PostThreadMessageW(hook_thread_id, WM_QUIT, WPARAM(0), LPARAM(0))
            .ok(); // Ignore error if thread already exited
    }
}
```

### SendInput VK_E8 Injection
```rust
// Source: Win32 SendInput docs + AHK MenuMaskKey mechanism
use windows::Win32::UI::Input::KeyboardAndMouse::*;

unsafe fn inject_mask_key() {
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0xE8),
                wScan: 0,
                dwFlags: KEYEVENTF_KEYDOWN,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `RegisterHotKey` Win32 API | `tauri-plugin-global-shortcut` (wraps RegisterHotKey) | Tauri 2 | Tauri-managed; cannot handle modifier-only combos |
| `keybd_event` for synthesis | `SendInput` | Windows XP era | keybd_event is deprecated; SendInput is the current API |
| WH_KEYBOARD hook (thread-scoped) | WH_KEYBOARD_LL (global, no DLL needed) | Windows 2000+ | LL hook fires for all input system-wide; no DLL injection needed; hmod=NULL |
| Journaling hooks for playback | Unsupported in Windows 11 | Windows 11 | SetWindowsHookEx with WH_JOURNALPLAYBACK/WH_JOURNALRECORD are removed; use SendInput |

**Deprecated/outdated:**
- `keybd_event`: Deprecated since Windows XP. Use `SendInput` instead.
- Journaling hooks (WH_JOURNALPLAYBACK, WH_JOURNALRECORD): Unsupported starting Windows 11. Irrelevant to this phase but noted for completeness.

---

## Open Questions

1. **VK_E8 KEYDOWN-only vs KEYDOWN+KEYUP on Windows 11**
   - What we know: KEYDOWN-only is the standard AHK approach and works on Windows 10. STATE.md flags this as requiring empirical validation.
   - What's unclear: Whether Windows 11's Start menu responds to KEYDOWN-only injection or requires both events.
   - Recommendation: Implement KEYDOWN-only first. If Success Criterion 3 fails on Windows 11, add KEYUP injection. Document which variant was used in implementation summary.

2. **Hook auto-install timing: always-on vs conditional**
   - What we know: CONTEXT.md marks this as Claude's discretion.
   - What's unclear: Whether installing a WH_KEYBOARD_LL hook when the user is using a non-Ctrl+Win hotkey wastes resources or causes issues.
   - Recommendation: Always install when the app starts. The hook callback is nearly instantaneous for non-matching keys (checks a few flags and calls CallNextHookEx). The overhead is negligible. Always-on is simpler to reason about and avoids edge cases where the user switches hotkeys.

3. **Hook failure behavior: silent vs notify**
   - What we know: CONTEXT.md marks this as Claude's discretion. INT-03 (fallback to RegisterHotKey on failure) is Phase 16.
   - What's unclear: Whether a clean silent fallback or a notification is better UX for Phase 15 scope.
   - Recommendation: Silent fallback for Phase 15. Log the error via `log::error!`. Phase 16 will add the notify-and-fallback path when it implements INT-03.

---

## Integration Map

### Changes Required in lib.rs

1. **Add `mod keyboard_hook;`** at top with other module declarations
2. **Add `.device_event_filter(tauri::DeviceEventFilter::Always)`** to Builder in `run()` before plugins
3. **Change default hotkey** from `"ctrl+shift+space"` to `"ctrl+win"` in `setup()` (line ~1430)
4. **Spawn hook in `setup()`** conditional on hotkey format: if `hotkey == "ctrl+win"`, spawn hook thread and store handle
5. **Add HookHandle managed state** so cleanup can access it from shutdown path
6. **Register global shortcut conditionally**: only register with `tauri_plugin_global_shortcut` when hotkey is NOT the hook-managed format

### New File: keyboard_hook.rs
- Public API: `install(app: &tauri::App) -> HookHandle`, `HookHandle::uninstall()`
- Internal: `hook_proc` (static unsafe extern), `ModifierState` (AtomicBool fields), `HOOK_TX` (OnceLock<SyncSender>)
- Dispatcher thread: spawned in `install()`, reads from rx, calls `handle_shortcut()`

---

## Sources

### Primary (HIGH confidence)
- [LowLevelKeyboardProc - Win32 Docs](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelkeyboardproc) — nCode, wParam, lParam semantics, return value, timing constraints, LLKHF_INJECTED behavior
- [SetWindowsHookExW - Win32 Docs](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexw) — hmod=NULL requirement for WH_KEYBOARD_LL, dwThreadId=0 for global, bitness requirements
- [KBDLLHOOKSTRUCT - docs.rs](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/UI/WindowsAndMessaging/struct.KBDLLHOOKSTRUCT.html) — struct fields (vkCode, flags, time, dwExtraInfo)
- [Tauri issue #13919](https://github.com/tauri-apps/tauri/issues/13919) — DeviceEventFilter::Always fix confirmed
- [AHK A_MenuMaskKey docs](https://www.autohotkey.com/docs/v2/lib/A_MenuMaskKey.htm) — VK_E8 mechanism for Start menu suppression

### Secondary (MEDIUM confidence)
- [win-hotkeys hook.rs](https://github.com/iholston/win-hotkeys) — GetMessage loop pattern, shutdown via ControlFlow::Exit channel, UnhookWindowsHookEx inside hook thread
- [AHK Virtual Key Codes discussion](https://www.autohotkey.com/boards/viewtopic.php?t=101812) — VK_E8 stability vs VK_07 deprecation

### Tertiary (LOW confidence — needs validation)
- Windows 11 Start menu VK_E8 behavior — no authoritative source found; STATE.md notes empirical validation required during implementation

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — windows 0.58 crate is already in Cargo.toml; feature flags are from official docs.rs; no new dependencies
- Architecture: HIGH — Win32 hook patterns from official Microsoft docs; Tauri fix from confirmed GitHub issue; channel bridge from established codebase patterns
- Pitfalls: HIGH for known Win32 pitfalls (timeout, LLKHF_INJECTED, dangling hook); MEDIUM for Windows 11 VK_E8 timing (empirical validation required)

**Research date:** 2026-03-02
**Valid until:** 2026-06-01 (Win32 hook APIs are stable; Tauri fix is merged; 90-day window)
