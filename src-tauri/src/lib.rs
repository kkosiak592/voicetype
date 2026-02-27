mod tray;
mod transcribe;

use std::sync::Arc;
use tauri::{Emitter, Manager};
use tray::build_tray;
use whisper_rs::WhisperContext;

/// Tauri managed state holding the loaded WhisperContext (if the model file was found).
pub struct WhisperState(pub Option<Arc<WhisperContext>>);

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

/// Tauri command for testing whisper inference on a WAV file.
///
/// Reads the WAV at `path`, normalises samples to f32, and runs transcription
/// using the GPU-accelerated WhisperContext stored in managed state.
/// Returns the transcription text and duration in milliseconds.
///
/// If the model was not loaded at startup (missing model file), returns an error.
#[tauri::command]
async fn transcribe_test_file(
    app: tauri::AppHandle,
    path: String,
) -> Result<String, String> {
    use std::time::Instant;

    let start = Instant::now();

    // Get the WhisperContext from managed state
    let state = app.state::<WhisperState>();
    let ctx = match &state.0 {
        Some(ctx) => Arc::clone(ctx),
        None => {
            return Err(
                "Whisper model not loaded. Check startup logs for the download instructions."
                    .to_string(),
            );
        }
    };

    // Read the WAV file
    let mut reader = hound::WavReader::open(&path)
        .map_err(|e| format!("Failed to open WAV file '{}': {}", path, e))?;

    let spec = reader.spec();
    log::info!(
        "WAV file: {} — {}Hz, {} channel(s), {}bit {:?}",
        path,
        spec.sample_rate,
        spec.channels,
        spec.bits_per_sample,
        spec.sample_format
    );

    // Decode samples to f32, downmixing to mono
    let channels = spec.channels as usize;
    let audio_f32: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => {
            let samples: Vec<f32> = reader
                .samples::<f32>()
                .collect::<Result<_, _>>()
                .map_err(|e| format!("Failed to read WAV samples: {}", e))?;
            // Downmix to mono by averaging channels
            samples
                .chunks(channels)
                .map(|ch| ch.iter().sum::<f32>() / channels as f32)
                .collect()
        }
        hound::SampleFormat::Int => {
            let max_val = (1i64 << (spec.bits_per_sample - 1)) as f32;
            let samples: Vec<i32> = reader
                .samples::<i32>()
                .collect::<Result<_, _>>()
                .map_err(|e| format!("Failed to read WAV samples: {}", e))?;
            // Downmix to mono and normalise to [-1.0, 1.0]
            samples
                .chunks(channels)
                .map(|ch| {
                    let mono_int: f32 = ch.iter().map(|&s| s as f32).sum::<f32>() / channels as f32;
                    mono_int / max_val
                })
                .collect()
        }
    };

    log::info!(
        "WAV decoded: {} samples at {}Hz ({:.1}s audio)",
        audio_f32.len(),
        spec.sample_rate,
        audio_f32.len() as f32 / spec.sample_rate as f32
    );

    // Run inference on a blocking thread to avoid stalling the Tauri async runtime
    let result = tokio::task::spawn_blocking(move || {
        transcribe::transcribe_audio(&ctx, &audio_f32)
    })
    .await
    .map_err(|e| format!("Blocking task panicked: {}", e))??;

    let total_ms = start.elapsed().as_millis();
    log::info!("transcribe_test_file completed in {}ms: '{}'", total_ms, result);

    Ok(format!("[{}ms] {}", total_ms, result))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

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
        .invoke_handler(tauri::generate_handler![rebind_hotkey, transcribe_test_file])
        .setup(|app| {
            build_tray(app)?;

            // Determine hotkey to register: use saved setting if present, else default
            let hotkey = read_saved_hotkey(app)
                .unwrap_or_else(|| "ctrl+shift+space".to_owned());

            log::info!("Registering hotkey: {}", hotkey);

            // Register global hotkey plugin (desktop only — no Android/iOS support)
            #[cfg(desktop)]
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_shortcuts([hotkey.as_str()])?
                    .with_handler(|app, shortcut, event| {
                        use tauri_plugin_global_shortcut::ShortcutState;
                        if event.state == ShortcutState::Pressed {
                            log::info!("Hotkey triggered: {}", shortcut);
                            let _ = app.emit("hotkey-triggered", ());
                        }
                    })
                    .build(),
            )?;

            // Attempt to load the whisper model — app continues even if model is missing
            let whisper_ctx = match transcribe::resolve_model_path() {
                Ok(model_path) => {
                    let model_str = model_path.to_string_lossy().to_string();
                    match transcribe::load_whisper_context(&model_str) {
                        Ok(ctx) => {
                            log::info!("CUDA whisper context initialized successfully");
                            Some(Arc::new(ctx))
                        }
                        Err(e) => {
                            log::error!("Whisper model not loaded: {}", e);
                            None
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Whisper model not loaded: {}", e);
                    None
                }
            };

            app.manage(WhisperState(whisper_ctx));

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
