mod audio;
mod corrections;
mod download;
mod inject;
mod pill;
mod pipeline;
mod profiles;
mod tray;
mod vad;
#[cfg(test)]
mod corrections_tests;

// transcribe.rs requires whisper-rs which needs LIBCLANG_PATH + optional CUDA.
// Gate it behind the "whisper" Cargo feature so the project builds without
// LLVM installed (audio-only verification, Phase 2 Plan 01).
#[cfg(feature = "whisper")]
mod transcribe;

// transcribe_parakeet.rs uses parakeet-rs and ONNX runtime.
// Gated so the project builds without the parakeet feature.
#[cfg(feature = "parakeet")]
mod transcribe_parakeet;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tauri::Manager;
use tray::build_tray;

/// Transcription engine selector: Whisper (default) or Parakeet.
///
/// Not feature-gated so settings persistence works regardless of compiled features.
/// Loaded from settings.json at startup via `read_saved_engine()`.
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TranscriptionEngine {
    Whisper,
    Parakeet,
}

/// Mutex-backed managed state for the active transcription engine.
///
/// CRITICAL: Registered on the Builder (not just in setup) so frontend IPC can
/// read the engine before setup() fires (same issue as CachedGpuMode).
pub struct ActiveEngine(pub std::sync::Mutex<TranscriptionEngine>);

/// Mutex-wrapped ParakeetTDT for runtime access.
///
/// Outer Mutex<Option<...>> mirrors WhisperStateMutex — allows replacing the model
/// at runtime (load-on-demand or engine switch).
///
/// Inner Arc<Mutex<ParakeetTDT>>: Arc makes it clonable for spawn_blocking;
/// inner Mutex provides the `&mut self` access that parakeet-rs 0.1.x requires.
/// ParakeetTDT is not Sync (transcribe_samples takes &mut self), so it cannot be
/// wrapped in Arc alone — the inner Mutex serialises &mut access.
#[cfg(feature = "parakeet")]
pub struct ParakeetStateMutex(
    pub std::sync::Mutex<Option<std::sync::Arc<std::sync::Mutex<parakeet_rs::ParakeetTDT>>>>,
);

/// Control flag for the RMS level streaming loop. Stored as managed state
/// so both the setup() and rebind_hotkey() hotkey handlers can access it.
pub struct LevelStreamActive(pub Arc<AtomicBool>);

/// Recording mode selector: hold-to-talk (default) or toggle mode.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mode {
    HoldToTalk = 0,
    Toggle = 1,
}

/// AtomicU8-backed managed state for the current recording mode.
///
/// Loaded from settings.json at startup via `read_saved_mode()`.
/// Updated immediately when the user changes mode in settings UI.
pub struct RecordingMode(pub std::sync::atomic::AtomicU8);

impl RecordingMode {
    pub fn new(mode: Mode) -> Self {
        RecordingMode(std::sync::atomic::AtomicU8::new(mode as u8))
    }
    pub fn get(&self) -> Mode {
        match self.0.load(std::sync::atomic::Ordering::Relaxed) {
            1 => Mode::Toggle,
            _ => Mode::HoldToTalk,
        }
    }
    pub fn set(&self, mode: Mode) {
        self.0.store(mode as u8, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Managed state holding the cancel handle for the active VAD worker (toggle mode).
///
/// Stored as `Mutex<Option<VadWorkerHandle>>` so the handle can be taken (replaced
/// with None) on second tap, pipeline entry, or any early-stop path.
pub struct VadWorkerState(pub std::sync::Mutex<Option<vad::VadWorkerHandle>>);

// WhisperState and related types are only available with the whisper feature.
#[cfg(feature = "whisper")]
use whisper_rs::WhisperContext;

/// Mutex-wrapped optional WhisperContext for runtime model switching.
///
/// The Mutex allows replacing the inner WhisperContext at runtime when the user
/// switches models. The Option handles the case where no model is loaded.
#[cfg(feature = "whisper")]
pub struct WhisperStateMutex(pub std::sync::Mutex<Option<Arc<WhisperContext>>>);

/// Cached GPU detection result. Populated once at startup so that
/// `list_models()` and `check_first_run()` don't re-probe NVML on every call.
#[cfg(feature = "whisper")]
pub struct CachedGpuMode(pub transcribe::ModelMode);

/// Read settings.json from the app data directory. Returns empty object on missing/corrupt file.
fn read_settings(app_handle: &tauri::AppHandle) -> Result<serde_json::Value, String> {
    let data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let settings_path = data_dir.join("settings.json");
    Ok(std::fs::read_to_string(&settings_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({})))
}

/// Write settings.json to the app data directory (pretty-printed).
fn write_settings(app_handle: &tauri::AppHandle, json: &serde_json::Value) -> Result<(), String> {
    let data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let settings_path = data_dir.join("settings.json");
    std::fs::write(&settings_path, serde_json::to_string_pretty(json).unwrap())
        .map_err(|e| e.to_string())
}

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

/// Read the saved recording mode from settings.json.
/// Returns Mode::HoldToTalk on first launch, file missing, or parse error (hold-to-talk is default).
fn read_saved_mode(app: &tauri::App) -> Mode {
    let data_dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(_) => return Mode::HoldToTalk,
    };
    let settings_path = data_dir.join("settings.json");

    let contents = match std::fs::read_to_string(&settings_path) {
        Ok(c) => c,
        Err(_) => return Mode::HoldToTalk,
    };
    let json: serde_json::Value = match serde_json::from_str(&contents) {
        Ok(j) => j,
        Err(_) => return Mode::HoldToTalk,
    };

    match json.get("recording_mode").and_then(|v| v.as_str()) {
        Some("toggle") => Mode::Toggle,
        _ => Mode::HoldToTalk,
    }
}

/// Read the saved transcription engine from settings.json.
/// Returns TranscriptionEngine::Whisper on first launch, file missing, or parse error.
fn read_saved_engine(app: &tauri::App) -> TranscriptionEngine {
    let data_dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(_) => return TranscriptionEngine::Whisper,
    };
    let settings_path = data_dir.join("settings.json");
    let contents = match std::fs::read_to_string(&settings_path) {
        Ok(c) => c,
        Err(_) => return TranscriptionEngine::Whisper,
    };
    let json: serde_json::Value = match serde_json::from_str(&contents) {
        Ok(j) => j,
        Err(_) => return TranscriptionEngine::Whisper,
    };
    match json.get("active_engine").and_then(|v| v.as_str()) {
        Some("parakeet") => TranscriptionEngine::Parakeet,
        _ => TranscriptionEngine::Whisper,
    }
}

/// Read the saved Parakeet model variant from settings.json.
/// Returns "parakeet-tdt-v2" (int8) by default.
fn read_saved_parakeet_model(app_handle: &tauri::AppHandle) -> String {
    let json = read_settings(app_handle).unwrap_or_else(|_| serde_json::json!({}));
    json.get("parakeet_model")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("parakeet-tdt-v2")
        .to_string()
}

/// Read the saved Parakeet model variant from settings.json at startup.
/// Returns "parakeet-tdt-v2" (int8) by default.
fn read_saved_parakeet_model_startup(app: &tauri::App) -> String {
    let data_dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(_) => return "parakeet-tdt-v2".to_string(),
    };
    let settings_path = data_dir.join("settings.json");
    let contents = match std::fs::read_to_string(&settings_path) {
        Ok(c) => c,
        Err(_) => return "parakeet-tdt-v2".to_string(),
    };
    let json: serde_json::Value = match serde_json::from_str(&contents) {
        Ok(j) => j,
        Err(_) => return "parakeet-tdt-v2".to_string(),
    };
    json.get("parakeet_model")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("parakeet-tdt-v2")
        .to_string()
}

