use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::System::Threading::GetCurrentThreadId;

/// Events sent from the hook callback to the dispatcher thread.
pub enum HookEvent {
    Pressed,
    Released,
}

/// Static sender shared between install() and hook_proc.
///
/// Stored as `Mutex<Option<SyncSender>>` (not OnceLock) so that:
/// 1. install() can be called again after a previous hook is torn down and
///    the old sender cleared — necessary if the hook thread exits unexpectedly
///    and must be reinstalled within the same process lifetime.
/// 2. hook_proc reads it with try_lock() which never blocks (no contention:
///    the hook thread only fires after install() has already set the sender
///    and released the lock, so the Mutex is always uncontested in the callback).
///
/// Replacing the previous OnceLock<SyncSender> which permanently prevented
/// a second install() call — the silent Err return from OnceLock::set() would
/// cause the hook to appear installed but fire no events.
static HOOK_TX: std::sync::Mutex<Option<std::sync::mpsc::SyncSender<HookEvent>>> =
    std::sync::Mutex::new(None);

/// Modifier state tracked by the hook callback.
/// All fields are atomic — hook_proc is called from a single thread but
/// must be lock-free for timing guarantees (HOOK-02: <5ms).
struct ModifierState {
    ctrl_held: AtomicBool,      // Any Ctrl (left or right) is down
    win_held: AtomicBool,       // Any Win (left or right) is down
    shift_held: AtomicBool,     // Any Shift is down (for exact-match check)
    alt_held: AtomicBool,       // Any Alt is down (for exact-match check)
    combo_active: AtomicBool,   // Ctrl+Win combo is currently active (recording)
    first_key_time: AtomicU32,  // Timestamp (ms) of first modifier keydown in potential combo
}

static STATE: ModifierState = ModifierState {
    ctrl_held: AtomicBool::new(false),
    win_held: AtomicBool::new(false),
    shift_held: AtomicBool::new(false),
    alt_held: AtomicBool::new(false),
    combo_active: AtomicBool::new(false),
    first_key_time: AtomicU32::new(0),
};

/// Owns the hook thread ID and provides clean shutdown via uninstall().
pub struct HookHandle {
    thread_id: Arc<AtomicU32>,
    _join_handle: std::thread::JoinHandle<()>,
}

impl HookHandle {
    /// Signal the hook thread to exit. Posts WM_QUIT to its message loop,
    /// which causes GetMessageW to return 0 and UnhookWindowsHookEx to run.
    pub fn uninstall(&self) {
        let tid = self.thread_id.load(Ordering::Acquire);
        if tid != 0 {
            log::info!("Hook uninstall requested");
            unsafe {
                let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
    }
}

impl Drop for HookHandle {
    /// Safety net: if the handle is dropped without an explicit uninstall(),
    /// send WM_QUIT so the hook thread exits and does not dangle.
    fn drop(&mut self) {
        self.uninstall();
    }
}

/// Install the WH_KEYBOARD_LL hook.
///
/// Spawns two threads:
/// - "keyboard-hook": installs SetWindowsHookExW, runs GetMessageW loop,
///   calls UnhookWindowsHookEx on exit.
/// - "hook-dispatcher": receives HookEvents from the channel and dispatches them
///   to the application via dispatch_hook_event().
///
/// Returns a HookHandle whose Drop impl ensures cleanup.
pub fn install(app_handle: tauri::AppHandle) -> Result<HookHandle, String> {
    // Bounded channel — hook_proc uses try_send so it never blocks in the callback.
    let (tx, rx) = std::sync::mpsc::sync_channel::<HookEvent>(32);

    // Store the sender under the Mutex. Replaces any previous sender (e.g., from a
    // previous install() call that has since been torn down). The old SyncSender, if
    // any, is dropped here — its receiver end was already dropped when the previous
    // dispatcher thread exited, so this is safe.
    {
        let mut guard = HOOK_TX
            .lock()
            .map_err(|_| "HOOK_TX mutex poisoned — cannot install hook".to_string())?;
        *guard = Some(tx);
    }

    let thread_id = Arc::new(AtomicU32::new(0));
    let thread_id_clone = Arc::clone(&thread_id);

    let join_handle = std::thread::Builder::new()
        .name("keyboard-hook".to_string())
        .spawn(move || {
            // Store our OS thread ID so HookHandle::uninstall() can post WM_QUIT.
            let tid = unsafe { GetCurrentThreadId() };
            thread_id_clone.store(tid, Ordering::Release);

            // Install the global low-level keyboard hook.
            // hmod=None + dwThreadId=0 → global scope (all threads on the desktop).
            let hook = unsafe {
                SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), None, 0)
            };

            let hook = match hook {
                Ok(h) => h,
                Err(e) => {
                    log::error!("SetWindowsHookExW failed: {:?}", e);
                    return;
                }
            };

            log::info!("WH_KEYBOARD_LL hook installed (thread {})", tid);

            // GetMessageW loop — required to keep the hook alive.
            // Windows silently removes hooks whose thread does not pump messages.
            let mut msg = MSG::default();
            loop {
                let ret = unsafe { GetMessageW(&mut msg, None, 0, 0) };
                match ret.0 {
                    0 => break,  // WM_QUIT received
                    -1 => {
                        log::error!("GetMessageW returned error");
                        break;
                    }
                    _ => {} // Translate/dispatch not needed for thread messages
                }
            }

            // Reset all modifier state on exit to prevent stale state on re-install.
            reset_state();

            // Clear the sender from HOOK_TX so a future install() call can set a new one.
            // After this, hook_proc's try_lock will see None and drop events silently —
            // safe because UnhookWindowsHookEx below prevents hook_proc from being called
            // after this point. The try_lock + None guard in hook_proc handles any race.
            if let Ok(mut guard) = HOOK_TX.try_lock() {
                *guard = None;
            }

            // Clean up the hook before the thread exits.
            let _ = unsafe { UnhookWindowsHookEx(hook) };
            log::info!("Hook thread exiting");
        })
        .map_err(|e| format!("Failed to spawn keyboard-hook thread: {}", e))?;

