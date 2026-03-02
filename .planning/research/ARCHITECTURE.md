# Architecture Research

**Domain:** Win32 low-level keyboard hook integration into existing Tauri 2.0 app (v1.2 milestone)
**Researched:** 2026-03-02
**Confidence:** HIGH — Win32 APIs are stable, well-documented on MSDN; Tauri AppHandle threading patterns verified via official Tauri docs and community discussions; debounce strategy derived from AutoHotkey community knowledge and Win32 message timing

---

> **Scope:** This document covers the WH_KEYBOARD_LL integration architecture for v1.2 only. It focuses on how the new hook module connects to the existing Tauri 2.0 app. The broader application architecture (audio pipeline, transcription, VAD, UI) is unchanged — see the 2026-02-27 version of this file in git history.

---

## System Overview

### Before (v1.1) — RegisterHotKey via tauri-plugin-global-shortcut

```
┌───────────────────────────────────────────────────────────────────┐
│                    TAURI MAIN THREAD                              │
│   Win32 message loop (pumped by Tauri / Webview2 COM)             │
│                                                                   │
│   tauri-plugin-global-shortcut                                    │
│     └─ RegisterHotKey(hwnd, id, MOD_CONTROL, VK_SPACE)            │
│          ↓ WM_HOTKEY delivered to message loop                    │
│          ↓ ShortcutEvent{Pressed/Released}                        │
│          ↓ handle_shortcut(app, event)  ← existing function       │
└───────────────────────────────────────────────────────────────────┘
```

**Why this cannot do Ctrl+Win:** `RegisterHotKey` requires a non-zero virtual key code in its `vk` parameter — there is no way to register a modifier-only hotkey (e.g. Ctrl+Win with no letter key). This is a fundamental Win32 API limitation, not a plugin bug.

### After (v1.2) — WH_KEYBOARD_LL hook (hybrid approach)

```
┌───────────────────────────────────────────────────────────────────┐
│                    TAURI MAIN THREAD                              │
│   tauri-plugin-global-shortcut KEPT for standard hotkeys          │
│   (e.g. user chose Ctrl+Shift+Space — still uses RegisterHotKey)  │
└───────────────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────────────┐
│              HOOK THREAD  (std::thread, named "keyboard-hook")    │
│                                                                   │
│  SetWindowsHookExW(WH_KEYBOARD_LL, hook_proc, NULL, 0)            │
│  ONLY installed when saved hotkey is a modifier-only combo        │
│                                                                   │
│  Runs: loop { GetMessageW(&msg, None, 0, 0); DispatchMessageW; }  │
│    ↑ mandatory: WH_KEYBOARD_LL fires by sending a message to the  │
│      installing thread — without GetMessage the hook never fires  │
│                                                                   │
│  hook_proc() [extern "system" fn — no user data allowed]:         │
│    reads  KBDLLHOOKSTRUCT { vkCode, flags, time }                 │
│    tracks CTRL_DOWN / WIN_DOWN via AtomicBool (thread-local)      │
│    debounce: both set within 50ms → set COMBO_ACTIVE flag         │
│    on combo: try_send(HookEvent::Pressed) — non-blocking          │
│    on Win keyup when COMBO_ACTIVE: return LRESULT(1)  ← swallow   │
│    otherwise: CallNextHookEx (pass through)                       │
└──────────────────────────┬────────────────────────────────────────┘
                           │ std::sync::mpsc::Sender<HookEvent>
                           │ (Sender stored in OnceLock, populated at startup)
                           ↓
┌───────────────────────────────────────────────────────────────────┐
│          HOOK DISPATCHER  (std::thread, named "hook-dispatcher")  │
│                                                                   │
│  loop { match rx.recv() {                                         │
│    HookEvent::Pressed  → handle_shortcut_pressed(&app)            │
│    HookEvent::Released → handle_shortcut_released(&app)           │
│    HookEvent::Stop     → break                                    │
│  }}                                                               │
│                                                                   │
│  AppHandle cloned in setup(), stored in HOOK_APP: OnceLock        │
└──────────────────────────┬────────────────────────────────────────┘
                           │ calls existing lib.rs functions
                           ↓
┌───────────────────────────────────────────────────────────────────┐
│              EXISTING lib.rs — UNCHANGED                          │
│                                                                   │
│  handle_shortcut(app, ShortcutState::Pressed/Released)            │
│    PipelineState transitions (IDLE→RECORDING→PROCESSING)          │
│    AudioCaptureMutex, VAD worker, transcription pipeline          │
│    tray state, pill overlay, Emitter events to frontend           │
└───────────────────────────────────────────────────────────────────┘
```