/// Resolve the model directory for a Parakeet variant.
fn resolve_parakeet_dir(model_id: &str) -> std::path::PathBuf {
    match model_id {
        "parakeet-tdt-v2-fp32" => download::parakeet_fp32_model_dir(),
        _ => download::parakeet_model_dir(), // default to int8
    }
}

#[tauri::command]
fn set_recording_mode(app: tauri::AppHandle, mode: String) -> Result<(), String> {
    // Update managed state immediately
    let recording_mode = app.state::<RecordingMode>();
    match mode.as_str() {
        "toggle" => recording_mode.set(Mode::Toggle),
        _ => recording_mode.set(Mode::HoldToTalk),
    }
    // Persist to settings.json
    let mut json = read_settings(&app)?;
    json["recording_mode"] = serde_json::Value::String(mode);
    write_settings(&app, &json)?;
    log::info!("Recording mode set to: {}", json["recording_mode"]);
    Ok(())
}

#[tauri::command]
fn get_recording_mode(app: tauri::AppHandle) -> String {
    let mode = app.state::<RecordingMode>();
    match mode.get() {
        Mode::Toggle => "toggle".to_string(),
        Mode::HoldToTalk => "hold".to_string(),
    }
}

/// Returns the current active transcription engine as a string ("whisper" or "parakeet").
#[tauri::command]
fn get_engine(app: tauri::AppHandle) -> String {
    let state = app.state::<ActiveEngine>();
    let guard = state.0.lock().unwrap_or_else(|e| e.into_inner());
    match *guard {
        TranscriptionEngine::Whisper => "whisper".to_string(),
        TranscriptionEngine::Parakeet => "parakeet".to_string(),
    }
}

/// Switch the active transcription engine.
///
/// Accepts an optional `parakeet_model` parameter to specify which Parakeet variant
/// (int8 or fp32) to load. When None, falls back to the saved variant in settings.json.
/// Always reloads the Parakeet model on any parakeet switch — required for variant switching
/// (int8 -> fp32 or fp32 -> int8) to take effect without an app restart.
/// Reverts to Whisper and returns Err if loading fails or model is not downloaded.
/// Persists the selection to settings.json.
#[tauri::command]
fn set_engine(app: tauri::AppHandle, engine: String, parakeet_model: Option<String>) -> Result<(), String> {
    let new_engine = match engine.as_str() {
        "parakeet" => TranscriptionEngine::Parakeet,
        _ => TranscriptionEngine::Whisper,
    };
    // Update ActiveEngine managed state
    {
        let state = app.state::<ActiveEngine>();
        let mut guard = state.0.lock().map_err(|e| format!("state lock failed: {}", e))?;
        *guard = new_engine;
    }
    // If switching to Parakeet, always reload model (required for variant switching)
    #[cfg(feature = "parakeet")]
    if new_engine == TranscriptionEngine::Parakeet {
        let parakeet_state = app.state::<ParakeetStateMutex>();
        let parakeet_model_id = parakeet_model
            .clone()
            .unwrap_or_else(|| read_saved_parakeet_model(&app));
        let model_dir = resolve_parakeet_dir(&parakeet_model_id);
        if model_dir.exists() {
            let dir_str = model_dir.to_string_lossy().to_string();
            match transcribe_parakeet::load_parakeet(&dir_str, true) {
                Ok(p) => {
                    let mut guard = parakeet_state.0.lock().unwrap_or_else(|e| e.into_inner());
                    *guard = Some(std::sync::Arc::new(std::sync::Mutex::new(p)));
                    log::info!("Parakeet model loaded on engine switch (variant: {})", parakeet_model_id);
                }
                Err(e) => {
                    log::error!("Failed to load Parakeet on engine switch: {}", e);
                    // Revert to Whisper since Parakeet failed
                    let state = app.state::<ActiveEngine>();
                    let mut guard = state.0.lock().unwrap_or_else(|e| e.into_inner());
                    *guard = TranscriptionEngine::Whisper;
                    return Err(format!(
                        "Parakeet model failed to load: {}. Reverting to Whisper.",
                        e
                    ));
                }
            }
        } else {
            // Revert — model not downloaded
            let state = app.state::<ActiveEngine>();
            let mut guard = state.0.lock().unwrap_or_else(|e| e.into_inner());
            *guard = TranscriptionEngine::Whisper;
            return Err(
                "Parakeet model not downloaded. Download it first from Settings.".to_string(),
            );
        }
    }
    // Persist to settings.json
    let mut json = read_settings(&app)?;
    if let Some(ref variant) = parakeet_model {
        json["parakeet_model"] = serde_json::Value::String(variant.clone());
    }
    json["active_engine"] = serde_json::Value::String(engine);
    write_settings(&app, &json)?;
    log::info!("Transcription engine set to: {:?}", new_engine);
    Ok(())
}

