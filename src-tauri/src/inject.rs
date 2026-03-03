use arboard::Clipboard;
use enigo::{Direction::{Click, Press, Release}, Enigo, Key, Keyboard, Settings};
use std::thread;
use std::time::Duration;

/// Release both LWIN and RWIN via Enigo before simulating Ctrl+V.
///
/// This is a defensive layer against the Win-key-stuck failure mode: if the
/// keyboard_hook passes Win-down through to the OS (Win-first press order) and the
/// synthetic Win-up injection in keyboard_hook.rs somehow fails or races, inject_text
/// would fire Ctrl+V while the OS still considers Win held — producing Win+Ctrl+V
/// (not a paste shortcut), causing a silent paste failure.
///
/// Releasing a key that is not actually held is a no-op at the OS level, so this is
/// always safe to call. Errors are logged and ignored: if we cannot release the Win key
/// via Enigo, we proceed with the paste attempt anyway (it may still succeed).
fn release_win_keys(enigo: &mut Enigo) {
    if let Err(e) = enigo.key(Key::LWin, Release) {
        log::warn!("inject_text: failed to release LWin before paste: {}", e);
    }
    if let Err(e) = enigo.key(Key::RWin, Release) {
        log::warn!("inject_text: failed to release RWin before paste: {}", e);
    }
}

/// Inject text at the current cursor position using clipboard paste.
///
/// Sequence:
///   1. Save existing clipboard content (None if non-text or empty)
///   2. Write `text` to clipboard
///   3. Sleep 50ms — Windows clipboard propagation delay
///   4. Simulate Ctrl+V
///   5. Sleep 80ms — let target app consume paste before restore
///   6. Restore original clipboard (log warning on failure, do not error)
///
/// Intentionally synchronous — callers must wrap in tokio::task::spawn_blocking.
/// A fresh Enigo instance is created per call (do not share across invocations).
pub fn inject_text(text: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;

    // Save existing clipboard content — .ok() converts Err (non-text content) to None
    let saved: Option<String> = clipboard.get_text().ok();

    // Write transcription to clipboard
    clipboard.set_text(text).map_err(|e| e.to_string())?;

    // Allow clipboard write to propagate before paste (Windows requirement)
    // 50ms clipboard propagation. Previously reduced to 30ms but intermittent paste failures
    // were observed under CPU load (transcription). Reverted to 50ms per the documented
    // fallback guidance. Windows clipboard propagation is async; arboard set_text() returning
    // Ok() does not guarantee the data is visible to other processes yet.
    thread::sleep(Duration::from_millis(50));

    // Simulate Ctrl+V — fresh Enigo instance per call (anti-pattern: sharing instances)
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;

    // Defensive: release Win keys before paste. If the hook's Win-first press path
    // passed Win-down through to the OS and the synthetic Win-up injection raced or
    // failed, the OS would treat our Ctrl+V as Win+Ctrl+V — a silent paste failure.
    // Releasing a key that is not held is a no-op; this is always safe to call.
    release_win_keys(&mut enigo);

    // Hold Ctrl, click V, release Ctrl — ensuring Ctrl is always released even if
    // V-click fails (previously ? on the middle call would early-return with Ctrl held).
    enigo.key(Key::Control, Press).map_err(|e| e.to_string())?;
    let v_result = enigo.key(Key::Unicode('v'), Click).map_err(|e| e.to_string());
    enigo.key(Key::Control, Release).map_err(|e| e.to_string())?;
    v_result?;

    // Allow target app to consume the paste before clipboard restore
    // 80ms paste consumption. Previously reduced to 50ms alongside the propagation delay
    // reduction; reverting to 80ms to match the documented fallback guidance.
    thread::sleep(Duration::from_millis(80));

    // Restore original clipboard content — per user decision: log failure, move on
    match saved {
        Some(original) => {
            if let Err(e) = clipboard.set_text(&original) {
                log::warn!(
                    "inject_text: clipboard restore failed: {} — clipboard contents lost",
                    e
                );
            }
        }
        None => {
            // Original was empty or non-text — clear by setting empty string
            // arboard has no explicit clear() method; empty string is the fallback
            let _ = clipboard.set_text("");
        }
    }

    Ok(())
}
