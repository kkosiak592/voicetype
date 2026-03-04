use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    Manager,
};

/// Three visual states for the system tray icon.
///
/// - Idle: normal/neutral state — waiting for hotkey
/// - Recording: hotkey held — actively capturing audio (red icon)
/// - Processing: hotkey released — whisper inference in progress (orange icon)
pub enum TrayState {
    Idle,
    Recording,
    Processing,
}

// Icon bytes embedded at compile time.
// PNG format is accepted by tauri::image::Image::from_bytes.
static ICON_IDLE: &[u8] = include_bytes!("../icons/tray-idle.png");
static ICON_RECORDING: &[u8] = include_bytes!("../icons/tray-recording.png");
static ICON_PROCESSING: &[u8] = include_bytes!("../icons/tray-processing.png");

/// Update the system tray icon and tooltip to reflect the current pipeline state.
///
/// Looks up the tray by ID "tray" at runtime — requires build_tray() to have
/// used TrayIconBuilder::with_id("tray") (Pitfall 5 from RESEARCH.md).
/// Failures are silently ignored — tray icon is non-critical feedback.
pub fn set_tray_state(app: &tauri::AppHandle, state: TrayState) {
    let (icon_bytes, tooltip) = match state {
        TrayState::Idle => (ICON_IDLE, "VoiceType - Idle"),
        TrayState::Recording => (ICON_RECORDING, "VoiceType - Recording"),
        TrayState::Processing => (ICON_PROCESSING, "VoiceType - Processing"),
    };
    if let Some(tray) = app.tray_by_id("tray") {
        if let Ok(image) = tauri::image::Image::from_bytes(icon_bytes) {
            let _ = tray.set_icon(Some(image));
        }
        let _ = tray.set_tooltip(Some(tooltip));
    }
}

/// Show or hide the "Update Available" indicator in the tray context menu.
///
/// When `available == true`, prepends an "Update Available" item above Settings.
/// When `available == false`, restores the default menu (Settings + Quit only).
///
/// Tauri 2 tray menus are immutable after creation — a new Menu is constructed
/// and swapped in via `tray.set_menu(Some(new_menu))`.
///
/// Clicking "Update Available" opens the settings window (same as "settings" item).
/// Failures are silently ignored — tray menu is non-critical UI.
pub fn set_tray_update_indicator(app: &tauri::AppHandle, available: bool) {
    let Some(tray) = app.tray_by_id("tray") else {
        return;
    };

    let result: tauri::Result<()> = (|| {
        if available {
            let update_i = MenuItem::with_id(app, "update_available", "Update Available", true, None::<&str>)?;
            let settings_i = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&update_i, &settings_i, &quit_i])?;
            tray.set_menu(Some(menu))?;
        } else {
            let settings_i = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings_i, &quit_i])?;
            tray.set_menu(Some(menu))?;
        }
        Ok(())
    })();

    if let Err(e) = result {
        log::warn!("Failed to update tray menu for update indicator: {}", e);
    }
}

/// Destroy the tray icon explicitly before relaunch.
/// Prevents Windows from showing a stale icon alongside the new process's icon.
/// Windows defers tray icon cleanup by ~200ms after process exit; calling this
/// before relaunch() ensures the icon is gone before the new process registers its own.
pub fn destroy_tray(app: &tauri::AppHandle) {
    if let Some(tray) = app.tray_by_id("tray") {
        let _ = tray.set_visible(false);
    }
}

pub fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let settings_i = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings_i, &quit_i])?;

    // Use with_id("tray") so tray_by_id("tray") works at runtime for icon state changes.
    // Without an ID, tray_by_id() returns None and set_tray_state() silently does nothing.
    // Use Image::from_bytes(ICON_IDLE) — same source as set_tray_state() — to prevent
    // Windows registering two separate HICON entries (duplicate tray icon bug).
    TrayIconBuilder::with_id("tray")
        .icon(tauri::image::Image::from_bytes(ICON_IDLE)?)
        .tooltip("VoiceType - Idle")
        .menu(&menu)
        .show_menu_on_left_click(false) // left-click does nothing per spec
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::DoubleClick { button, .. } = event {
                if button == MouseButton::Left {
                    let app = tray.app_handle();
                    if let Some(w) = app.get_webview_window("settings") {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                }
            }
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "settings" | "update_available" => {
                if let Some(w) = app.get_webview_window("settings") {
                    w.show().unwrap();
                    w.set_focus().unwrap();
                }
            }
            "quit" => {
                // Explicitly uninstall the keyboard hook before exit.
                // HookHandle::Drop is the safety net; this ensures clean shutdown
                // even if managed state outlives Drop ordering.
                #[cfg(windows)]
                {
                    use crate::HookHandleState;
                    if let Some(hook_state) = app.try_state::<HookHandleState>() {
                        let guard = hook_state.0.lock().unwrap_or_else(|e| e.into_inner());
                        if let Some(ref handle) = *guard {
                            handle.uninstall();
                            log::info!("Keyboard hook uninstalled on quit");
                        }
                    }
                }
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}