/// Shared hotkey handler body — called from both setup() and rebind_hotkey() handlers.
fn handle_shortcut(app: &tauri::AppHandle, event: &tauri_plugin_global_shortcut::ShortcutEvent) {
    use tauri_plugin_global_shortcut::ShortcutState;
    use std::sync::atomic::Ordering;
    use tauri::Emitter;

    let pipeline = app.state::<pipeline::PipelineState>();

    match event.state {
        ShortcutState::Pressed => {
            let mode = app.state::<RecordingMode>().get();

            match mode {
                Mode::HoldToTalk => {
                    if pipeline.transition(pipeline::IDLE, pipeline::RECORDING) {
                        let audio_mutex = app.state::<audio::AudioCaptureMutex>();
                        let guard = audio_mutex.0.lock().unwrap_or_else(|e| e.into_inner());
                        let audio = match guard.as_ref() {
                            Some(a) => a,
                            None => {
                                log::error!("No microphone — cannot record");
                                pipeline.reset_to_idle();
                                return;
                            }
                        };
                        audio.clear_buffer();
                        audio.recording.store(true, Ordering::Relaxed);
                        let buffer_clone = audio.buffer.clone();
                        drop(guard);
                        tray::set_tray_state(app, tray::TrayState::Recording);
                        pill::show_pill(app);
                        app.emit_to("pill", "pill-state", "recording").ok();

                        let stream_active = app.state::<LevelStreamActive>();
                        stream_active.0.store(true, Ordering::Relaxed);
                        pill::start_level_stream(
                            app.clone(),
                            buffer_clone,
                            stream_active.0.clone(),
                        );

                        log::info!("Pipeline: IDLE -> RECORDING (hold-to-talk)");
                    }
                }
                Mode::Toggle => {
                    if pipeline.transition(pipeline::IDLE, pipeline::RECORDING) {
                        let audio_mutex = app.state::<audio::AudioCaptureMutex>();
                        let guard = audio_mutex.0.lock().unwrap_or_else(|e| e.into_inner());
                        let audio = match guard.as_ref() {
                            Some(a) => a,
                            None => {
                                log::error!("No microphone — cannot record (toggle)");
                                pipeline.reset_to_idle();
                                return;
                            }
                        };
                        audio.clear_buffer();
                        audio.recording.store(true, Ordering::Relaxed);
                        let buffer_clone = audio.buffer.clone();
                        drop(guard);
                        tray::set_tray_state(app, tray::TrayState::Recording);
                        pill::show_pill(app);
                        app.emit_to("pill", "pill-state", "recording").ok();
                        let stream_active = app.state::<LevelStreamActive>();
                        stream_active.0.store(true, Ordering::Relaxed);
                        pill::start_level_stream(
                            app.clone(),
                            buffer_clone.clone(),
                            stream_active.0.clone(),
                        );

                        let vad_handle = vad::spawn_vad_worker(
                            app.clone(),
                            buffer_clone,
                        );
                        let vad_state = app.state::<VadWorkerState>();
                        if let Ok(mut guard) = vad_state.0.lock() {
                            *guard = Some(vad_handle);
                        }

                        log::info!("Pipeline: IDLE -> RECORDING (toggle mode, VAD worker started)");
                    } else if pipeline.transition(pipeline::RECORDING, pipeline::PROCESSING) {
                        let vad_state = app.state::<VadWorkerState>();
                        if let Ok(mut guard) = vad_state.0.lock() {
                            if let Some(mut handle) = guard.take() {
                                handle.cancel();
                            }
                        }

                        let stream_active = app.state::<LevelStreamActive>();
                        stream_active.0.store(false, Ordering::Relaxed);
                        app.emit_to("pill", "pill-state", "processing").ok();
                        tray::set_tray_state(app, tray::TrayState::Processing);
                        log::info!("Pipeline: RECORDING -> PROCESSING (toggle mode, second tap)");

                        let app_handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            pipeline::run_pipeline(app_handle).await;
                        });
                    }
                }
            }
        }
        ShortcutState::Released => {
            let mode = app.state::<RecordingMode>().get();
            match mode {
                Mode::HoldToTalk => {
                    if pipeline.transition(pipeline::RECORDING, pipeline::PROCESSING) {
                        let stream_active = app.state::<LevelStreamActive>();
                        stream_active.0.store(false, Ordering::Relaxed);
                        app.emit_to("pill", "pill-state", "processing").ok();
                        tray::set_tray_state(app, tray::TrayState::Processing);
                        log::info!("Pipeline: RECORDING -> PROCESSING");
                        let app_handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            pipeline::run_pipeline(app_handle).await;
                        });
                    }
                }
                Mode::Toggle => {
                    // Toggle mode: release is ignored — VAD or second tap stops
                }
            }
        }
    }
}

#[tauri::command]
fn rebind_hotkey(app: tauri::AppHandle, old: String, new_key: String) -> Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    let gs = app.global_shortcut();

    if !old.is_empty() {
        gs.unregister(old.as_str()).map_err(|e| e.to_string())?;
    }

    gs.on_shortcut(new_key.as_str(), |app, _shortcut, event| {
        handle_shortcut(app, &event);
    })
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Start recording: clears the audio buffer and sets the recording flag.
///
/// Audio captured after this call is accumulated in memory at 16kHz mono.
#[tauri::command]
fn start_recording(state: tauri::State<'_, audio::AudioCaptureMutex>) -> Result<(), String> {
    let guard = state.0.lock().map_err(|e| format!("state lock failed: {}", e))?;
    let audio = guard.as_ref().ok_or("No microphone available")?;
    audio.clear_buffer();
    audio.recording.store(true, std::sync::atomic::Ordering::Relaxed);
    log::info!("Recording started");
    Ok(())
}

