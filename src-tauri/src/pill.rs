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

const PILL_WIDTH: i32 = 178;
const PILL_HEIGHT: i32 = 46;

/// Find the work area of the monitor the cursor is on.
/// Returns (wa_x, wa_y, wa_w, wa_h) or None if no monitors detected.
fn cursor_work_area(pill_window: &tauri::WebviewWindow) -> Option<(i32, i32, i32, i32)> {
    let cursor = match pill_window.cursor_position() {
        Ok(pos) => pos,
        Err(e) => {
            log::warn!("Failed to get cursor position: {}, using primary monitor", e);
            tauri::PhysicalPosition { x: 0.0, y: 0.0 }
        }
    };

    let monitors = pill_window.available_monitors().unwrap_or_default();
    let target_monitor = monitors.iter().find(|m| {
        let pos = m.position();
        let size = m.size();
        let (mx, my) = (pos.x as f64, pos.y as f64);
        let (mw, mh) = (size.width as f64, size.height as f64);
        cursor.x >= mx && cursor.x < mx + mw && cursor.y >= my && cursor.y < my + mh
    });

    let work_area = if let Some(mon) = target_monitor {
        mon.work_area().clone()
    } else if let Some(primary) = monitors.first() {
        primary.work_area().clone()
    } else {
        return None;
    };

    Some((
        work_area.position.x,
        work_area.position.y,
        work_area.size.width as i32,
        work_area.size.height as i32,
    ))
}

/// Compute the bottom-center (home) position for the pill on the given work area.
fn home_position(wa_x: i32, wa_y: i32, wa_w: i32, wa_h: i32) -> (i32, i32) {
    let margin_bottom: i32 = 14;
    let x = wa_x + (wa_w - PILL_WIDTH) / 2;
    let y = wa_y + wa_h - PILL_HEIGHT - margin_bottom;
    (x, y)
}

/// Convert fractional offsets (0.0-1.0) to absolute position on the given work area.
fn frac_to_abs(frac_x: f64, frac_y: f64, wa_x: i32, wa_y: i32, wa_w: i32, wa_h: i32) -> (i32, i32) {
    let x = wa_x + (frac_x * (wa_w - PILL_WIDTH) as f64).round() as i32;
    let y = wa_y + (frac_y * (wa_h - PILL_HEIGHT) as f64).round() as i32;
    (x, y)
}

/// Position the pill on the monitor the cursor is on, then emit pill-show.
/// If a saved fractional position exists in settings, apply it to the current
/// monitor's work area. Otherwise, use bottom-center (home) position.
pub fn show_pill(app: &tauri::AppHandle) {
    if let Some(pill_window) = app.get_webview_window("pill") {
        let wa = match cursor_work_area(&pill_window) {
            Some(wa) => wa,
            None => {
                log::warn!("No monitors detected, emitting pill-show without positioning");
                app.emit_to("pill", "pill-show", ()).ok();
                return;
            }
        };
        let (wa_x, wa_y, wa_w, wa_h) = wa;

        // Check for saved fractional position
        let saved_frac = crate::read_settings(app)
            .ok()
            .and_then(|json| {
                let fx = json["pill_position"]["frac_x"].as_f64()?;
                let fy = json["pill_position"]["frac_y"].as_f64()?;
                Some((fx, fy))
            });

        let (x, y) = if let Some((fx, fy)) = saved_frac {
            let pos = frac_to_abs(fx, fy, wa_x, wa_y, wa_w, wa_h);
            log::debug!("Pill restored to frac ({:.3}, {:.3}) -> abs ({}, {})", fx, fy, pos.0, pos.1);
            pos
        } else {
            let pos = home_position(wa_x, wa_y, wa_w, wa_h);
            log::debug!("Pill positioned at bottom-center ({}, {})", pos.0, pos.1);
            pos
        };

        let _ = pill_window.set_position(tauri::PhysicalPosition::new(x, y));
        app.emit_to("pill", "pill-show", ()).ok();
    } else {
        log::warn!("Pill window not found, cannot position or show pill");
    }
}

