use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
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

/// Update the system tray icon to reflect the current pipeline state.
///
/// Looks up the tray by ID "tray" at runtime — requires build_tray() to have
/// used TrayIconBuilder::with_id("tray") (Pitfall 5 from RESEARCH.md).
/// Failures are silently ignored — tray icon is non-critical feedback.
pub fn set_tray_state(app: &tauri::AppHandle, state: TrayState) {
    let icon_bytes = match state {
        TrayState::Idle => ICON_IDLE,
        TrayState::Recording => ICON_RECORDING,
        TrayState::Processing => ICON_PROCESSING,
    };
    if let Some(tray) = app.tray_by_id("tray") {
        if let Ok(image) = tauri::image::Image::from_bytes(icon_bytes) {
            let _ = tray.set_icon(Some(image));
        }
    }
}

pub fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let settings_i = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings_i, &quit_i])?;

    // Use with_id("tray") so tray_by_id("tray") works at runtime for icon state changes.
    // Without an ID, tray_by_id() returns None and set_tray_state() silently does nothing.
    TrayIconBuilder::with_id("tray")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false) // left-click does nothing per spec
        .on_menu_event(|app, event| match event.id.as_ref() {
            "settings" => {
                if let Some(w) = app.get_webview_window("settings") {
                    w.show().unwrap();
                    w.set_focus().unwrap();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}