/// Stop recording: clears the recording flag, flushes the resampler,
/// and returns the number of 16kHz samples captured.
#[tauri::command]
fn stop_recording(state: tauri::State<'_, audio::AudioCaptureMutex>) -> Result<usize, String> {
    let guard = state.0.lock().map_err(|e| format!("state lock failed: {}", e))?;
    let audio = guard.as_ref().ok_or("No microphone available")?;
    let n = audio.flush_and_stop();
    let seconds = n as f32 / 16000.0;
    log::info!("Recording stopped: {} samples ({:.1}s)", n, seconds);
    Ok(n)
}

/// Save the captured audio buffer to a WAV file in test-fixtures/.
///
/// Returns the file path on success.
#[tauri::command]
fn save_test_wav(app: tauri::AppHandle, state: tauri::State<'_, audio::AudioCaptureMutex>) -> Result<String, String> {
    let guard = state.0.lock().map_err(|e| format!("state lock failed: {}", e))?;
    let audio = guard.as_ref().ok_or("No microphone available")?;
    let samples = audio.get_buffer();
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

/// Read the saved active profile ID from settings.json.
/// Returns "general" on first launch, file missing, or parse error.
fn read_saved_profile_id(app: &tauri::App) -> String {
    let data_dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(_) => return "general".to_string(),
    };
    let settings_path = data_dir.join("settings.json");
    let contents = match std::fs::read_to_string(&settings_path) {
        Ok(c) => c,
        Err(_) => return "general".to_string(),
    };
    let json: serde_json::Value = match serde_json::from_str(&contents) {
        Ok(j) => j,
        Err(_) => return "general".to_string(),
    };
    json.get("active_profile_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned())
        .unwrap_or_else(|| "general".to_string())
}

/// Read the saved user corrections for a specific profile from settings.json.
/// Returns an empty HashMap on first launch, file missing, or parse error.
fn read_saved_corrections(app: &tauri::App, profile_id: &str) -> std::collections::HashMap<String, String> {
    let data_dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(_) => return std::collections::HashMap::new(),
    };
    let settings_path = data_dir.join("settings.json");
    let contents = match std::fs::read_to_string(&settings_path) {
        Ok(c) => c,
        Err(_) => return std::collections::HashMap::new(),
    };
    let json: serde_json::Value = match serde_json::from_str(&contents) {
        Ok(j) => j,
        Err(_) => return std::collections::HashMap::new(),
    };
    let key = format!("corrections.{}", profile_id);
    json.get(&key)
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

/// Read the saved ALL CAPS flag for a specific profile from settings.json.
/// Returns false on first launch, file missing, or parse error.
fn read_saved_all_caps(app: &tauri::App, profile_id: &str) -> bool {
    let data_dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(_) => return false,
    };
    let settings_path = data_dir.join("settings.json");
    let contents = match std::fs::read_to_string(&settings_path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let json: serde_json::Value = match serde_json::from_str(&contents) {
        Ok(j) => j,
        Err(_) => return false,
    };
    let key = format!("profiles.{}.all_caps", profile_id);
    json.get(&key)
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// Profile info returned by get_profiles command — lightweight, no corrections dictionary.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileInfo {
    id: String,
    name: String,
    is_active: bool,
}

/// Returns the list of available profiles with their IDs, names, and active flag.
#[tauri::command]
fn get_profiles(app: tauri::AppHandle) -> Result<Vec<ProfileInfo>, String> {
    let active_id = {
        let state = app.state::<profiles::ActiveProfile>();
        let guard = state.0.lock().map_err(|e| format!("state lock failed: {}", e))?;
        guard.id.clone()
    };
    Ok(profiles::get_all_profiles()
        .into_iter()
        .map(|p| ProfileInfo {
            is_active: p.id == active_id,
            id: p.id,
            name: p.name,
        })
        .collect())
}

/// Switch the active profile by ID. Rebuilds the CorrectionsEngine from the new profile's
/// corrections merged with any user-saved corrections for that profile.
/// Persists the active profile ID to settings.json.
#[tauri::command]
fn set_active_profile(app: tauri::AppHandle, profile_id: String) -> Result<(), String> {
    // Build the new profile
    let mut new_profile = match profile_id.as_str() {
        "structural-engineering" => profiles::structural_engineering_profile(),
        "general" => profiles::general_profile(),
        _ => return Err(format!("Unknown profile id: {}", profile_id)),
    };

    // Merge saved user corrections (user overrides defaults)
    let mut json = read_settings(&app)?;

    let user_corrections_key = format!("corrections.{}", profile_id);
    if let Some(user_map) = json.get(&user_corrections_key).and_then(|v| v.as_object()) {
        for (k, v) in user_map {
            if let Some(s) = v.as_str() {
                new_profile.corrections.insert(k.clone(), s.to_string());
            }
        }
    }

    // Load saved ALL CAPS flag for this profile
    let all_caps_key = format!("profiles.{}.all_caps", profile_id);
    if let Some(flag) = json.get(&all_caps_key).and_then(|v| v.as_bool()) {
        new_profile.all_caps = flag;
    }

    // Rebuild corrections engine from merged corrections
    let engine = corrections::CorrectionsEngine::from_map(&new_profile.corrections)?;

    // Update managed states
    {
        let profile_state = app.state::<profiles::ActiveProfile>();
        let mut guard = profile_state.0.lock().map_err(|e| format!("state lock failed: {}", e))?;
        *guard = new_profile;
    }
    {
        let corrections_state = app.state::<corrections::CorrectionsState>();
        let mut guard = corrections_state.0.lock().map_err(|e| format!("state lock failed: {}", e))?;
        *guard = engine;
    }

    // Persist active profile ID
    json["active_profile_id"] = serde_json::Value::String(profile_id.clone());
    write_settings(&app, &json)?;

    log::info!("Active profile set to: {}", profile_id);
    Ok(())
}

/// Returns the current active profile's corrections dictionary (for the UI editor).
#[tauri::command]
fn get_corrections(app: tauri::AppHandle) -> Result<std::collections::HashMap<String, String>, String> {
    let state = app.state::<profiles::ActiveProfile>();
    let guard = state.0.lock().map_err(|e| format!("state lock failed: {}", e))?;
    Ok(guard.corrections.clone())
}

/// Save user corrections for the active profile. Merges with defaults and rebuilds engine.
/// Persists to settings.json under `corrections.{profile_id}`.
#[tauri::command]
fn save_corrections(app: tauri::AppHandle, corrections: std::collections::HashMap<String, String>) -> Result<(), String> {
    let profile_id = {
        let state = app.state::<profiles::ActiveProfile>();
        let mut guard = state.0.lock().map_err(|e| format!("state lock failed: {}", e))?;
        // Rebuild corrections from defaults + user map so deleted keys are removed.
        let defaults = match guard.id.as_str() {
            "structural-engineering" => profiles::structural_engineering_profile().corrections,
            _ => profiles::general_profile().corrections,
        };
        guard.corrections.clear();
        guard.corrections.extend(defaults);
        guard.corrections.extend(corrections.clone());
        guard.id.clone()
    };

    // Rebuild engine from updated corrections
    let engine = {
        let state = app.state::<profiles::ActiveProfile>();
        let guard = state.0.lock().map_err(|e| format!("state lock failed: {}", e))?;
        corrections::CorrectionsEngine::from_map(&guard.corrections)?
    };
    {
        let corrections_state = app.state::<corrections::CorrectionsState>();
        let mut guard = corrections_state.0.lock().map_err(|e| format!("state lock failed: {}", e))?;
        *guard = engine;
    }

    // Persist user corrections to settings.json
    let mut json = read_settings(&app)?;
    let key = format!("corrections.{}", profile_id);
    json[&key] = serde_json::to_value(&corrections).unwrap();
    write_settings(&app, &json)?;

    log::info!("Corrections saved for profile '{}'", profile_id);
    Ok(())
}

/// Toggle ALL CAPS for the active profile. Persists to settings.json.
#[tauri::command]
fn set_all_caps(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let profile_id = {
        let state = app.state::<profiles::ActiveProfile>();
        let mut guard = state.0.lock().map_err(|e| format!("state lock failed: {}", e))?;
        guard.all_caps = enabled;
        guard.id.clone()
    };

    let mut json = read_settings(&app)?;
    let key = format!("profiles.{}.all_caps", profile_id);
    json[&key] = serde_json::Value::Bool(enabled);
    write_settings(&app, &json)?;

    log::info!("ALL CAPS set to {} for profile '{}'", enabled, profile_id);
    Ok(())
}

/// Read the saved microphone device name from settings.json.
/// Returns None on first launch, file missing, or no saved mic (system default).
fn read_saved_mic(app: &tauri::App) -> Option<String> {
    let data_dir = app.path().app_data_dir().ok()?;
    let settings_path = data_dir.join("settings.json");
    let contents = std::fs::read_to_string(&settings_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&contents).ok()?;
    json.get("microphone_device")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty() && *s != "System Default")
        .map(|s| s.to_owned())
}