/// Move the pill window to (x, y). Does NOT persist — position is saved on exit_pill_move_mode.
#[tauri::command]
pub async fn set_pill_position(app: tauri::AppHandle, x: i32, y: i32) -> Result<(), String> {
    let pill_window = app.get_webview_window("pill").ok_or("pill window not found")?;
    pill_window
        .set_position(tauri::PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Clear the saved pill position and reposition to bottom-center home.
#[tauri::command]
pub async fn reset_pill_position(app: tauri::AppHandle) -> Result<(), String> {
    // Remove saved position from settings
    let mut json = crate::read_settings(&app)?;
    if let Some(obj) = json.as_object_mut() {
        obj.remove("pill_position");
    }
    crate::write_settings(&app, &json)?;

    // Reposition to home on current monitor
    if let Some(pill_window) = app.get_webview_window("pill") {
        match cursor_work_area(&pill_window) {
            Some((wa_x, wa_y, wa_w, wa_h)) => {
                let (x, y) = home_position(wa_x, wa_y, wa_w, wa_h);
                let _ = pill_window.set_position(tauri::PhysicalPosition::new(x, y));
                log::debug!("Pill reset to bottom-center ({}, {})", x, y);
            }
            None => {
                log::warn!("No monitors detected, cannot reset pill to home position");
            }
        }
    }
    Ok(())
}

/// Enter pill move mode — sets the PillMoveActive flag and spawns a backend cursor
/// tracking loop. The loop polls cursor_position() every ~16ms and repositions the
/// pill to follow the cursor until exit_pill_move_mode clears the flag.
///
/// Using a backend loop instead of frontend document.addEventListener("mousemove")
/// because the pill webview is only 178x46px — DOM mouse events stop firing the
/// instant the cursor leaves that tiny window, so frontend tracking is unusable.
#[tauri::command]
pub async fn enter_pill_move_mode(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<crate::PillMoveActive>();
    state.0.store(true, std::sync::atomic::Ordering::Relaxed);
    log::info!("Pill move mode: ACTIVE");

    // Spawn the cursor tracking loop using Win32 GetCursorPos for absolute
    // screen coordinates. Tauri's cursor_position() returns window-relative
    // coords which break on multi-monitor setups.
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        use std::sync::atomic::Ordering;
        use windows::Win32::Foundation::POINT;
        use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

        let pill_half_w: i32 = 89;  // 178/2
        let pill_half_h: i32 = 23;  // 46/2

        loop {
            let pill_move = app_clone.state::<crate::PillMoveActive>();
            if !pill_move.0.load(Ordering::Relaxed) {
                break;
            }

            let mut pt = POINT { x: 0, y: 0 };
            let ok = unsafe { GetCursorPos(&mut pt) };
            if ok.is_ok() {
                if let Some(pill_window) = app_clone.get_webview_window("pill") {
                    let x = pt.x - pill_half_w;
                    let y = pt.y - pill_half_h;
                    let _ = pill_window.set_position(tauri::PhysicalPosition::new(x, y));
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(16)).await;
        }

        log::info!("Pill move mode tracking loop: exited");
    });

    Ok(())
}

/// Exit pill move mode — clears the flag (stopping the tracking loop) and persists
/// the final pill position as fractional offsets relative to the current monitor's
/// work area. This allows the position to replicate on any monitor.
#[tauri::command]
pub async fn exit_pill_move_mode(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<crate::PillMoveActive>();
    state.0.store(false, std::sync::atomic::Ordering::Relaxed);
    log::info!("Pill move mode: INACTIVE");

    // Persist fractional position relative to the work area the pill is on
    if let Some(pill_window) = app.get_webview_window("pill") {
        if let Ok(pos) = pill_window.outer_position() {
            // Find which monitor the pill is currently on
            let monitors = pill_window.available_monitors().unwrap_or_default();
            let target_monitor = monitors.iter().find(|m| {
                let mpos = m.position();
                let msize = m.size();
                let center_x = pos.x + PILL_WIDTH / 2;
                let center_y = pos.y + PILL_HEIGHT / 2;
                center_x >= mpos.x
                    && center_x < mpos.x + msize.width as i32
                    && center_y >= mpos.y
                    && center_y < mpos.y + msize.height as i32
            });

            let work_area = if let Some(mon) = target_monitor {
                Some(mon.work_area().clone())
            } else {
                monitors.first().map(|m| m.work_area().clone())
            };

            if let Some(wa) = work_area {
                let wa_x = wa.position.x;
                let wa_y = wa.position.y;
                let wa_w = wa.size.width as i32;
                let wa_h = wa.size.height as i32;

                let usable_w = (wa_w - PILL_WIDTH).max(1) as f64;
                let usable_h = (wa_h - PILL_HEIGHT).max(1) as f64;
                let frac_x = ((pos.x - wa_x) as f64 / usable_w).clamp(0.0, 1.0);
                let frac_y = ((pos.y - wa_y) as f64 / usable_h).clamp(0.0, 1.0);

                let mut json = crate::read_settings(&app).unwrap_or_default();
                json["pill_position"] = serde_json::json!({"frac_x": frac_x, "frac_y": frac_y});
                let _ = crate::write_settings(&app, &json);
                log::debug!("Pill position saved as frac ({:.3}, {:.3})", frac_x, frac_y);
            }
        }
    }

    Ok(())
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
