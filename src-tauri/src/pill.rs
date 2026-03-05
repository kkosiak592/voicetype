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

/// Compute the bottom-center position for the pill on the monitor the cursor is on.
/// Returns None if monitors cannot be detected.
fn compute_home_position(pill_window: &tauri::WebviewWindow) -> Option<(i32, i32)> {
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
    let work_area = if let Some(mon) = target_monitor {
        mon.work_area().clone()
    } else if let Some(primary) = monitors.first() {
        primary.work_area().clone()
    } else {
        return None;
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
    Some((x, y))
}

/// Position the pill at bottom-center of the monitor the cursor is on,
/// then emit pill-show to make it visible.
/// If a saved position exists in settings, use that instead of recomputing.
pub fn show_pill(app: &tauri::AppHandle) {
    if let Some(pill_window) = app.get_webview_window("pill") {
        // Check for a saved position first
        let saved_pos = crate::read_settings(app)
            .ok()
            .and_then(|json| {
                let x = json["pill_position"]["x"].as_i64()?;
                let y = json["pill_position"]["y"].as_i64()?;
                Some((x as i32, y as i32))
            });

        let (x, y) = if let Some((sx, sy)) = saved_pos {
            log::debug!("Pill restored to saved position ({}, {})", sx, sy);
            (sx, sy)
        } else {
            match compute_home_position(&pill_window) {
                Some(pos) => {
                    log::debug!("Pill positioned at bottom-center ({}, {})", pos.0, pos.1);
                    pos
                }
                None => {
                    log::warn!("No monitors detected, emitting pill-show without positioning");
                    app.emit_to("pill", "pill-show", ()).ok();
                    return;
                }
            }
        };

        let _ = pill_window.set_position(tauri::PhysicalPosition::new(x, y));

        // Emit pill-show AFTER positioning
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

    // Reposition to home
    if let Some(pill_window) = app.get_webview_window("pill") {
        match compute_home_position(&pill_window) {
            Some((x, y)) => {
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
/// the final pill position to settings.json.
#[tauri::command]
pub async fn exit_pill_move_mode(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<crate::PillMoveActive>();
    state.0.store(false, std::sync::atomic::Ordering::Relaxed);
    log::info!("Pill move mode: INACTIVE");

    // Persist the final position so the pill reopens here next time
    if let Some(pill_window) = app.get_webview_window("pill") {
        if let Ok(pos) = pill_window.outer_position() {
            let mut json = crate::read_settings(&app).unwrap_or_default();
            json["pill_position"] = serde_json::json!({"x": pos.x, "y": pos.y});
            let _ = crate::write_settings(&app, &json);
            log::debug!("Pill position saved: ({}, {})", pos.x, pos.y);
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
