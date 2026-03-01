mod audio;
mod corrections;
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

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tauri::Manager;
use tray::build_tray;

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

#[tauri::command]
fn set_recording_mode(app: tauri::AppHandle, mode: String) -> Result<(), String> {
    // Update managed state immediately
    let recording_mode = app.state::<RecordingMode>();
    match mode.as_str() {
        "toggle" => recording_mode.set(Mode::Toggle),
        _ => recording_mode.set(Mode::HoldToTalk),
    }
    // Persist to settings.json (merge into existing JSON — same pattern as hotkey)
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let settings_path = data_dir.join("settings.json");
    let mut json: serde_json::Value = std::fs::read_to_string(&settings_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    json["recording_mode"] = serde_json::Value::String(mode);
    std::fs::write(&settings_path, serde_json::to_string_pretty(&json).unwrap())
        .map_err(|e| e.to_string())?;
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

#[tauri::command]
fn rebind_hotkey(app: tauri::AppHandle, old: String, new_key: String) -> Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    let gs = app.global_shortcut();

    if !old.is_empty() {
        gs.unregister(old.as_str()).map_err(|e| e.to_string())?;
    }

    gs.on_shortcut(new_key.as_str(), |app, _shortcut, event| {
        use tauri_plugin_global_shortcut::ShortcutState;
        use std::sync::atomic::Ordering;
        use tauri::Emitter;

        let pipeline = app.state::<pipeline::PipelineState>();

        match event.state {
            ShortcutState::Pressed => {
                let mode = app.state::<RecordingMode>().get();

                match mode {
                    Mode::HoldToTalk => {
                        // Existing behavior: start on press (release stops)
                        if pipeline.transition(pipeline::IDLE, pipeline::RECORDING) {
                            let audio_mutex = app.state::<audio::AudioCaptureMutex>();
                            let audio = audio_mutex.0.lock().unwrap();
                            audio.clear_buffer();
                            audio.recording.store(true, Ordering::Relaxed);
                            let buffer_clone = audio.buffer.clone();
                            drop(audio);
                            tray::set_tray_state(app, tray::TrayState::Recording);

                            // Pill: show and set recording state
                            app.emit_to("pill", "pill-show", ()).ok();
                            app.emit_to("pill", "pill-state", "recording").ok();

                            // Start RMS level stream
                            let stream_active = app.state::<LevelStreamActive>();
                            stream_active.0.store(true, Ordering::Relaxed);
                            pill::start_level_stream(
                                app.clone(),
                                buffer_clone,
                                stream_active.0.clone(),
                            );

                            log::info!("Pipeline: IDLE -> RECORDING (hold-to-talk, rebound hotkey)");
                        }
                    }
                    Mode::Toggle => {
                        if pipeline.transition(pipeline::IDLE, pipeline::RECORDING) {
                            // First tap: start recording
                            let audio_mutex = app.state::<audio::AudioCaptureMutex>();
                            let audio = audio_mutex.0.lock().unwrap();
                            audio.clear_buffer();
                            audio.recording.store(true, Ordering::Relaxed);
                            let buffer_clone = audio.buffer.clone();
                            drop(audio);
                            tray::set_tray_state(app, tray::TrayState::Recording);
                            app.emit_to("pill", "pill-show", ()).ok();
                            app.emit_to("pill", "pill-state", "recording").ok();
                            let stream_active = app.state::<LevelStreamActive>();
                            stream_active.0.store(true, Ordering::Relaxed);
                            pill::start_level_stream(
                                app.clone(),
                                buffer_clone.clone(),
                                stream_active.0.clone(),
                            );

                            // Spawn VAD worker for auto-stop
                            let vad_handle = vad::spawn_vad_worker(
                                app.clone(),
                                buffer_clone,
                            );
                            let vad_state = app.state::<VadWorkerState>();
                            if let Ok(mut guard) = vad_state.0.lock() {
                                *guard = Some(vad_handle);
                            }

                            log::info!("Pipeline: IDLE -> RECORDING (toggle mode, VAD worker started, rebound hotkey)");
                        } else if pipeline.transition(pipeline::RECORDING, pipeline::PROCESSING) {
                            // Second tap: instant hard stop — go straight to transcription
                            // Cancel VAD worker first
                            let vad_state = app.state::<VadWorkerState>();
                            if let Ok(mut guard) = vad_state.0.lock() {
                                if let Some(mut handle) = guard.take() {
                                    handle.cancel();
                                }
                            }

                            // Stop recording and level stream
                            let stream_active = app.state::<LevelStreamActive>();
                            stream_active.0.store(false, Ordering::Relaxed);
                            app.emit_to("pill", "pill-state", "processing").ok();
                            tray::set_tray_state(app, tray::TrayState::Processing);
                            log::info!("Pipeline: RECORDING -> PROCESSING (toggle mode, second tap, rebound hotkey)");

                            let app_handle = app.clone();
                            tauri::async_runtime::spawn(async move {
                                pipeline::run_pipeline(app_handle).await;
                            });
                        }
                        // If PROCESSING, ignore tap (CAS prevents double-execution)
                    }
                }
            }
            ShortcutState::Released => {
                let mode = app.state::<RecordingMode>().get();
                match mode {
                    Mode::HoldToTalk => {
                        // Existing behavior: release stops recording
                        if pipeline.transition(pipeline::RECORDING, pipeline::PROCESSING) {
                            // Stop RMS level stream BEFORE transitioning to processing
                            let stream_active = app.state::<LevelStreamActive>();
                            stream_active.0.store(false, Ordering::Relaxed);

                            // Pill: switch to processing state (bars stop, animated border starts)
                            app.emit_to("pill", "pill-state", "processing").ok();

                            tray::set_tray_state(app, tray::TrayState::Processing);
                            log::info!("Pipeline: RECORDING -> PROCESSING (hold-to-talk release, rebound hotkey)");
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
    })
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Start recording: clears the audio buffer and sets the recording flag.
///
/// Audio captured after this call is accumulated in memory at 16kHz mono.
#[tauri::command]
fn start_recording(state: tauri::State<'_, audio::AudioCaptureMutex>) -> Result<(), String> {
    let guard = state.0.lock().unwrap();
    guard.clear_buffer();
    guard.recording.store(true, std::sync::atomic::Ordering::Relaxed);
    log::info!("Recording started");
    Ok(())
}

/// Stop recording: clears the recording flag, flushes the resampler,
/// and returns the number of 16kHz samples captured.
#[tauri::command]
fn stop_recording(state: tauri::State<'_, audio::AudioCaptureMutex>) -> Result<usize, String> {
    let guard = state.0.lock().unwrap();
    let n = guard.flush_and_stop();
    let seconds = n as f32 / 16000.0;
    log::info!("Recording stopped: {} samples ({:.1}s)", n, seconds);
    Ok(n)
}

/// Save the captured audio buffer to a WAV file in test-fixtures/.
///
/// Returns the file path on success.
#[tauri::command]
fn save_test_wav(app: tauri::AppHandle, state: tauri::State<'_, audio::AudioCaptureMutex>) -> Result<String, String> {
    let samples = state.0.lock().unwrap().get_buffer();
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
struct ProfileInfo {
    id: String,
    name: String,
    is_active: bool,
}

/// Returns the list of available profiles with their IDs, names, and active flag.
#[tauri::command]
fn get_profiles(app: tauri::AppHandle) -> Vec<ProfileInfo> {
    let active_id = {
        let state = app.state::<profiles::ActiveProfile>();
        let guard = state.0.lock().unwrap();
        guard.id.clone()
    };
    profiles::get_all_profiles()
        .into_iter()
        .map(|p| ProfileInfo {
            is_active: p.id == active_id,
            id: p.id,
            name: p.name,
        })
        .collect()
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
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let settings_path = data_dir.join("settings.json");
    let mut json: serde_json::Value = std::fs::read_to_string(&settings_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));

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
        let mut guard = profile_state.0.lock().unwrap();
        *guard = new_profile;
    }
    {
        let corrections_state = app.state::<corrections::CorrectionsState>();
        let mut guard = corrections_state.0.lock().unwrap();
        *guard = engine;
    }

    // Persist active profile ID
    json["active_profile_id"] = serde_json::Value::String(profile_id.clone());
    std::fs::write(&settings_path, serde_json::to_string_pretty(&json).unwrap())
        .map_err(|e| e.to_string())?;

    log::info!("Active profile set to: {}", profile_id);
    Ok(())
}

/// Returns the current active profile's corrections dictionary (for the UI editor).
#[tauri::command]
fn get_corrections(app: tauri::AppHandle) -> std::collections::HashMap<String, String> {
    let state = app.state::<profiles::ActiveProfile>();
    let guard = state.0.lock().unwrap();
    guard.corrections.clone()
}

/// Save user corrections for the active profile. Merges with defaults and rebuilds engine.
/// Persists to settings.json under `corrections.{profile_id}`.
#[tauri::command]
fn save_corrections(app: tauri::AppHandle, corrections_map: std::collections::HashMap<String, String>) -> Result<(), String> {
    let profile_id = {
        let state = app.state::<profiles::ActiveProfile>();
        let mut guard = state.0.lock().unwrap();
        // Merge user corrections into profile
        guard.corrections.extend(corrections_map.clone());
        guard.id.clone()
    };

    // Rebuild engine from updated corrections
    let engine = {
        let state = app.state::<profiles::ActiveProfile>();
        let guard = state.0.lock().unwrap();
        corrections::CorrectionsEngine::from_map(&guard.corrections)?
    };
    {
        let corrections_state = app.state::<corrections::CorrectionsState>();
        let mut guard = corrections_state.0.lock().unwrap();
        *guard = engine;
    }

    // Persist user corrections to settings.json
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let settings_path = data_dir.join("settings.json");
    let mut json: serde_json::Value = std::fs::read_to_string(&settings_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));

    let key = format!("corrections.{}", profile_id);
    json[&key] = serde_json::to_value(&corrections_map).unwrap();
    std::fs::write(&settings_path, serde_json::to_string_pretty(&json).unwrap())
        .map_err(|e| e.to_string())?;

    log::info!("Corrections saved for profile '{}'", profile_id);
    Ok(())
}

/// Toggle ALL CAPS for the active profile. Persists to settings.json.
#[tauri::command]
fn set_all_caps(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let profile_id = {
        let state = app.state::<profiles::ActiveProfile>();
        let mut guard = state.0.lock().unwrap();
        guard.all_caps = enabled;
        guard.id.clone()
    };

    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let settings_path = data_dir.join("settings.json");
    let mut json: serde_json::Value = std::fs::read_to_string(&settings_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));

    let key = format!("profiles.{}.all_caps", profile_id);
    json[&key] = serde_json::Value::Bool(enabled);
    std::fs::write(&settings_path, serde_json::to_string_pretty(&json).unwrap())
        .map_err(|e| e.to_string())?;

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
        let mut guard = audio_mutex.0.lock().unwrap();
        *guard = new_capture;
    }

    // Persist to settings.json
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let settings_path = data_dir.join("settings.json");
    let mut json: serde_json::Value = std::fs::read_to_string(&settings_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    json["microphone_device"] = serde_json::Value::String(device_name.clone());
    std::fs::write(&settings_path, serde_json::to_string_pretty(&json).unwrap())
        .map_err(|e| e.to_string())?;

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
        "medium" => "ggml-medium.bin",
        "small-en" => "ggml-small.en-q5_1.bin",
        _ => return Err(format!("Unknown model id: {}", model_id)),
    };
    Ok(models_dir().join(filename))
}

/// Info about a whisper model including availability and GPU recommendation.
#[cfg(feature = "whisper")]
#[derive(serde::Serialize)]
struct ModelInfo {
    id: String,
    name: String,
    description: String,
    recommended: bool,
    downloaded: bool,
}

/// List available whisper models with download status and GPU recommendation.
#[cfg(feature = "whisper")]
#[tauri::command]
fn list_models() -> Result<Vec<ModelInfo>, String> {
    use crate::transcribe::{detect_gpu, models_dir, ModelMode};
    let gpu_mode = matches!(detect_gpu(), ModelMode::Gpu);
    let dir = models_dir();

    Ok(vec![
        ModelInfo {
            id: "large-v3-turbo".to_string(),
            name: "Large v3 Turbo".to_string(),
            description: "Best accuracy, requires NVIDIA GPU".to_string(),
            recommended: gpu_mode,
            downloaded: dir.join("ggml-large-v3-turbo-q5_0.bin").exists(),
        },
        ModelInfo {
            id: "medium".to_string(),
            name: "Medium".to_string(),
            description: "Balanced speed and accuracy".to_string(),
            recommended: false,
            downloaded: dir.join("ggml-medium.bin").exists(),
        },
        ModelInfo {
            id: "small-en".to_string(),
            name: "Small (English)".to_string(),
            description: "Fastest, works without GPU".to_string(),
            recommended: !gpu_mode,
            downloaded: dir.join("ggml-small.en-q5_1.bin").exists(),
        },
    ])
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
        let mut guard = whisper_mutex.0.lock().unwrap();
        *guard = Some(Arc::new(new_ctx));
    }

    // Persist model_id to settings.json
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let settings_path = data_dir.join("settings.json");
    let mut json: serde_json::Value = std::fs::read_to_string(&settings_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    json["whisper_model_id"] = serde_json::Value::String(model_id_clone.clone());
    std::fs::write(&settings_path, serde_json::to_string_pretty(&json).unwrap())
        .map_err(|e| e.to_string())?;

    log::info!("Whisper model switched to: '{}'", model_id_clone);
    Ok(())
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

    // Get the WhisperContext from managed state — lock, clone Arc, drop guard before blocking work
    let ctx = {
        let state = app.state::<WhisperStateMutex>();
        let guard = state.0.lock().unwrap();
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
            set_recording_mode,
            get_recording_mode,
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
            #[cfg(feature = "whisper")]
            list_models,
            #[cfg(feature = "whisper")]
            set_model,
            #[cfg(feature = "whisper")]
            transcribe_test_file,
            #[cfg(feature = "whisper")]
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

                // Restore saved pill position from settings.json (sync read — same pattern as read_saved_hotkey)
                let data_dir = app.path().app_data_dir().ok();
                if let Some(dir) = data_dir {
                    let settings_path = dir.join("settings.json");
                    if let Ok(contents) = std::fs::read_to_string(&settings_path) {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&contents) {
                            if let Some(pos) = json.get("pill-position") {
                                if let (Some(x), Some(y)) = (
                                    pos.get("x").and_then(|v| v.as_f64()),
                                    pos.get("y").and_then(|v| v.as_f64()),
                                ) {
                                    let _ = pill_window.set_position(tauri::PhysicalPosition::new(x as i32, y as i32));
                                    log::info!("Pill position restored to ({}, {})", x, y);
                                }
                            }
                        }
                    }
                }

                log::info!("Pill overlay window configured (focusable=false, position restored)");
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
                        use tauri_plugin_global_shortcut::ShortcutState;
                        use std::sync::atomic::Ordering;
                        use tauri::Emitter;

                        let pipeline = app.state::<pipeline::PipelineState>();

                        match event.state {
                            ShortcutState::Pressed => {
                                let mode = app.state::<RecordingMode>().get();

                                match mode {
                                    Mode::HoldToTalk => {
                                        // Existing behavior: start on press (release stops)
                                        if pipeline.transition(pipeline::IDLE, pipeline::RECORDING) {
                                            let audio_mutex = app.state::<audio::AudioCaptureMutex>();
                                            let audio = audio_mutex.0.lock().unwrap();
                                            audio.clear_buffer();
                                            audio.recording.store(true, Ordering::Relaxed);
                                            let buffer_clone = audio.buffer.clone();
                                            drop(audio);
                                            tray::set_tray_state(app, tray::TrayState::Recording);

                                            // Pill: show and set recording state
                                            app.emit_to("pill", "pill-show", ()).ok();
                                            app.emit_to("pill", "pill-state", "recording").ok();

                                            // Start RMS level stream
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
                                            // First tap: start recording
                                            let audio_mutex = app.state::<audio::AudioCaptureMutex>();
                                            let audio = audio_mutex.0.lock().unwrap();
                                            audio.clear_buffer();
                                            audio.recording.store(true, Ordering::Relaxed);
                                            let buffer_clone = audio.buffer.clone();
                                            drop(audio);
                                            tray::set_tray_state(app, tray::TrayState::Recording);
                                            app.emit_to("pill", "pill-show", ()).ok();
                                            app.emit_to("pill", "pill-state", "recording").ok();
                                            let stream_active = app.state::<LevelStreamActive>();
                                            stream_active.0.store(true, Ordering::Relaxed);
                                            pill::start_level_stream(
                                                app.clone(),
                                                buffer_clone.clone(),
                                                stream_active.0.clone(),
                                            );

                                            // Spawn VAD worker for auto-stop
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
                                            // Second tap: instant hard stop — go straight to transcription
                                            // Cancel VAD worker first (prevents double-trigger)
                                            let vad_state = app.state::<VadWorkerState>();
                                            if let Ok(mut guard) = vad_state.0.lock() {
                                                if let Some(mut handle) = guard.take() {
                                                    handle.cancel();
                                                }
                                            }

                                            // Stop recording and level stream
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
                                        // If PROCESSING, ignore tap (CAS prevents double-execution)
                                    }
                                }
                            }
                            ShortcutState::Released => {
                                let mode = app.state::<RecordingMode>().get();
                                match mode {
                                    Mode::HoldToTalk => {
                                        // Only fire pipeline if we were actually recording
                                        if pipeline.transition(pipeline::RECORDING, pipeline::PROCESSING) {
                                            // Stop RMS level stream BEFORE transitioning to processing
                                            let stream_active = app.state::<LevelStreamActive>();
                                            stream_active.0.store(false, Ordering::Relaxed);

                                            // Pill: switch to processing state (bars stop, animated border starts)
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
                    app.manage(audio::AudioCaptureMutex(std::sync::Mutex::new(capture)));
                }
                Err(e) => {
                    log::error!("Audio capture failed to initialize: {} — recording commands will not function", e);
                    // Register a dummy capture — no device, but state type is required for commands
                    // Rather than crashing, we skip manage() and let commands fail gracefully.
                    // AudioCaptureMutex must still be registered for Tauri state to work.
                    // Use a dummy path: if stream fails, we cannot create AudioCapture.
                    // Best approach: log and skip manage; commands will panic on missing state.
                    // This matches original behavior (no AudioCapture managed = commands error).
                    log::warn!("Audio state not registered — start_recording/stop_recording will fail");
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
                    let mode = transcribe::detect_gpu();
                    log::info!("Inference mode selected (auto-detect): {:?}", mode);
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
