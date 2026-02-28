mod audio;
mod inject;
mod tray;

// transcribe.rs requires whisper-rs which needs LIBCLANG_PATH + optional CUDA.
// Gate it behind the "whisper" Cargo feature so the project builds without
// LLVM installed (audio-only verification, Phase 2 Plan 01).
#[cfg(feature = "whisper")]
mod transcribe;

#[cfg(feature = "whisper")]
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tray::build_tray;

// WhisperState and related types are only available with the whisper feature.
#[cfg(feature = "whisper")]
use whisper_rs::WhisperContext;

#[cfg(feature = "whisper")]
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

/// Start recording: clears the audio buffer and sets the recording flag.
///
/// Audio captured after this call is accumulated in memory at 16kHz mono.
#[tauri::command]
fn start_recording(state: tauri::State<'_, audio::AudioCapture>) -> Result<(), String> {
    state.clear_buffer();
    state.recording.store(true, std::sync::atomic::Ordering::Relaxed);
    log::info!("Recording started");
    Ok(())
}

/// Stop recording: clears the recording flag, flushes the resampler,
/// and returns the number of 16kHz samples captured.
#[tauri::command]
fn stop_recording(state: tauri::State<'_, audio::AudioCapture>) -> Result<usize, String> {
    let n = state.flush_and_stop();
    let seconds = n as f32 / 16000.0;
    log::info!("Recording stopped: {} samples ({:.1}s)", n, seconds);
    Ok(n)
}

/// Save the captured audio buffer to a WAV file in test-fixtures/.
///
/// Returns the file path on success.
#[tauri::command]
fn save_test_wav(app: tauri::AppHandle, state: tauri::State<'_, audio::AudioCapture>) -> Result<String, String> {
    let samples = state.get_buffer();
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let wav_dir = data_dir.join("test-fixtures");
    std::fs::create_dir_all(&wav_dir).map_err(|e| e.to_string())?;
    let path = wav_dir.join("capture-test.wav");
    let path_str = path.to_string_lossy().to_string();

    audio::write_wav(&path_str, &samples).map_err(|e| e.to_string())?;

    log::info!(
        "WAV saved: {} ({} samples, {:.1}s)",
        path_str,
        samples.len(),
        samples.len() as f32 / 16000.0
    );

    Ok(path_str)
}

/// Reads a WAV file and decodes it to mono f32 samples.
///
/// Supports float and integer WAV formats. Downmixes multi-channel audio to mono.
/// Shared by transcribe_test_file and force_cpu_transcribe.
#[cfg(feature = "whisper")]
fn read_wav_to_f32(path: &str) -> Result<(Vec<f32>, u32), String> {
    let mut reader = hound::WavReader::open(path)
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

    let channels = spec.channels as usize;
    let audio_f32: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => {
            let samples: Vec<f32> = reader
                .samples::<f32>()
                .collect::<Result<_, _>>()
                .map_err(|e| format!("Failed to read WAV samples: {}", e))?;
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

    Ok((audio_f32, spec.sample_rate))
}

/// Test whisper inference on a WAV file.
///
/// Reads the WAV at `path`, normalises samples to f32, and runs transcription
/// using the WhisperContext stored in managed state (GPU or CPU depending on detected mode).
/// Returns the transcription text prefixed with duration in milliseconds.
///
/// Only available when compiled with the "whisper" feature flag.
#[cfg(feature = "whisper")]
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

    let (audio_f32, _sample_rate) = read_wav_to_f32(&path)?;

    // Run inference on a blocking thread to avoid stalling the Tauri async runtime
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(transcribe::transcribe_audio(&ctx, &audio_f32));
    });
    let result = rx.recv()
        .map_err(|e| format!("Inference thread failed: {}", e))?
        .map_err(|e| e.to_string())?;

    let total_ms = start.elapsed().as_millis();
    log::info!("transcribe_test_file completed in {}ms: '{}'", total_ms, result);

    Ok(format!("[{}ms] {}", total_ms, result))
}

/// Force CPU inference on a WAV file for testing the CPU fallback path.
///
/// Loads the small.en CPU model with use_gpu(false) regardless of whether a GPU
/// is present. This allows verifying CORE-04 (CPU fallback) on the dev machine
/// (which has a Quadro P2000). CPU inference on the small model typically takes
/// 2-10s for a 5s clip — this is acceptable per Phase 2 success criteria.
///
/// Only available when compiled with the "whisper" feature flag.
/// Phase 2 verification command — will be removed or hidden in later phases.
#[cfg(feature = "whisper")]
#[tauri::command]
async fn force_cpu_transcribe(path: String) -> Result<String, String> {
    use std::time::Instant;

    let start = Instant::now();
    log::info!("force_cpu_transcribe: loading CPU model for path '{}'", path);

    // Resolve the CPU model path (ggml-small.en-q5_1.bin)
    let cpu_mode = transcribe::ModelMode::Cpu;
    let model_path = transcribe::resolve_model_path(&cpu_mode)?;
    let model_str = model_path.to_string_lossy().to_string();

    // Load CPU model with use_gpu(false) on a blocking thread
    let (audio_f32, _sample_rate) = read_wav_to_f32(&path)?;

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let ctx_result = transcribe::load_whisper_context(&model_str, &cpu_mode);
        let result = ctx_result.and_then(|ctx| transcribe::transcribe_audio(&ctx, &audio_f32));
        let _ = tx.send(result);
    });

    let result = rx.recv()
        .map_err(|e| format!("CPU inference thread failed: {}", e))?
        .map_err(|e| e.to_string())?;

    let total_ms = start.elapsed().as_millis();
    log::info!(
        "force_cpu_transcribe completed in {}ms (GPU=false): '{}'",
        total_ms,
        result
    );

    Ok(format!("[{}ms CPU] {}", total_ms, result))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .init();

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
        .invoke_handler(tauri::generate_handler![
            rebind_hotkey,
            start_recording,
            stop_recording,
            save_test_wav,
            #[cfg(feature = "whisper")]
            transcribe_test_file,
            #[cfg(feature = "whisper")]
            force_cpu_transcribe,
        ])
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

            // Start persistent audio capture stream.
            // App continues even if microphone is unavailable.
            match audio::start_persistent_stream() {
                Ok(capture) => {
                    log::info!("Audio capture initialized successfully");
                    app.manage(capture);
                }
                Err(e) => {
                    log::error!("Audio capture failed to initialize: {} — recording commands will not function", e);
                }
            }

            // Load whisper model (only when compiled with "whisper" feature).
            #[cfg(feature = "whisper")]
            {
                // Detect GPU presence at runtime — selects large-v3-turbo (GPU) or small.en (CPU)
                let mode = transcribe::detect_gpu();
                log::info!("Inference mode selected: {:?}", mode);

                let whisper_ctx = match transcribe::resolve_model_path(&mode) {
                    Ok(model_path) => {
                        let model_str = model_path.to_string_lossy().to_string();
                        match transcribe::load_whisper_context(&model_str, &mode) {
                            Ok(ctx) => {
                                log::info!("Whisper context initialized successfully ({:?} mode)", mode);
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
            }

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