    // Dispatcher thread — receives events and calls into the application.
    std::thread::Builder::new()
        .name("hook-dispatcher".to_string())
        .spawn(move || {
            while let Ok(event) = rx.recv() {
                dispatch_hook_event(&app_handle, event);
            }
            log::info!("Hook dispatcher thread exiting");
        })
        .map_err(|e| format!("Failed to spawn hook-dispatcher thread: {}", e))?;

    Ok(HookHandle {
        thread_id,
        _join_handle: join_handle,
    })
}

/// Inject VK_E8 (unassigned key) via SendInput to break the Win key's
/// Start menu activation sequence. The system sees an intervening keypress
/// between Win-down and Win-up, so it does not open the Start menu.
///
/// Uses KEYDOWN only (standard AHK MenuMaskKey behavior).
/// If Windows 11 testing shows Start menu still opens, add KEYUP injection
/// as fallback (see 15-RESEARCH.md Open Question 1).
unsafe fn inject_mask_key() {
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0xE8),  // VK_E8: unassigned, stable across Windows versions
                wScan: 0,
                dwFlags: KEYBD_EVENT_FLAGS(0),  // KEYDOWN (dwFlags=0 means keydown)
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
}

/// Reset all modifier state to defaults. Called on uninstall or error recovery.
fn reset_state() {
    STATE.ctrl_held.store(false, Ordering::Relaxed);
    STATE.win_held.store(false, Ordering::Relaxed);
    STATE.shift_held.store(false, Ordering::Relaxed);
    STATE.alt_held.store(false, Ordering::Relaxed);
    STATE.combo_active.store(false, Ordering::Relaxed);
    STATE.first_key_time.store(0, Ordering::Relaxed);
}

