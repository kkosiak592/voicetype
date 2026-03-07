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
///   1. Write `text` to clipboard with verify-and-retry loop (up to 5 attempts):
///      - set_text() -> sleep 25ms -> get_text() -> compare
///      - Retries on mismatch (handles Chromium WebView clipboard races)
///   2. Sleep 150ms — let Office apps sync their internal clipboard cache
///   3. Simulate Ctrl+V (with defensive Win key release)
///
/// After injection, the transcription text remains on the clipboard. Users can
/// re-paste via Ctrl+V in any application.
///
/// Intentionally synchronous — callers must wrap in tokio::task::spawn_blocking.
/// A fresh Enigo instance is created per call (do not share across invocations).
pub fn inject_text(text: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;

    // Write transcription to clipboard, then verify it was actually set.
    //
    // Why verify: Windows clipboard propagation is async. arboard::set_text() returning Ok()
    // does not guarantee the data is visible to other processes. Additionally, if the Chromium
    // WebView recently called navigator.clipboard.writeText() (e.g., user copied from history),
    // it may reclaim clipboard ownership, silently overwriting our content. A fixed sleep (the
    // previous 50ms approach) cannot detect this race. Instead, we read the clipboard back and
    // confirm it matches the intended text before proceeding to Ctrl+V.
    const MAX_CLIPBOARD_RETRIES: u32 = 5;
    const CLIPBOARD_RETRY_DELAY_MS: u64 = 25;

    let mut clipboard_verified = false;
    for attempt in 0..MAX_CLIPBOARD_RETRIES {
        clipboard.set_text(text).map_err(|e| e.to_string())?;

        // Small delay to allow clipboard propagation before read-back
        thread::sleep(Duration::from_millis(CLIPBOARD_RETRY_DELAY_MS));

        match clipboard.get_text() {
            Ok(ref contents) if contents == text => {
                if attempt > 0 {
                    log::info!(
                        "inject_text: clipboard verified on retry {} (after {} ms total)",
                        attempt,
                        (attempt + 1) as u64 * CLIPBOARD_RETRY_DELAY_MS
                    );
                }
                clipboard_verified = true;
                break;
            }
            Ok(ref contents) => {
                log::warn!(
                    "inject_text: clipboard mismatch on attempt {} — expected {} bytes, got {} bytes (first 80 chars: {:?})",
                    attempt + 1,
                    text.len(),
                    contents.len(),
                    &contents[..contents.len().min(80)]
                );
            }
            Err(e) => {
                log::warn!(
                    "inject_text: clipboard read-back failed on attempt {}: {}",
                    attempt + 1,
                    e
                );
            }
        }
    }

    if !clipboard_verified {
        log::error!(
            "inject_text: clipboard verification failed after {} attempts — proceeding with paste anyway (best-effort)",
            MAX_CLIPBOARD_RETRIES
        );
    }

    // Delay between clipboard write and Ctrl+V keystroke.
    //
    // Why: Some applications (notably Outlook and other Office apps) maintain an internal
    // clipboard cache and process WM_CLIPBOARDUPDATE asynchronously. Even though the OS
    // clipboard is verified correct at this point, Outlook may not have ingested the new
    // content yet. Without this delay, the synthetic Ctrl+V arrives before Outlook reads
    // the updated clipboard, causing it to paste stale cached content.
    //
    // 150ms is chosen as a conservative value — WM_CLIPBOARDUPDATE processing in Office
    // apps typically completes within 50-100ms, but we add margin for slower machines.
    thread::sleep(Duration::from_millis(150));

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

    Ok(())
}
