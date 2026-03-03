use arboard::Clipboard;
use enigo::{Direction::{Click, Press, Release}, Enigo, Key, Keyboard, Settings};
use std::thread;
use std::time::Duration;

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
    enigo.key(Key::Control, Press).map_err(|e| e.to_string())?;
    enigo.key(Key::Unicode('v'), Click).map_err(|e| e.to_string())?;
    enigo.key(Key::Control, Release).map_err(|e| e.to_string())?;

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
