mod tray;

use tauri::{Emitter, Manager};
use tray::build_tray;

/// Read the saved hotkey from settings.json in the app data directory.
/// Returns None on first launch, file missing, or parse error — callers fall back to default.
fn read_saved_hotkey(app: &tauri::App) -> Option<String> {
    let data_dir = app.path().app_data_dir().ok()?;
    let settings_path = data_dir.join("settings.json");

    let contents = std::fs::read_to_string(&settings_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&contents).ok()?;

    json.get("hotkey")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned())
}

#[tauri::command]
fn rebind_hotkey(app: tauri::AppHandle, old: String, new_key: String) -> Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    let gs = app.global_shortcut();

    if !old.is_empty() {
        gs.unregister(old.as_str()).map_err(|e| e.to_string())?;
    }

    gs.on_shortcut(new_key.as_str(), |app, _shortcut, event| {
        use tauri_plugin_global_shortcut::ShortcutState;
        if event.state == ShortcutState::Pressed {
            let _ = app.emit("hotkey-triggered", ());
        }
    })
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // single-instance MUST be registered first (before setup)
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Second instance launched — show and focus existing settings window
            if let Some(w) = app.get_webview_window("settings") {
                w.show().unwrap();
                w.set_focus().unwrap();
            }
        }))
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .invoke_handler(tauri::generate_handler![rebind_hotkey])
        .setup(|app| {
            build_tray(app)?;

            // Determine hotkey to register: use saved setting if present, else default
            let hotkey = read_saved_hotkey(app)
                .unwrap_or_else(|| "ctrl+shift+space".to_owned());

            println!("Registering hotkey: {}", hotkey);

            // Register global hotkey plugin (desktop only — no Android/iOS support)
            #[cfg(desktop)]
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_shortcuts([hotkey.as_str()])?
                    .with_handler(|app, shortcut, event| {
                        use tauri_plugin_global_shortcut::ShortcutState;
                        if event.state == ShortcutState::Pressed {
                            println!("Hotkey triggered: {}", shortcut);
                            let _ = app.emit("hotkey-triggered", ());
                        }
                    })
                    .build(),
            )?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "settings" {
                    // Hide to tray instead of closing
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
