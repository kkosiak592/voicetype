use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Emitter;
use tauri::Manager;

/// Start a ~30fps loop that reads the audio buffer, computes RMS, and emits
/// normalized level (0.0-1.0) to the pill window.
///
/// The loop runs until `active` is set to false. Call this when entering
/// RECORDING state and stop it when leaving RECORDING state.
pub fn start_level_stream(
    app: tauri::AppHandle,
    buffer: Arc<std::sync::Mutex<Vec<f32>>>,
    active: Arc<AtomicBool>,
) {
    tauri::async_runtime::spawn(async move {
        while active.load(Ordering::Relaxed) {
            let level = if let Ok(buf) = buffer.try_lock() {
                compute_rms(&buf, 512)
            } else {
                0.0
            };
            let _ = app.emit_to("pill", "pill-level", level);
            tokio::time::sleep(std::time::Duration::from_millis(33)).await;
        }
        // Send a final zero level when stopping
        let _ = app.emit_to("pill", "pill-level", 0.0_f32);
    });
}

/// Position the pill at bottom-center of the monitor the cursor is on,
/// then emit pill-show to make it visible.
pub fn show_pill(app: &tauri::AppHandle) {
    if let Some(pill_window) = app.get_webview_window("pill") {
        // 1. Get cursor position
        let cursor = match pill_window.cursor_position() {
            Ok(pos) => pos,
            Err(e) => {
                log::warn!("Failed to get cursor position: {}, using primary monitor", e);
                tauri::PhysicalPosition { x: 0.0, y: 0.0 }
            }
        };

        // 2. Find which monitor the cursor is on
        let monitors = pill_window.available_monitors().unwrap_or_default();
        let target_monitor = monitors.iter().find(|m| {
            let pos = m.position();
            let size = m.size();
            let (mx, my) = (pos.x as f64, pos.y as f64);
            let (mw, mh) = (size.width as f64, size.height as f64);
            cursor.x >= mx && cursor.x < mx + mw && cursor.y >= my && cursor.y < my + mh
        });

        // 3. Get the work area (excludes taskbar) of that monitor.
        // Fall back to primary monitor if cursor isn't found on any.
        let work_area = if let Some(mon) = target_monitor {
            mon.work_area().clone()
        } else if let Some(primary) = monitors.first() {
            primary.work_area().clone()
        } else {
            log::warn!("No monitors detected, emitting pill-show without positioning");
            app.emit_to("pill", "pill-show", ()).ok();
            return;
        };

        // 4. Calculate bottom-center position within work area.
        // Pill dimensions: 178 x 46 (from tauri.conf.json)
        let pill_width: i32 = 178;
        let pill_height: i32 = 46;
        let margin_bottom: i32 = 14; // pixels above bottom of work area
        let wa_x = work_area.position.x;
        let wa_y = work_area.position.y;
        let wa_w = work_area.size.width as i32;
        let wa_h = work_area.size.height as i32;

        let x = wa_x + (wa_w - pill_width) / 2;
        let y = wa_y + wa_h - pill_height - margin_bottom;

        let _ = pill_window.set_position(tauri::PhysicalPosition::new(x, y));
        log::debug!("Pill positioned at ({}, {}) on monitor work area", x, y);

        // Emit pill-show AFTER positioning
        app.emit_to("pill", "pill-show", ()).ok();
    } else {
        log::warn!("Pill window not found, cannot position or show pill");
    }
}

/// Compute normalized RMS from the last `window` samples of the buffer.
///
/// Returns 0.0-1.0 where typical speech is 0.45-1.0.
/// Uses a 15x multiplier: speech RMS is usually 0.01-0.1, so * 15 normalizes
/// to a higher range (0.15-1.0) for more prominent bar reactivity.
fn compute_rms(buf: &[f32], window: usize) -> f32 {
    if buf.is_empty() {
        return 0.0;
    }
    let n = buf.len().min(window);
    let tail = &buf[buf.len() - n..];
    let mean_sq: f32 = tail.iter().map(|&s| s * s).sum::<f32>() / n as f32;
    (mean_sq.sqrt() * 15.0).min(1.0)
}