/// List available audio input device names.
///
/// The first entry is always "System Default" so the UI has a way to revert.
#[tauri::command]
fn list_input_devices() -> Result<Vec<String>, String> {
    use cpal::traits::{DeviceTrait, HostTrait};
    let host = cpal::default_host();
    let devices = host.input_devices().map_err(|e| e.to_string())?;
    let mut names = vec!["System Default".to_string()];
    for device in devices {
        if let Ok(desc) = device.description() {
            names.push(desc.name().to_string());
        }
    }
    Ok(names)
}

/// Switch the active microphone device. Restarts the audio stream with the new device.
///
/// Passing "System Default" or an empty string uses `host.default_input_device()`.
/// Persists the selection to settings.json so it is restored at next startup.
#[tauri::command]
fn set_microphone(app: tauri::AppHandle, device_name: String) -> Result<(), String> {
    use cpal::traits::{DeviceTrait, HostTrait};

    let host = cpal::default_host();
    let device = if device_name.is_empty() || device_name == "System Default" {
        host.default_input_device()
            .ok_or_else(|| "No default input device found".to_string())?
    } else {
        host.input_devices()
            .map_err(|e| e.to_string())?
            .find(|d| {
                d.description()
                    .map(|desc: cpal::DeviceDescription| desc.name() == device_name.as_str())
                    .unwrap_or(false)
            })
            .ok_or_else(|| format!("Input device '{}' not found", device_name))?
    };

    let new_capture = audio::start_persistent_stream_with_device(device)
        .map_err(|e| e.to_string())?;

    // Replace the inner AudioCapture — old stream drops, new one starts
    {
        let audio_mutex = app.state::<audio::AudioCaptureMutex>();
        let mut guard = audio_mutex.0.lock().map_err(|e| format!("state lock failed: {}", e))?;
        *guard = Some(new_capture);
    }

    // Persist to settings.json
    let mut json = read_settings(&app)?;
    json["microphone_device"] = serde_json::Value::String(device_name.clone());
    write_settings(&app, &json)?;

    log::info!("Microphone switched to: '{}'", device_name);
    Ok(())
}