## Component Responsibilities

| Component | Status | Responsibility |
|-----------|--------|----------------|
| `keyboard_hook.rs` | NEW | Install WH_KEYBOARD_LL, run GetMessage loop, track modifier state, debounce, channel, suppression |
| `OnceLock<AppHandle>` in `keyboard_hook.rs` | NEW | Bridge between Win32 callback (no user param) and Tauri runtime |
| `std::sync::mpsc` channel | NEW | Carry `HookEvent` from hook thread to dispatcher thread |
| Hook dispatcher thread | NEW | Receive `HookEvent`, call `handle_shortcut_pressed/released` |
| `handle_shortcut()` in `lib.rs` | EXISTING, UNCHANGED | Drives recording pipeline from Pressed/Released events |
| `tauri-plugin-global-shortcut` | EXISTING, KEPT | Handles standard (non-modifier-only) hotkeys; fallback if hook fails |
| Frontend hotkey capture UI | MODIFIED | Must display and capture modifier-only combos (Ctrl+Win shows no letter) |
| `rebind_hotkey` IPC command | MODIFIED | Route to hook path vs. plugin path based on hotkey type |

## Recommended File Changes

```
src-tauri/src/
├── lib.rs                    # MODIFIED
│   ├── add mod keyboard_hook
│   ├── setup(): call keyboard_hook::start() if saved hotkey is modifier-only
│   ├── setup(): keep plugin registration for standard hotkeys
│   ├── rebind_hotkey(): detect modifier-only format, route to hook::rebind()
│   └── app teardown: call keyboard_hook::stop()
│
├── keyboard_hook.rs          # NEW — ~200 LOC
│   ├── pub enum HookEvent { Pressed, Released, Stop }
│   ├── static CTRL_DOWN: AtomicBool
│   ├── static WIN_DOWN: AtomicBool
│   ├── static COMBO_ACTIVE: AtomicBool
│   ├── static LAST_MOD_TIME: AtomicU64     (KBDLLHOOKSTRUCT.time in ms)
│   ├── static HOOK_SENDER: OnceLock<Mutex<mpsc::Sender<HookEvent>>>
│   ├── static HOOK_APP: OnceLock<AppHandle<Wry>>
│   ├── static HOOK_THREAD_ID: AtomicU32    (for PostThreadMessageW on shutdown)
│   ├── pub fn start(app: AppHandle<Wry>)   → installs hook, spawns dispatcher
│   ├── pub fn stop()                       → sends Stop + WM_QUIT to hook thread
│   ├── pub fn rebind(combo: HookCombo)     → update target modifier set atomically
│   └── unsafe extern "system" fn hook_proc(ncode, wparam, lparam) -> LRESULT
│
├── audio.rs, pipeline.rs, vad.rs, etc.     # UNCHANGED
└── ...
```

## Architectural Patterns

### Pattern 1: Dedicated Hook Thread with Synchronous GetMessage Loop

**What:** Install WH_KEYBOARD_LL on a `std::thread` (not a Tokio task) that runs a blocking `GetMessageW` / `DispatchMessageW` loop. The hook callback fires on this same thread.

**Why mandatory:** The Win32 documentation states explicitly: "This hook is called in the context of the thread that installed it. The call is made by sending a message to the thread that installed the hook. Therefore, the thread that installed the hook must have a message loop." Installing on a Tokio worker thread (no Win32 message loop) causes the hook to silently never fire.

