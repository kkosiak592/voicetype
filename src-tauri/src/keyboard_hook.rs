use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};
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
/// Set once during install(); hook_proc uses try_send (never blocks).
static HOOK_TX: OnceLock<std::sync::mpsc::SyncSender<HookEvent>> = OnceLock::new();

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
    HOOK_TX
        .set(tx)
        .map_err(|_| "HOOK_TX already initialised — install() called twice".to_string())?;

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

/// Low-level keyboard hook procedure.
///
/// MUST be sub-millisecond — no allocation, no Mutex, no async, no sleep.
/// Only AtomicBool/AtomicU32 reads and mpsc::try_send are permitted here.
unsafe extern "system" fn hook_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if ncode < 0 {
        return CallNextHookEx(None, ncode, wparam, lparam);
    }

    let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);

    // Skip injected events (prevents infinite loop when Plan 02 injects VK_E8).
    if (kb.flags.0 & LLKHF_INJECTED.0) != 0 {
        return CallNextHookEx(None, ncode, wparam, lparam);
    }

    // TODO(Plan 02): Modifier state machine + debounce + Start menu suppression
    // For now, pass all events through.
    CallNextHookEx(None, ncode, wparam, lparam)
}

/// Dispatch a HookEvent to the application.
///
/// Plan 03 will wire this to handle_shortcut().
/// For now, log the event for verification.
fn dispatch_hook_event(app: &tauri::AppHandle, event: HookEvent) {
    let _ = app; // used in Plan 03
    match event {
        HookEvent::Pressed => log::info!("Hook dispatch: Pressed"),
        HookEvent::Released => log::info!("Hook dispatch: Released"),
    }
}