/// Read the saved whisper model ID from settings.json.
/// Returns None if not set (auto-detect will be used).
#[cfg(feature = "whisper")]
fn read_saved_model_id(app: &tauri::App) -> Option<String> {
    let data_dir = app.path().app_data_dir().ok()?;
    let settings_path = data_dir.join("settings.json");
    let contents = std::fs::read_to_string(&settings_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&contents).ok()?;
    json.get("whisper_model_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned())
}

/// Map a model_id string to the expected model file path.
///
/// Returns Err if the model_id is unknown.
#[cfg(feature = "whisper")]
fn model_id_to_path(model_id: &str) -> Result<std::path::PathBuf, String> {
    use crate::transcribe::models_dir;
    let filename = match model_id {
        "large-v3-turbo" => "ggml-large-v3-turbo-q5_0.bin",
        "small-en" => "ggml-small.en-q5_1.bin",
        _ => return Err(format!("Unknown model id: {}", model_id)),
    };
    Ok(models_dir().join(filename))
}

/// Info about a transcription model including availability and GPU recommendation.
///
/// Used by both Whisper models (feature-gated) and Parakeet (always visible so
/// the UI can show download status regardless of compiled features).
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ModelInfo {
    id: String,
    name: String,
    description: String,
    recommended: bool,
    downloaded: bool,
}

/// List available transcription models (Whisper + Parakeet) with download status.
#[cfg(feature = "whisper")]
#[tauri::command]
fn list_models(app: tauri::AppHandle) -> Result<Vec<ModelInfo>, String> {
    use crate::transcribe::{models_dir, ModelMode};
    let cached = app.state::<CachedGpuMode>();
    let gpu_mode = matches!(cached.0, ModelMode::Gpu);
    let dir = models_dir();

    let mut models = vec![
        ModelInfo {
            id: "large-v3-turbo".to_string(),
            name: "Large v3 Turbo".to_string(),
            description: "Best accuracy — 574 MB — requires NVIDIA GPU".to_string(),
            recommended: false,
            downloaded: dir.join("ggml-large-v3-turbo-q5_0.bin").exists(),
        },
        ModelInfo {
            id: "small-en".to_string(),
            name: "Small (English)".to_string(),
            description: "Fastest — 190 MB — works on any CPU".to_string(),
            recommended: !gpu_mode,
            downloaded: dir.join("ggml-small.en-q5_1.bin").exists(),
        },
    ];

    // Parakeet TDT int8 — always listed regardless of the parakeet feature flag
    // (download.rs is not feature-gated, so parakeet_model_exists() is always available).
    models.push(ModelInfo {
        id: "parakeet-tdt-v2".to_string(),
        name: "Parakeet TDT (int8)".to_string(),
        description: "Fastest — 661 MB — requires NVIDIA GPU (ONNX)".to_string(),
        recommended: false,
        downloaded: crate::download::parakeet_model_exists(),
    });

    // Parakeet TDT fp32 — full precision variant, recommended for GPU users
    models.push(ModelInfo {
        id: "parakeet-tdt-v2-fp32".to_string(),
        name: "Parakeet TDT (fp32)".to_string(),
        description: "Full precision — 2.56 GB — requires NVIDIA GPU (ONNX)".to_string(),
        recommended: gpu_mode,
        downloaded: crate::download::parakeet_fp32_model_exists(),
    });

    Ok(models)
}

/// Response type for check_first_run — tells the frontend whether setup is needed
/// and which model to recommend based on GPU detection.
#[cfg(feature = "whisper")]
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct FirstRunStatus {
    needs_setup: bool,
    gpu_detected: bool,
    recommended_model: String,
}

/// Check whether the app needs first-run model setup.
///
/// Returns needs_setup=true when no model file exists (neither Whisper nor Parakeet).
/// Also surfaces GPU detection so the frontend can pre-select the appropriate model.
#[cfg(feature = "whisper")]
#[tauri::command]
fn check_first_run(app: tauri::AppHandle) -> FirstRunStatus {
    use crate::transcribe::{models_dir, ModelMode};
    let cached = app.state::<CachedGpuMode>();
    let gpu_mode = matches!(cached.0, ModelMode::Gpu);
    let dir = models_dir();
    let large_exists = dir.join("ggml-large-v3-turbo-q5_0.bin").exists();
    let small_exists = dir.join("ggml-small.en-q5_1.bin").exists();
    // Parakeet variants are also valid installed models — skip first-run if any is present
    let parakeet_exists = crate::download::parakeet_model_exists();
    let parakeet_fp32_exists = crate::download::parakeet_fp32_model_exists();
    FirstRunStatus {
        needs_setup: !large_exists && !small_exists && !parakeet_exists && !parakeet_fp32_exists,
        gpu_detected: gpu_mode,
        recommended_model: if gpu_mode {
            "parakeet-tdt-v2-fp32".to_string()
        } else {
            "small-en".to_string()
        },
    }
}

/// Register VoiceType in Windows startup via tauri-plugin-autostart.
///
/// Called after the user completes the first-run setup flow (per locked decision:
/// "Auto-start with Windows enabled by default"). The plugin is already registered
/// in the builder — this command exposes the enable action to the frontend.
#[tauri::command]
async fn enable_autostart(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let autostart = app.autolaunch();
    autostart.enable().map_err(|e| e.to_string())
}

/// Switch the active whisper model. Reloads the WhisperContext without app restart.
///
/// Uses spawn_blocking because model loading is CPU-intensive.
/// Persists model_id to settings.json.
#[cfg(feature = "whisper")]
#[tauri::command]
async fn set_model(app: tauri::AppHandle, model_id: String) -> Result<(), String> {
    let model_path = model_id_to_path(&model_id)?;
    if !model_path.exists() {
        return Err(format!("Model file not downloaded: {}", model_path.display()));
    }

    // Skip reload if the requested model is already loaded
    {
        let json = read_settings(&app)?;
        if let Some(current) = json.get("whisper_model_id").and_then(|v| v.as_str()) {
            if current == model_id {
                log::info!("Whisper model '{}' already loaded, skipping reload", model_id);
                return Ok(());
            }
        }
    }

    let path_str = model_path.to_string_lossy().to_string();
    let model_id_clone = model_id.clone();

    // Determine GPU mode: large-v3-turbo uses GPU, others use CPU
    let mode = if model_id == "large-v3-turbo" {
        crate::transcribe::ModelMode::Gpu
    } else {
        crate::transcribe::ModelMode::Cpu
    };

    // Load new context on a blocking thread — model loading is CPU-intensive
    let new_ctx = tauri::async_runtime::spawn_blocking(move || {
        crate::transcribe::load_whisper_context(&path_str, &mode)
    })
    .await
    .map_err(|e| format!("spawn_blocking panicked: {}", e))?
    .map_err(|e| e)?;

    // Replace WhisperContext in managed state
    {
        let whisper_mutex = app.state::<WhisperStateMutex>();
        let mut guard = whisper_mutex.0.lock().map_err(|e| format!("state lock failed: {}", e))?;
        *guard = Some(Arc::new(new_ctx));
    }

    // Persist model_id to settings.json
    let mut json = read_settings(&app)?;
    json["whisper_model_id"] = serde_json::Value::String(model_id_clone.clone());
    write_settings(&app, &json)?;

    log::info!("Whisper model switched to: '{}'", model_id_clone);
    Ok(())
}

/// Reads a WAV file and decodes it to mono f32 samples.
///
/// Supports float and integer WAV formats. Downmixes multi-channel audio to mono.
/// Shared by transcribe_test_file and force_cpu_transcribe.
#[cfg(feature = "whisper")]
#[cfg(debug_assertions)]
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
#[cfg(debug_assertions)]
#[tauri::command]
async fn transcribe_test_file(
    app: tauri::AppHandle,
    path: String,
) -> Result<String, String> {
    use std::time::Instant;

    let start = Instant::now();

    // Get the WhisperContext from managed state — lock, clone Arc, drop guard before blocking work
    let ctx = {
        let state = app.state::<WhisperStateMutex>();
        let guard = state.0.lock().map_err(|e| format!("state lock failed: {}", e))?;
        match guard.as_ref() {
            Some(ctx) => Arc::clone(ctx),
            None => {
                return Err(
                    "Whisper model not loaded. Check startup logs for the download instructions."
                        .to_string(),
                );
            }
        }
    };

    let (audio_f32, _sample_rate) = read_wav_to_f32(&path)?;

    // Run inference on a blocking thread to avoid stalling the Tauri async runtime
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(transcribe::transcribe_audio(&ctx, &audio_f32, ""));
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
#[cfg(debug_assertions)]
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
        let result = ctx_result.and_then(|ctx| transcribe::transcribe_audio(&ctx, &audio_f32, ""));
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

    // Cache GPU detection BEFORE Builder::run() — Tauri creates webviews during
    // run() and the webview2 COM init pumps the Win32 message loop, which lets
    // the frontend call check_first_run/list_models before setup() even fires.
    #[cfg(feature = "whisper")]
    let cached_gpu = {
        let mode = transcribe::detect_gpu();
        log::info!("GPU detection cached: {:?}", mode);
        mode
    };

    let mut builder = tauri::Builder::default()
        // single-instance MUST be registered first (before setup)
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Second instance launched — show and focus existing settings window
            if let Some(w) = app.get_webview_window("settings") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_autostart::Builder::new().build());

    // GPU cache MUST be registered on Builder (before .run()) because webview2
    // COM init pumps Win32 messages, allowing frontend IPC before setup() runs.
    #[cfg(feature = "whisper")]
    {
        builder = builder.manage(CachedGpuMode(cached_gpu));
    }

    // ActiveEngine MUST be registered on Builder (same reason as CachedGpuMode).
    // setup() will overwrite with the saved value from settings.json.
    builder = builder.manage(ActiveEngine(std::sync::Mutex::new(TranscriptionEngine::Whisper)));

    // ParakeetStateMutex starts as None — model is loaded on demand (engine switch)
    // or at startup if saved engine is Parakeet.
    #[cfg(feature = "parakeet")]
    {
        builder = builder.manage(ParakeetStateMutex(std::sync::Mutex::new(None)));
    }

    builder.invoke_handler(tauri::generate_handler![
            rebind_hotkey,
            set_recording_mode,
            get_recording_mode,
            get_engine,
            set_engine,
            start_recording,
            stop_recording,
            save_test_wav,
            list_input_devices,
            set_microphone,
            get_profiles,
            set_active_profile,
            get_corrections,
            save_corrections,
            set_all_caps,
            download::download_model,
            download::download_parakeet_model,
            download::download_parakeet_fp32_model,
            enable_autostart,
            #[cfg(feature = "whisper")]
            check_first_run,
            #[cfg(feature = "whisper")]
            list_models,
            #[cfg(feature = "whisper")]
            set_model,
            #[cfg(all(feature = "whisper", debug_assertions))]
            transcribe_test_file,
            #[cfg(all(feature = "whisper", debug_assertions))]
            force_cpu_transcribe,
        ])
        .setup(|app| {
            build_tray(app)?;

            // Configure pill overlay: no focus steal + restore saved position
            if let Some(pill_window) = app.get_webview_window("pill") {
                log::info!("Pill window found — applying configuration");

                // focusable(false) sets WS_EX_NOACTIVATE — pill never steals focus
                let _ = pill_window.set_focusable(false);

                // Disable DWM shadow — rectangular shadow doesn't respect CSS border-radius (tauri#11321)
                let _ = pill_window.set_shadow(false);

                log::info!("Pill overlay window configured (focusable=false, no-shadow)");
            }

            // Determine hotkey to register: use saved setting if present, else default
            let hotkey = read_saved_hotkey(app)
                .unwrap_or_else(|| "ctrl+shift+space".to_owned());

            log::info!("Registering hotkey: {}", hotkey);

            // Register pipeline state machine BEFORE hotkey handler
            app.manage(pipeline::PipelineState::new());

            // Register RMS level stream control flag (starts false, toggled in hotkey handler)
            let level_stream_active = Arc::new(AtomicBool::new(false));
            app.manage(LevelStreamActive(level_stream_active));

            // Load and register recording mode from saved settings
            let saved_mode = read_saved_mode(app);
            log::info!("Recording mode: {:?}", saved_mode);
            app.manage(RecordingMode::new(saved_mode));
            app.manage(VadWorkerState(std::sync::Mutex::new(None)));

            // Update ActiveEngine from saved settings (Builder registered it as Whisper default).
            {
                let saved_engine = read_saved_engine(app);
                log::info!("Transcription engine (saved): {:?}", saved_engine);
                let engine_state = app.state::<ActiveEngine>();
                let mut guard = engine_state.0.lock().unwrap_or_else(|e| e.into_inner());
                *guard = saved_engine;
            }

            // If saved engine is Parakeet and parakeet feature is enabled, load model at startup.
            // Uses variant-aware directory resolution so fp32 is loaded if that was the last selected variant.
            #[cfg(feature = "parakeet")]
            {
                let saved_engine = {
                    let engine_state = app.state::<ActiveEngine>();
                    let guard = engine_state.0.lock().unwrap_or_else(|e| e.into_inner());
                    *guard
                };
                if saved_engine == TranscriptionEngine::Parakeet {
                    let parakeet_model_id = read_saved_parakeet_model_startup(app);
                    let model_dir = resolve_parakeet_dir(&parakeet_model_id);
                    if model_dir.exists() {
                        let dir_str = model_dir.to_string_lossy().to_string();
                        match transcribe_parakeet::load_parakeet(&dir_str, true) {
                            Ok(p) => {
                                let parakeet_state = app.state::<ParakeetStateMutex>();
                                let mut guard =
                                    parakeet_state.0.lock().unwrap_or_else(|e| e.into_inner());
                                *guard = Some(std::sync::Arc::new(std::sync::Mutex::new(p)));
                                log::info!(
                                    "Parakeet model loaded at startup (variant: {})",
                                    parakeet_model_id
                                );
                            }
                            Err(e) => {
                                log::warn!(
                                    "Parakeet startup load failed: {} — falling back to Whisper",
                                    e
                                );
                                let engine_state = app.state::<ActiveEngine>();
                                let mut guard =
                                    engine_state.0.lock().unwrap_or_else(|e| e.into_inner());
                                *guard = TranscriptionEngine::Whisper;
                            }
                        }
                    } else {
                        log::warn!(
                            "Parakeet set as engine but model files not found (variant: {}) — falling back to Whisper",
                            parakeet_model_id
                        );
                        let engine_state = app.state::<ActiveEngine>();
                        let mut guard = engine_state.0.lock().unwrap_or_else(|e| e.into_inner());
                        *guard = TranscriptionEngine::Whisper;
                    }
                }
            }

            // Load and register vocabulary profile + corrections engine
            {
                let profile_id = read_saved_profile_id(app);
                let mut active_profile = match profile_id.as_str() {
                    "structural-engineering" => profiles::structural_engineering_profile(),
                    _ => profiles::general_profile(),
                };

                // Merge user-saved corrections for this profile (user overrides defaults)
                let user_corrections = read_saved_corrections(app, &profile_id);
                for (k, v) in user_corrections {
                    active_profile.corrections.insert(k, v);
                }

                // Load saved ALL CAPS flag for this profile
                active_profile.all_caps = read_saved_all_caps(app, &profile_id);

                // Build corrections engine from merged corrections
                let engine = corrections::CorrectionsEngine::from_map(&active_profile.corrections)
                    .unwrap_or_else(|e| {
                        log::error!("Failed to build corrections engine at startup: {}", e);
                        corrections::CorrectionsEngine::from_map(&std::collections::HashMap::new()).unwrap()
                    });

                log::info!("Active vocabulary profile: '{}' (all_caps={})", active_profile.id, active_profile.all_caps);
                app.manage(profiles::ActiveProfile(std::sync::Mutex::new(active_profile)));
                app.manage(corrections::CorrectionsState(std::sync::Mutex::new(engine)));
            }

            // Register global hotkey plugin (desktop only — no Android/iOS support)
            #[cfg(desktop)]
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_shortcuts([hotkey.as_str()])?
                    .with_handler(|app, _shortcut, event| {
                        handle_shortcut(app, &event);
                    })
                    .build(),
            )?;

            // Start persistent audio capture stream.
            // Prefer saved mic device if found; fall back to system default silently (RESEARCH.md Pitfall 6).
            // App continues even if microphone is unavailable.
            let saved_mic_name = read_saved_mic(app);
            let capture_result = if let Some(ref name) = saved_mic_name {
                use cpal::traits::{DeviceTrait, HostTrait};
                let host = cpal::default_host();
                let found = host.input_devices().ok().and_then(|mut devs| {
                    devs.find(|d| {
                        d.description()
                            .map(|desc: cpal::DeviceDescription| desc.name() == name.as_str())
                            .unwrap_or(false)
                    })
                });
                if let Some(device) = found {
                    log::info!("Restoring saved microphone: '{}'", name);
                    audio::start_persistent_stream_with_device(device)
                } else {
                    log::warn!("Saved microphone '{}' not found — falling back to system default", name);
                    audio::start_persistent_stream()
                }
            } else {
                audio::start_persistent_stream()
            };
            match capture_result {
                Ok(capture) => {
                    log::info!("Audio capture initialized successfully");
                    app.manage(audio::AudioCaptureMutex(std::sync::Mutex::new(Some(capture))));
                }
                Err(e) => {
                    log::error!("Audio capture failed to initialize: {} — recording commands will not function", e);
                    app.manage(audio::AudioCaptureMutex(std::sync::Mutex::new(None)));
                    log::warn!("Audio state registered as None — start_recording/stop_recording will return errors");
                }
            }

            // Load whisper model (only when compiled with "whisper" feature).
            #[cfg(feature = "whisper")]
            {
                // If user has a saved model preference, try that first.
                // Fall back to GPU-auto-detection if saved model file is missing or no preference.
                let saved_model_id = read_saved_model_id(app);
                let whisper_ctx = if let Some(ref model_id) = saved_model_id {
                    // Try to load the saved model
                    let model_path_result = model_id_to_path(model_id);
                    match model_path_result {
                        Ok(path) if path.exists() => {
                            let path_str = path.to_string_lossy().to_string();
                            // Determine GPU mode based on model selection
                            let mode = if model_id == "large-v3-turbo" {
                                transcribe::ModelMode::Gpu
                            } else {
                                transcribe::ModelMode::Cpu
                            };
                            match transcribe::load_whisper_context(&path_str, &mode) {
                                Ok(ctx) => {
                                    log::info!("Whisper context loaded from saved model '{}' ({:?} mode)", model_id, mode);
                                    Some(Arc::new(ctx))
                                }
                                Err(e) => {
                                    log::warn!("Saved model '{}' failed to load: {} — falling back to auto-detect", model_id, e);
                                    None
                                }
                            }
                        }
                        _ => {
                            log::warn!("Saved model '{}' file not found — falling back to auto-detect", model_id);
                            None
                        }
                    }
                } else {
                    None
                };

                // If saved model didn't load, fall back to GPU auto-detection
                let whisper_ctx = if whisper_ctx.is_none() {
                    let mode = app.state::<CachedGpuMode>().0.clone();
                    log::info!("Inference mode selected (cached): {:?}", mode);
                    match transcribe::resolve_model_path(&mode) {
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
                    }
                } else {
                    whisper_ctx
                };

                app.manage(WhisperStateMutex(std::sync::Mutex::new(whisper_ctx)));
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