**Example:**
```rust
// keyboard_hook.rs
pub fn start(app: AppHandle<Wry>) {
    let (tx, rx) = std::sync::mpsc::channel::<HookEvent>();
    HOOK_SENDER.set(Mutex::new(tx)).ok();
    HOOK_APP.set(app.clone()).ok();

    // Dispatcher thread — reads channel, calls handle_shortcut
    let dispatch_app = app.clone();
    std::thread::Builder::new()
        .name("hook-dispatcher".into())
        .spawn(move || {
            for event in rx {
                match event {
                    HookEvent::Pressed  => handle_shortcut_pressed(&dispatch_app),
                    HookEvent::Released => handle_shortcut_released(&dispatch_app),
                    HookEvent::Stop     => break,
                }
            }
        })
        .expect("hook dispatcher spawn failed");

    // Hook thread — installs WH_KEYBOARD_LL and pumps GetMessage
    std::thread::Builder::new()
        .name("keyboard-hook".into())
        .spawn(|| unsafe {
            HOOK_THREAD_ID.store(
                windows::Win32::System::Threading::GetCurrentThreadId(),
                Ordering::Relaxed,
            );
            let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), None, 0)
                .expect("WH_KEYBOARD_LL installation failed");

            let mut msg = MSG::default();
            // Blocks until WM_QUIT
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            UnhookWindowsHookEx(hook).ok();
        })
        .expect("hook thread spawn failed");
}
```

### Pattern 2: Modifier State Tracking via Atomics (not GetAsyncKeyState)

**What:** Track Ctrl and Win key down/up state using `AtomicBool` updated inside the hook proc on every relevant keydown/keyup event. Never call `GetAsyncKeyState` inside the callback.

**Why mandatory:** The LowLevelKeyboardProc documentation explicitly states: "the asynchronous state of the key cannot be determined by calling GetAsyncKeyState from within the callback function." The atomic approach is lock-free and safe to call from the hook proc's non-async context.

**Virtual key codes to watch:**
- `VK_LCONTROL` (0xA2), `VK_RCONTROL` (0xA3) — left and right Ctrl
- `VK_LWIN` (0x5B), `VK_RWIN` (0x5C) — left and right Win

**Debounce logic (50ms window):** Physical keypresses of two keys simultaneously arrive 5–30ms apart depending on hardware. The debounce window fires the combo when both modifiers are down within 50ms of each other, regardless of which was pressed first. `KBDLLHOOKSTRUCT.time` provides the millisecond timestamp for each event.

**Example:**
```rust
// Inside hook_proc — modifier state machine
let is_ctrl = vk == VK_LCONTROL.0 as u32 || vk == VK_RCONTROL.0 as u32;
let is_win  = vk == VK_LWIN.0 as u32      || vk == VK_RWIN.0 as u32;

if is_keydown && (is_ctrl || is_win) {
    if is_ctrl { CTRL_DOWN.store(true, Ordering::Relaxed); }
    if is_win  { WIN_DOWN.store(true,  Ordering::Relaxed); }
    LAST_MOD_TIME.store(kb.time as u64, Ordering::Relaxed);

    let ctrl = CTRL_DOWN.load(Ordering::Relaxed);
    let win  = WIN_DOWN.load(Ordering::Relaxed);
    let age  = kb.time as u64 - LAST_MOD_TIME.load(Ordering::Relaxed);

    if ctrl && win && age < 50 && !COMBO_ACTIVE.load(Ordering::Relaxed) {
        COMBO_ACTIVE.store(true, Ordering::Relaxed);
        if let Some(lock) = HOOK_SENDER.get() {
            if let Ok(tx) = lock.lock() {
                let _ = tx.try_send(HookEvent::Pressed);
            }
        }
    }
}

if is_keyup && (is_ctrl || is_win) {
    let was_combo = COMBO_ACTIVE.load(Ordering::Relaxed);

    if is_ctrl { CTRL_DOWN.store(false, Ordering::Relaxed); }
    if is_win  { WIN_DOWN.store(false,  Ordering::Relaxed); }

    // Send Released when either modifier is lifted (first one to release ends combo)
    if was_combo {
        if let Some(lock) = HOOK_SENDER.get() {
            if let Ok(tx) = lock.lock() {
                let _ = tx.try_send(HookEvent::Released);
            }
        }
        COMBO_ACTIVE.store(false, Ordering::Relaxed);

        // Swallow Win keyup to suppress Start menu
        if is_win {
            return LRESULT(1); // consumed — do NOT call CallNextHookEx
        }
    }
}
```