/// Low-level keyboard hook procedure.
///
/// MUST be sub-millisecond — no allocation, no blocking Mutex, no async, no sleep.
/// Uses AtomicBool/AtomicU32 reads, mpsc::try_send, and HOOK_TX.try_lock() (non-blocking —
/// never contended at call time because the Mutex is only held briefly during install/teardown,
/// not during normal operation).
unsafe extern "system" fn hook_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if ncode < 0 {
        return CallNextHookEx(None, ncode, wparam, lparam);
    }

    let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);

    // Skip injected events (prevents infinite loop from VK_E8 injection).
    if (kb.flags.0 & LLKHF_INJECTED.0) != 0 {
        return CallNextHookEx(None, ncode, wparam, lparam);
    }

    let vk = VIRTUAL_KEY(kb.vkCode as u16);
    let msg = wparam.0 as u32;
    let is_down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
    let is_up = msg == WM_KEYUP || msg == WM_SYSKEYUP;

    // --- Shift tracking (exact-match enforcement) ---
    if vk == VK_LSHIFT || vk == VK_RSHIFT || vk == VK_SHIFT {
        STATE.shift_held.store(is_down, Ordering::Relaxed);
        return CallNextHookEx(None, ncode, wparam, lparam);
    }

    // --- Alt tracking (exact-match enforcement) ---
    if vk == VK_LMENU || vk == VK_RMENU || vk == VK_MENU {
        STATE.alt_held.store(is_down, Ordering::Relaxed);
        return CallNextHookEx(None, ncode, wparam, lparam);
    }

    // --- Ctrl keydown ---
    if is_down && (vk == VK_LCONTROL || vk == VK_RCONTROL) {
        STATE.ctrl_held.store(true, Ordering::Relaxed);

        if STATE.win_held.load(Ordering::Relaxed) && !STATE.combo_active.load(Ordering::Relaxed) {
            // Win was pressed first — check debounce and exact-match
            let elapsed = kb.time.wrapping_sub(STATE.first_key_time.load(Ordering::Relaxed));
            let no_extra = !STATE.shift_held.load(Ordering::Relaxed)
                && !STATE.alt_held.load(Ordering::Relaxed);
            if elapsed <= 50 && no_extra {
                STATE.combo_active.store(true, Ordering::Relaxed);
                if let Ok(guard) = HOOK_TX.try_lock() {
                    if let Some(ref tx) = *guard {
                        if let Err(e) = tx.try_send(HookEvent::Pressed) {
                            log::warn!("Hook channel full — Pressed event dropped: {:?}", e);
                        }
                    }
                }
            }
        } else if !STATE.win_held.load(Ordering::Relaxed) {
            // Ctrl was first key — record timestamp for debounce
            STATE.first_key_time.store(kb.time, Ordering::Relaxed);
        }

        return CallNextHookEx(None, ncode, wparam, lparam);
    }

    // --- Ctrl keyup ---
    if is_up && (vk == VK_LCONTROL || vk == VK_RCONTROL) {
        STATE.ctrl_held.store(false, Ordering::Relaxed);

        if STATE.combo_active.load(Ordering::Relaxed) {
            STATE.combo_active.store(false, Ordering::Relaxed);
            if let Ok(guard) = HOOK_TX.try_lock() {
                if let Some(ref tx) = *guard {
                    if let Err(e) = tx.try_send(HookEvent::Released) {
                        log::warn!("Hook channel full — Released event dropped: {:?}", e);
                    }
                }
            }
        }

        return CallNextHookEx(None, ncode, wparam, lparam);
    }

    // --- Win keydown ---
    if is_down && (vk == VK_LWIN || vk == VK_RWIN) {
        // If combo is already active, suppress repeated Win keydown
        if STATE.combo_active.load(Ordering::Relaxed) {
            inject_mask_key();
            return LRESULT(1);
        }

        STATE.win_held.store(true, Ordering::Relaxed);

        if STATE.ctrl_held.load(Ordering::Relaxed) && !STATE.combo_active.load(Ordering::Relaxed) {
            // Ctrl was pressed first — check debounce and exact-match
            let elapsed = kb.time.wrapping_sub(STATE.first_key_time.load(Ordering::Relaxed));
            let no_extra = !STATE.shift_held.load(Ordering::Relaxed)
                && !STATE.alt_held.load(Ordering::Relaxed);
            if elapsed <= 50 && no_extra {
                STATE.combo_active.store(true, Ordering::Relaxed);
                inject_mask_key();
                if let Ok(guard) = HOOK_TX.try_lock() {
                    if let Some(ref tx) = *guard {
                        if let Err(e) = tx.try_send(HookEvent::Pressed) {
                            log::warn!("Hook channel full — Pressed event dropped: {:?}", e);
                        }
                    }
                }
                return LRESULT(1); // Suppress Win keydown
            }
        } else if !STATE.ctrl_held.load(Ordering::Relaxed) {
            // Win was first key — record timestamp for debounce
            STATE.first_key_time.store(kb.time, Ordering::Relaxed);
        }

        // Combo did not fire — pass Win keydown through (Win alone opens Start menu, MOD-05)
        return CallNextHookEx(None, ncode, wparam, lparam);
    }

    // --- Win keyup ---
    if is_up && (vk == VK_LWIN || vk == VK_RWIN) {
        STATE.win_held.store(false, Ordering::Relaxed);

        if STATE.combo_active.load(Ordering::Relaxed) {
            STATE.combo_active.store(false, Ordering::Relaxed);
            if let Ok(guard) = HOOK_TX.try_lock() {
                if let Some(ref tx) = *guard {
                    if let Err(e) = tx.try_send(HookEvent::Released) {
                        log::warn!("Hook channel full — Released event dropped: {:?}", e);
                    }
                }
            }
            // Suppress Win keyup to prevent Start menu opening (MOD-04)
            return LRESULT(1);
        }

        // Combo was not active — pass Win keyup through (Start menu opens normally, MOD-05)
        return CallNextHookEx(None, ncode, wparam, lparam);
    }

    // All other keys pass through without modification
    CallNextHookEx(None, ncode, wparam, lparam)
}

/// Dispatch a HookEvent to the application by calling handle_hotkey_event().
///
/// Both the WH_KEYBOARD_LL hook path (this function) and the global-shortcut path
/// (handle_shortcut in lib.rs) converge on handle_hotkey_event, avoiding the need
/// to construct the private ShortcutEvent type from tauri-plugin-global-shortcut.
fn dispatch_hook_event(app: &tauri::AppHandle, event: HookEvent) {
    match event {
        HookEvent::Pressed => crate::handle_hotkey_event(app, true),
        HookEvent::Released => crate::handle_hotkey_event(app, false),
    }
}