### Pattern 3: AppHandle in Hook via OnceLock Static

**What:** Win32 hook proc signatures are fixed (`extern "system" fn(i32, WPARAM, LPARAM) -> LRESULT`) — no user data pointer, no closure capture. Store `AppHandle` in a `OnceLock<AppHandle<Wry>>` initialized once during `setup()`. The dispatcher thread reads it from there.

**Tauri 2.0 specifics:** `AppHandle<Wry>` is `Send` in Tauri 2.0 (the historical issue #2343 "AppHandle is not Send+Sync" was for Tauri 1.x and is resolved). Cloning is cheap (reference-counted internally). The canonical pattern from the Tauri community for extern callbacks is `OnceLock` or `once_cell::sync::OnceLock`.

**Example:**
```rust
static HOOK_APP: OnceLock<AppHandle<Wry>> = OnceLock::new();
// In setup():
HOOK_APP.set(app.handle().clone()).ok();
// In dispatcher thread:
let app = HOOK_APP.get().expect("AppHandle not initialized");
handle_shortcut_pressed(app);
```

### Pattern 4: Start Menu Suppression via Return Value

**What:** The Win key fires the Start menu on WM_KEYUP (release), not WM_KEYDOWN. To prevent it: when the hook sees VK_LWIN or VK_RWIN keyup AND the Ctrl+Win combo was consumed (COMBO_ACTIVE was true), return `LRESULT(1)` instead of calling `CallNextHookEx`. This swallows the keyup and the OS never sees it.

**Why this works:** The LowLevelKeyboardProc docs say: "If the hook procedure processed the message, it may return a nonzero value to prevent the system from passing the message to the rest of the hook chain or the target window procedure."

**Belt-and-suspenders note:** The AutoHotkey community documents that Ctrl+Win naturally does not trigger the Start menu on most Windows versions (Win key is only masked if it was pressed and released without any other key in between). The explicit return value suppression ensures this works even on edge-case Windows builds.

**Only suppress when combo was consumed:** If the hook is installed but the user presses Win alone (e.g. Win+E for Explorer), `COMBO_ACTIVE` is false and Win keyup passes through normally.

## Data Flow

### Hotkey Activation Flow (modifier-only path)

```
Physical keyboard: Ctrl keydown, then Win keydown (or any order within 50ms)
    ↓
Win32 kernel delivers each keystroke to all WH_KEYBOARD_LL hooks
    ↓
hook_proc fires on HOOK THREAD for Ctrl keydown
    CTRL_DOWN = true; LAST_MOD_TIME = kb.time
    WIN_DOWN is false → no combo yet; CallNextHookEx
    ↓
hook_proc fires on HOOK THREAD for Win keydown (e.g. 18ms later)
    WIN_DOWN = true; LAST_MOD_TIME = kb.time
    CTRL_DOWN=true, WIN_DOWN=true, age=18ms < 50ms → COMBO!
    COMBO_ACTIVE = true
    try_send(HookEvent::Pressed)   ← non-blocking, returns instantly
    CallNextHookEx (Win keydown still passes through)
    ↓
HOOK DISPATCHER thread receives HookEvent::Pressed
    ↓
handle_shortcut_pressed(&app)   ← calls existing lib.rs handle_shortcut logic
    ↓
PipelineState::IDLE → RECORDING
    audio.clear_buffer(); audio.recording = true
    tray::set_tray_state(Recording); pill::show_pill(); emit "recording"
```

### Hotkey Release Flow

```
Physical keyboard: Win keyup (first modifier released)
    ↓
hook_proc fires on HOOK THREAD for Win keyup
    WIN_DOWN = false
    COMBO_ACTIVE was true → send HookEvent::Released; COMBO_ACTIVE = false
    is_win=true → return LRESULT(1)   ← Start menu SUPPRESSED
    ↓
HOOK DISPATCHER thread receives HookEvent::Released
    ↓
handle_shortcut_released(&app)   ← calls existing lib.rs logic
    ↓
PipelineState::RECORDING → PROCESSING (hold-to-talk mode)
    audio.recording = false; emit "processing"
    tauri::async_runtime::spawn(pipeline::run_pipeline(app))
```

### Startup Flow

```
app.setup() on main thread
    ↓
read_saved_hotkey(app) → "ctrl+win" (modifier-only format)
    ↓
keyboard_hook::start(app.handle().clone())
    ├─ sets HOOK_APP, HOOK_SENDER OnceLocks
    ├─ spawns "hook-dispatcher" std::thread
    └─ spawns "keyboard-hook" std::thread
         └─ SetWindowsHookExW(WH_KEYBOARD_LL, ...)
         └─ GetMessageW loop begins
    ↓
skip tauri-plugin-global-shortcut registration for this path
```

### Shutdown Flow

```
app.on_window_event(CloseRequested) or system shutdown
    ↓
keyboard_hook::stop()
    ├─ tx.send(HookEvent::Stop)         → dispatcher thread exits loop
    └─ PostThreadMessageW(hook_tid, WM_QUIT, ...)
         └─ GetMessageW returns false
         └─ UnhookWindowsHookEx(hook)
         └─ hook thread exits
```

### Hotkey Rebind Flow (user changes hotkey in Settings)

```
Frontend captures new hotkey combo
    ↓
invoke("rebind_hotkey", { old: "ctrl+win", new: "ctrl+shift+space" })
    ↓
lib.rs rebind_hotkey():
    if old was modifier-only:
        keyboard_hook::stop()          → unhooks WH_KEYBOARD_LL
    if new is modifier-only:
        keyboard_hook::start(app)      → installs WH_KEYBOARD_LL
    if new is standard:
        plugin.on_shortcut(new, ...)   → uses RegisterHotKey path
    persist new hotkey to settings.json
```

## Integration Points

### New vs. Existing Components

| Change | File | What |
|--------|------|------|
| NEW module | `src-tauri/src/keyboard_hook.rs` | All WH_KEYBOARD_LL logic (~200 LOC) |
| MODIFIED | `src-tauri/src/lib.rs` | `mod keyboard_hook`; conditional hook startup in `setup()`; routing in `rebind_hotkey`, `register_hotkey`, `unregister_hotkey` commands |
| MODIFIED | `src-tauri/Cargo.toml` | Add `Win32_UI_WindowsAndMessaging` and `Win32_Foundation` windows-rs features |
| MODIFIED | Frontend settings hotkey capture | Accept modifier-only combos (Ctrl+Win with no letter key) |
| KEPT AS-IS | `tauri-plugin-global-shortcut` | Still used for standard hotkeys; fallback if hook install fails |
| KEPT AS-IS | `handle_shortcut()` in `lib.rs` | Zero changes — hook dispatcher calls same function |

### Cargo.toml Changes Required

```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = [
    "Win32_Graphics_Dxgi",              # already present
    "Win32_UI_WindowsAndMessaging",     # ADD: SetWindowsHookExW, GetMessageW, PostThreadMessageW, etc.
    "Win32_Foundation",                 # ADD: LRESULT, WPARAM, LPARAM, BOOL
    "Win32_System_Threading",           # ADD: GetCurrentThreadId
] }
```

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `keyboard_hook.rs` hook proc → dispatcher | `mpsc::Sender::try_send(HookEvent)` | Non-blocking; hook proc must return in <1000ms |
| `keyboard_hook.rs` dispatcher → `lib.rs` | Direct function calls `handle_shortcut_*(&app)` | No channel needed; dispatcher owns the AppHandle |
| `keyboard_hook.rs` → Tauri AppHandle | `OnceLock<AppHandle<Wry>>` | Set once in `setup()`, read-only thereafter |
| Hook thread → shutdown | `PostThreadMessageW(WM_QUIT)` via stored thread ID | Thread ID stored in `AtomicU32` at hook thread startup |
| Frontend → `rebind_hotkey` | Existing IPC command (modified routing) | String format for modifier-only: `"ctrl+win"` (no virtual key suffix) |
| `keyboard_hook.rs` ↔ `tauri-plugin-global-shortcut` | Mutually exclusive registration | Only one active per hotkey; `rebind_hotkey` manages the switch |

## Fallback Strategy

If `SetWindowsHookExW` returns `NULL` (error), log the failure and fall back to registering a composite standard hotkey via `tauri-plugin-global-shortcut`. Suggested fallback: `Ctrl+Win+Space` (still usable, Start menu not triggered). Surface this to the user via a settings warning: "Modifier-only hotkey unavailable — using Ctrl+Win+Space instead."

## Build Order

1. **Add `keyboard_hook.rs` skeleton** — `HookEvent` enum, `OnceLock` statics, stub `start()`/`stop()` that do nothing. Verify compilation.
2. **Implement modifier state machine with unit tests** — `CTRL_DOWN`/`WIN_DOWN` logic and 50ms debounce, tested without any Win32 calls using mock timestamp injection.
3. **Install the hook + GetMessage loop** — `SetWindowsHookExW` with logging only inside the callback (no channel sends yet). Run the app and verify the hook fires for every keystroke (check logs).
4. **Wire the channel** — add `HOOK_SENDER`, `try_send` in hook proc, dispatcher thread that prints received events. Verify Pressed/Released arrive correctly for Ctrl+Win.
5. **Connect to `handle_shortcut`** — replace dispatcher print with actual `handle_shortcut_pressed/released` calls. Verify hold-to-talk and toggle modes work end-to-end with Ctrl+Win.
6. **Start menu suppression** — add `COMBO_ACTIVE` tracking and `return LRESULT(1)` for Win keyup. Verify Start menu does not open.
7. **Shutdown path** — implement `stop()` with `PostThreadMessageW(WM_QUIT)` and `UnhookWindowsHookEx`. Verify clean exit with no hook handle leak.
8. **Rebind integration** — wire `rebind_hotkey` IPC command to call `keyboard_hook::stop()` / `start()` vs. plugin registration based on hotkey format.
9. **Frontend modifier-only capture** — update settings UI to display and store `"ctrl+win"` format combos without requiring a letter key.
10. **Fallback** — if `SetWindowsHookExW` fails, fall back to plugin registration and surface warning in UI.

## Anti-Patterns

### Anti-Pattern 1: Blocking Inside the Hook Proc

**What people do:** Lock a Mutex, call `app.emit()`, allocate, or do any I/O inside `hook_proc`.

**Why it's wrong:** The hook proc has a hard 1000ms timeout on Windows 10 1709+. If it exceeds the timeout, the hook is silently removed permanently with no notification. Mutex contention alone can cause this under load.

**Do this instead:** Use `mpsc::Sender::try_send` (non-blocking) to hand off the event. All actual work happens in the dispatcher thread. If `try_send` returns `Err` (channel full or disconnected), log and discard.

### Anti-Pattern 2: Installing the Hook on a Tokio Task

**What people do:** Call `SetWindowsHookExW` inside `tauri::async_runtime::spawn(async { ... })`.

**Why it's wrong:** Tokio worker threads do not run a Win32 message loop. The hook installs successfully (no error), but the system cannot deliver hook messages to it — the hook silently never fires.

**Do this instead:** Use `std::thread::spawn` for both the hook thread and the dispatcher. They are synchronous threads, not async tasks.

### Anti-Pattern 3: Using GetAsyncKeyState Inside the Hook Proc

**What people do:** Check `GetAsyncKeyState(VK_CONTROL)` to detect if Ctrl is held while processing a Win keydown.

**Why it's wrong:** The LowLevelKeyboardProc documentation explicitly states the asynchronous key state is unreliable inside the callback. Results are indeterminate.

**Do this instead:** Track modifier state with `AtomicBool` updated by the hook proc itself on every VK_LCONTROL / VK_RCONTROL / VK_LWIN / VK_RWIN keydown and keyup.

### Anti-Pattern 4: Trying RegisterHotKey with Modifier-Only Combos

**What people do:** Pass `MOD_CONTROL | MOD_WIN` with `vk = 0` to `RegisterHotKey`, expecting it to work.

**Why it's wrong:** `RegisterHotKey` requires a non-zero virtual key code. The `vk` parameter is mandatory. Passing 0 fails silently or returns an error. This is the root reason WH_KEYBOARD_LL is necessary for this feature.

**Do this instead:** Use `WH_KEYBOARD_LL` as described in this document.

### Anti-Pattern 5: Suppressing All Win Keyups

**What people do:** Return `LRESULT(1)` for every VK_LWIN/VK_RWIN keyup to guarantee Start menu suppression.

**Why it's wrong:** This breaks all Win key shortcuts when the user is not triggering the Ctrl+Win combo. Win+E (Explorer), Win+D (Desktop), Win+L (Lock) all stop working.

**Do this instead:** Only suppress Win keyup when `COMBO_ACTIVE` was true — meaning the hook intentionally consumed the Ctrl+Win combo. All other Win key usage passes through normally.

### Anti-Pattern 6: Forgetting to Unhook on Shutdown

**What people do:** Let the hook thread drop naturally on app exit without calling `UnhookWindowsHookEx`.

**Why it's wrong:** The hook handle is a system resource. On process exit, Windows automatically removes it, but if the process crashes without exiting cleanly, the hook can linger and affect all applications on the desktop until the next login.

**Do this instead:** Always call `UnhookWindowsHookEx(hook)` in the hook thread before it exits. Trigger this via `PostThreadMessageW(hook_tid, WM_QUIT, 0, 0)` which breaks the `GetMessageW` loop cleanly.

## Sources

- [SetWindowsHookExA — Win32 docs](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexa) — thread requirements, bitness rules, dwThreadId=0 scope
- [LowLevelKeyboardProc — Win32 docs](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelkeyboardproc) — 1000ms timeout, message loop requirement, return value semantics
- [KBDLLHOOKSTRUCT — Win32 docs](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-kbdllhookstruct) — vkCode, flags, time fields
- [RegisterHotKey — Win32 docs](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerhotkey) — confirms vk is mandatory; modifier-only combos impossible
- [Tauri Discussion #6309](https://github.com/orgs/tauri-apps/discussions/6309) — OnceLock pattern for AppHandle in extern "system" fn callbacks
- [Tauri Discussion #8538](https://github.com/tauri-apps/tauri/discussions/8538) — AppHandle state access across threads
- [Tauri Issue #2343](https://github.com/tauri-apps/tauri/issues/2343) — historical Send/Sync issue, resolved in Tauri 2.x
- [AutoHotkey community — Win key suppression](https://www.autohotkey.com/boards/viewtopic.php?t=101812) — Ctrl+Win naturally does not trigger Start menu in most cases
- [DaniWeb — system-wide keystroke blocking with WH_KEYBOARD_LL](https://www.daniweb.com/programming/software-development/threads/369970/system-wide-hook-to-block-certain-keystrokes) — return nonzero to swallow keystroke
- [SetWindowsHookExW in windows-docs-rs](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/UI/WindowsAndMessaging/fn.SetWindowsHookExW.html) — Rust binding signature and feature flags

---
*Architecture research for: WH_KEYBOARD_LL integration into Tauri 2.0 voice-to-text (v1.2 milestone)*
*Researched: 2026-03-02*
