mod audio;
mod corrections;
mod download;
mod inject;
mod pill;
mod pipeline;
mod profiles;
mod tray;
mod vad;
mod updater;
#[cfg(windows)]
mod keyboard_hook;
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

// transcribe_moonshine.rs uses transcribe-rs (ONNX) for Moonshine Tiny batch inference.
// Gated so the project builds without the moonshine feature.
#[cfg(feature = "moonshine")]
mod transcribe_moonshine;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tauri::Manager;
use tray::build_tray;

/// Transcription engine selector: Whisper (default), Parakeet, or Moonshine.
///
/// Not feature-gated so settings persistence works regardless of compiled features.
/// Loaded from settings.json at startup via `read_saved_engine()`.
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TranscriptionEngine {
    Whisper,
    Parakeet,
    Moonshine,
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

/// Mutex-wrapped MoonshineEngine for runtime access.
///
/// Outer Mutex<Option<...>> mirrors ParakeetStateMutex — allows replacing the model
/// at runtime (load-on-demand or engine switch).
///
/// Inner Arc<Mutex<MoonshineEngine>>: Arc makes it clonable for spawn_blocking;
/// inner Mutex provides the `&mut self` access that transcribe_samples requires.
/// MoonshineEngine is not Sync (transcribe_samples takes &mut self), so it cannot be
/// wrapped in Arc alone — the inner Mutex serialises &mut access.
#[cfg(feature = "moonshine")]
pub struct MoonshineStateMutex(
    pub std::sync::Mutex<Option<std::sync::Arc<std::sync::Mutex<transcribe_rs::engines::moonshine::MoonshineEngine>>>>,
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

/// Managed state holding the keyboard hook handle (if installed).
/// Used for cleanup on app shutdown.
#[cfg(windows)]
pub struct HookHandleState(pub std::sync::Mutex<Option<keyboard_hook::HookHandle>>);

/// Tracks whether WH_KEYBOARD_LL hook installation succeeded.
/// Set at startup, queried by get_hook_status IPC and rebind_hotkey.
pub struct HookAvailable(pub std::sync::Arc<std::sync::atomic::AtomicBool>);

/// Set to true at the END of setup() after hook routing is resolved.
/// The notify_frontend_ready command checks this to decide whether to emit
/// hook-status-changed immediately or let setup() emit when it finishes.
pub struct SetupComplete(pub std::sync::atomic::AtomicBool);

/// Set to true when the frontend calls notify_frontend_ready (listener registered).
/// setup() checks this at completion to decide whether to emit hook-status-changed.
pub struct FrontendReady(pub std::sync::atomic::AtomicBool);

/// Returns true if the hotkey string contains only modifier tokens and no base key.
/// Used as the single routing predicate for hook vs global-shortcut backend.
///
/// "ctrl+win" -> true (modifier-only -> hook backend)
/// "ctrl+shift+v" -> false (has base key -> global-shortcut backend)
fn is_modifier_only(hotkey: &str) -> bool {
    const MODIFIER_TOKENS: &[&str] = &["ctrl", "alt", "shift", "meta", "win", "super"];
    !hotkey.is_empty()
        && hotkey
            .split('+')
            .filter(|t| !t.is_empty())
            .all(|t| MODIFIER_TOKENS.contains(&t.to_lowercase().as_str()))
}

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

/// Cached full GPU detection result. Populated once at startup alongside CachedGpuMode.
/// Provides GPU name, Parakeet provider recommendation, and NVIDIA flag.
/// Registered before Builder::run() for the same reason as CachedGpuMode.
#[cfg(feature = "whisper")]
pub struct CachedGpuDetection(pub transcribe::GpuDetection);

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
/// Defaults to Parakeet when `gpu_mode` is true, Whisper otherwise.
/// Falls back to the GPU-aware default on first launch, file missing, or parse error.
fn read_saved_engine(app: &tauri::App, gpu_mode: bool) -> TranscriptionEngine {
    let default_engine = if gpu_mode {
        TranscriptionEngine::Parakeet
    } else {
        TranscriptionEngine::Whisper
    };
    let data_dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(_) => return default_engine,
    };
    let settings_path = data_dir.join("settings.json");
    let contents = match std::fs::read_to_string(&settings_path) {
        Ok(c) => c,
        Err(_) => return default_engine,
    };
    let json: serde_json::Value = match serde_json::from_str(&contents) {
        Ok(j) => j,
        Err(_) => return default_engine,
    };
    match json.get("active_engine").and_then(|v| v.as_str()) {
        Some("parakeet") => TranscriptionEngine::Parakeet,
        Some("whisper") => TranscriptionEngine::Whisper,
        Some("moonshine") => TranscriptionEngine::Moonshine,
        _ => default_engine,
    }
}

/// Read the saved Parakeet model variant from settings.json.
/// Returns "parakeet-tdt-v2-fp32" by default.
fn read_saved_parakeet_model(app_handle: &tauri::AppHandle) -> String {
    let json = read_settings(app_handle).unwrap_or_else(|_| serde_json::json!({}));
    json.get("parakeet_model")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("parakeet-tdt-v2-fp32")
        .to_string()
}

/// Read the saved Parakeet model variant from settings.json at startup.
/// Returns "parakeet-tdt-v2-fp32" by default.
fn read_saved_parakeet_model_startup(app: &tauri::App) -> String {
    let data_dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(_) => return "parakeet-tdt-v2-fp32".to_string(),
    };
    let settings_path = data_dir.join("settings.json");
    let contents = match std::fs::read_to_string(&settings_path) {
        Ok(c) => c,
        Err(_) => return "parakeet-tdt-v2-fp32".to_string(),
    };
    let json: serde_json::Value = match serde_json::from_str(&contents) {
        Ok(j) => j,
        Err(_) => return "parakeet-tdt-v2-fp32".to_string(),
    };
    json.get("parakeet_model")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("parakeet-tdt-v2-fp32")
        .to_string()
}

/// Resolve the model directory for a Parakeet variant.
fn resolve_parakeet_dir(_model_id: &str) -> std::path::PathBuf {
    download::parakeet_fp32_model_dir()
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

/// Returns the current active transcription engine as a string ("whisper", "parakeet", or "moonshine").
#[tauri::command]
fn get_engine(app: tauri::AppHandle) -> String {
    let state = app.state::<ActiveEngine>();
    let guard = state.0.lock().unwrap_or_else(|e| e.into_inner());
    match *guard {
        TranscriptionEngine::Whisper => "whisper".to_string(),
        TranscriptionEngine::Parakeet => "parakeet".to_string(),
        TranscriptionEngine::Moonshine => "moonshine".to_string(),
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
        "moonshine" => TranscriptionEngine::Moonshine,
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
            #[cfg(feature = "whisper")]
            let provider = {
                let gpu_detection = app.state::<CachedGpuDetection>();
                gpu_detection.0.parakeet_provider.clone()
            };
            #[cfg(not(feature = "whisper"))]
            let provider = "cpu".to_string();
            match transcribe_parakeet::load_parakeet(&dir_str, &provider) {
                Ok(p) => {
                    let inner_arc = std::sync::Arc::new(std::sync::Mutex::new(p));
                    let warmup_arc = inner_arc.clone();
                    {
                        let mut guard = parakeet_state.0.lock().unwrap_or_else(|e| e.into_inner());
                        *guard = Some(inner_arc);
                    }
                    log::info!("Parakeet model loaded on engine switch (variant: {})", parakeet_model_id);
                    // Warm up after engine switch
                    std::thread::spawn(move || {
                        let mut guard = warmup_arc.lock().unwrap_or_else(|e| e.into_inner());
                        transcribe_parakeet::warm_up_parakeet(&mut guard);
                    });
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
    // If switching to Moonshine, load model with GPU provider
    #[cfg(feature = "moonshine")]
    if new_engine == TranscriptionEngine::Moonshine {
        let moonshine_state = app.state::<MoonshineStateMutex>();
        let model_dir = download::moonshine_tiny_model_dir();
        if model_dir.exists() {
            #[cfg(feature = "whisper")]
            let provider = {
                let gpu_detection = app.state::<CachedGpuDetection>();
                gpu_detection.0.parakeet_provider.clone()
            };
            #[cfg(not(feature = "whisper"))]
            let provider = "cpu".to_string();
            match transcribe_moonshine::load_moonshine(&model_dir, &provider) {
                Ok(engine_instance) => {
                    let inner_arc = std::sync::Arc::new(std::sync::Mutex::new(engine_instance));
                    let warmup_arc = inner_arc.clone();
                    {
                        let mut guard = moonshine_state.0.lock().unwrap_or_else(|e| e.into_inner());
                        *guard = Some(inner_arc);
                    }
                    log::info!("Moonshine model loaded on engine switch");
                    std::thread::spawn(move || {
                        let mut guard = warmup_arc.lock().unwrap_or_else(|e| e.into_inner());
                        transcribe_moonshine::warm_up_moonshine(&mut guard);
                    });
                }
                Err(e) => {
                    log::error!("Failed to load Moonshine on engine switch: {}", e);
                    let state = app.state::<ActiveEngine>();
                    let mut guard = state.0.lock().unwrap_or_else(|e| e.into_inner());
                    *guard = TranscriptionEngine::Whisper;
                    return Err(format!(
                        "Moonshine model failed to load: {}. Reverting to Whisper.",
                        e
                    ));
                }
            }
        } else {
            let state = app.state::<ActiveEngine>();
            let mut guard = state.0.lock().unwrap_or_else(|e| e.into_inner());
            *guard = TranscriptionEngine::Whisper;
            return Err(
                "Moonshine model not downloaded. Download it first from Settings.".to_string(),
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

/// Open an on-demand audio stream using the saved microphone preference.
///
/// Resolves the device from settings.json, opens a capture stream, clears the buffer,
/// sets recording=true, stores the AudioCapture in managed state, and returns the
/// buffer Arc for level streaming. Returns None if the stream could not be opened.
fn open_recording_stream(app: &tauri::AppHandle) -> Option<Arc<std::sync::Mutex<Vec<f32>>>> {
    let device_name = read_settings(app)
        .ok()
        .and_then(|json| json.get("microphone_device")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty() && *s != "System Default")
            .map(|s| s.to_owned()));

    let device = match audio::resolve_device_by_name(device_name.as_deref().unwrap_or("")) {
        Ok(d) => d,
        Err(e) => {
            log::error!("Cannot resolve microphone: {}", e);
            return None;
        }
    };

    let capture = match audio::open_stream_with_device(device) {
        Ok(c) => c,
        Err(e) => {
            log::error!("Cannot open audio stream: {}", e);
            return None;
        }
    };

    capture.clear_buffer();
    capture.recording.store(true, std::sync::atomic::Ordering::Relaxed);
    let buffer_clone = capture.buffer.clone();

    let audio_mutex = app.state::<audio::AudioCaptureMutex>();
    let mut guard = audio_mutex.0.lock().unwrap_or_else(|e| e.into_inner());
    *guard = Some(capture);

    Some(buffer_clone)
}

/// Hotkey handler body — called from both handle_shortcut() (global-shortcut path)
/// and dispatch_hook_event in keyboard_hook.rs (WH_KEYBOARD_LL path).
///
/// `pressed=true` maps to the Pressed branch; `pressed=false` to Released.
pub(crate) fn handle_hotkey_event(app: &tauri::AppHandle, pressed: bool) {
    use std::sync::atomic::Ordering;
    use tauri::Emitter;

    let pipeline = app.state::<pipeline::PipelineState>();

    if pressed {
        let mode = app.state::<RecordingMode>().get();

        match mode {
            Mode::HoldToTalk => {
                if pipeline.transition(pipeline::IDLE, pipeline::RECORDING) {
                    let buffer_clone = match open_recording_stream(app) {
                        Some(b) => b,
                        None => {
                            log::error!("No microphone — cannot record");
                            pipeline.reset_to_idle();
                            return;
                        }
                    };
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
                    let buffer_clone = match open_recording_stream(app) {
                        Some(b) => b,
                        None => {
                            log::error!("No microphone — cannot record (toggle)");
                            pipeline.reset_to_idle();
                            return;
                        }
                    };
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
    } else {
        // pressed=false — Released
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

/// Thin wrapper so the global-shortcut plugin handler can call handle_hotkey_event.
fn handle_shortcut(app: &tauri::AppHandle, event: &tauri_plugin_global_shortcut::ShortcutEvent) {
    use tauri_plugin_global_shortcut::ShortcutState;
    match event.state {
        ShortcutState::Pressed => handle_hotkey_event(app, true),
        ShortcutState::Released => handle_hotkey_event(app, false),
    }
}

#[tauri::command]
fn rebind_hotkey(app: tauri::AppHandle, old: String, new_key: String) -> Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    // Guard: refuse to switch backends mid-recording
    let pipeline = app.state::<pipeline::PipelineState>();
    if pipeline.current() != pipeline::Phase::Idle {
        return Err("Recording in progress — wait for it to finish before changing hotkey".to_string());
    }

    // Tear down old backend (old off before new on)
    if !old.is_empty() {
        if is_modifier_only(&old) {
            // Stop hook backend: take handle from managed state (Drop calls uninstall)
            #[cfg(windows)]
            {
                let hook_state = app.state::<HookHandleState>();
                let mut guard = hook_state.0.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(handle) = guard.take() {
                    handle.uninstall();
                }
            }
        } else {
            app.global_shortcut().unregister(old.as_str()).map_err(|e| e.to_string())?;
        }
    }

    // Start new backend
    if is_modifier_only(&new_key) {
        // Check hook availability before attempting modifier-only combo
        let hook_status = app.state::<HookAvailable>();
        if !hook_status.0.load(std::sync::atomic::Ordering::Relaxed) {
            // Hook not yet installed — attempt installation now
            #[cfg(windows)]
            {
                match keyboard_hook::install(app.clone()) {
                    Ok(handle) => {
                        hook_status.0.store(true, std::sync::atomic::Ordering::Relaxed);
                        let hook_state = app.state::<HookHandleState>();
                        let mut guard = hook_state.0.lock().unwrap_or_else(|e| e.into_inner());
                        *guard = Some(handle);
                        log::info!("Keyboard hook installed via rebind for: {}", new_key);
                    }
                    Err(e) => {
                        return Err(format!(
                            "Modifier-only combos require the keyboard hook, which failed to install: {}. Choose a standard combo like Ctrl+Shift+V.",
                            e
                        ));
                    }
                }
            }
            #[cfg(not(windows))]
            {
                return Err("Modifier-only combos require the keyboard hook, which is unavailable on this system. Choose a standard combo like Ctrl+Shift+V.".to_string());
            }
        } else {
            // Hook was previously installed — re-install to activate for new combo
            // (hook may have been stopped by the old-backend teardown above)
            #[cfg(windows)]
            {
                let hook_state = app.state::<HookHandleState>();
                let already_active = {
                    let guard = hook_state.0.lock().unwrap_or_else(|e| e.into_inner());
                    guard.is_some()
                };
                if !already_active {
                    match keyboard_hook::install(app.clone()) {
                        Ok(handle) => {
                            let mut guard = hook_state.0.lock().unwrap_or_else(|e| e.into_inner());
                            *guard = Some(handle);
                        }
                        Err(e) => {
                            return Err(format!("Failed to re-install keyboard hook: {}", e));
                        }
                    }
                }
            }
        }
    } else {
        app.global_shortcut()
            .on_shortcut(new_key.as_str(), |app, _shortcut, event| {
                handle_shortcut(app, &event);
            })
            .map_err(|e| e.to_string())?;
    }

    // Persist the new hotkey to settings.json so it survives app restarts.
    let mut json = read_settings(&app)?;
    json["hotkey"] = serde_json::Value::String(new_key);
    write_settings(&app, &json)?;

    Ok(())
}

/// Temporarily unregister the global hotkey so keystrokes can be captured
/// by the frontend hotkey-rebind UI without triggering the shortcut action.
#[tauri::command]
fn unregister_hotkey(app: tauri::AppHandle, key: String) -> Result<(), String> {
    if !key.is_empty() {
        if is_modifier_only(&key) {
            // Stop hook backend: take handle from managed state (Drop calls uninstall)
            #[cfg(windows)]
            {
                let hook_state = app.state::<HookHandleState>();
                let mut guard = hook_state.0.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(handle) = guard.take() {
                    handle.uninstall();
                }
            }
        } else {
            use tauri_plugin_global_shortcut::GlobalShortcutExt;
            app.global_shortcut().unregister(key.as_str()).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Re-register a previously unregistered hotkey (e.g. when the user cancels
/// the hotkey capture without choosing a new key).
#[tauri::command]
fn register_hotkey(app: tauri::AppHandle, key: String) -> Result<(), String> {
    if !key.is_empty() {
        if is_modifier_only(&key) {
            // Re-install hook backend
            #[cfg(windows)]
            {
                let hook_state = app.state::<HookHandleState>();
                let already_active = {
                    let guard = hook_state.0.lock().unwrap_or_else(|e| e.into_inner());
                    guard.is_some()
                };
                if !already_active {
                    match keyboard_hook::install(app.clone()) {
                        Ok(handle) => {
                            let mut guard = hook_state.0.lock().unwrap_or_else(|e| e.into_inner());
                            *guard = Some(handle);
                        }
                        Err(e) => {
                            return Err(format!("Failed to re-install keyboard hook: {}", e));
                        }
                    }
                }
            }
        } else {
            use tauri_plugin_global_shortcut::GlobalShortcutExt;
            app.global_shortcut()
                .on_shortcut(key.as_str(), |app, _shortcut, event| {
                    handle_shortcut(app, &event);
                })
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Returns whether the keyboard hook is available on this system.
/// Called by the frontend settings panel to display hook-failure warning.
#[tauri::command]
fn get_hook_status(app: tauri::AppHandle) -> bool {
    app.state::<HookAvailable>().0.load(std::sync::atomic::Ordering::Relaxed)
}

/// Called by the frontend after it has registered all event listeners (listen() resolved).
///
/// Solves the startup race: setup() emits "hook-status-changed" but the webview may not
/// have loaded JS (or the listen() round-trip may not be complete) when setup() fires.
/// Tauri events have no queue — if nobody is listening when the event fires, it is lost.
///
/// Protocol:
///   - Frontend registers listen("hook-status-changed") then calls notify_frontend_ready().
///   - If setup() has already completed: this handler emits the current hook status immediately.
///   - If setup() is still running: this handler sets FrontendReady=true; setup() will emit
///     when it finishes (it checks FrontendReady at completion).
///
/// Either way, the emit happens only after BOTH sides are ready, eliminating the race.
#[tauri::command]
fn notify_frontend_ready(app: tauri::AppHandle) {
    use std::sync::atomic::Ordering::Relaxed;
    app.state::<FrontendReady>().0.store(true, Relaxed);

    // If setup() has already completed, emit the current hook status now.
    // (setup() will not emit again — it only emits if FrontendReady was true at that moment.)
    if app.state::<SetupComplete>().0.load(Relaxed) {
        #[cfg(desktop)]
        {
            use tauri::Emitter;
            let hook_ok = app.state::<HookAvailable>().0.load(Relaxed);
            log::debug!("notify_frontend_ready: setup already complete, emitting hook-status-changed={}", hook_ok);
            if let Some(w) = app.get_webview_window("settings") {
                w.emit("hook-status-changed", hook_ok).ok();
            }
        }
    }
}

/// Start recording: opens an on-demand audio stream if none exists, then clears
/// the buffer and sets the recording flag.
///
/// Audio captured after this call is accumulated in memory at 16kHz mono.
#[tauri::command]
fn start_recording(app: tauri::AppHandle, state: tauri::State<'_, audio::AudioCaptureMutex>) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| format!("state lock failed: {}", e))?;
    if guard.is_none() {
        // Open stream on demand using saved microphone preference
        let device_name = read_settings(&app)
            .ok()
            .and_then(|json| json.get("microphone_device")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty() && *s != "System Default")
                .map(|s| s.to_owned()));
        let device = audio::resolve_device_by_name(device_name.as_deref().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        let capture = audio::open_stream_with_device(device)
            .map_err(|e| e.to_string())?;
        *guard = Some(capture);
    }
    let audio = guard.as_ref().unwrap();
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

/// Save the microphone device preference. No stream is opened — the preference
/// will be used on the next recording start.
///
/// Validates that the device exists before saving (unless "System Default").
/// Persists the selection to settings.json.
#[tauri::command]
fn set_microphone(app: tauri::AppHandle, device_name: String) -> Result<(), String> {
    // Validate device exists before saving (unless System Default)
    if !device_name.is_empty() && device_name != "System Default" {
        audio::resolve_device_by_name(&device_name)
            .map_err(|e| e.to_string())?;
    }

    // Persist to settings.json — stream will use this on next recording start
    let mut json = read_settings(&app)?;
    json["microphone_device"] = serde_json::Value::String(device_name.clone());
    write_settings(&app, &json)?;

    log::info!("Microphone preference saved: '{}' (will take effect on next recording)", device_name);
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
        "distil-large-v3.5" => "ggml-distil-large-v3.5.bin",
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
            description: "Most accurate — 574 MB — requires NVIDIA GPU".to_string(),
            recommended: false,
            downloaded: dir.join("ggml-large-v3-turbo-q5_0.bin").exists(),
        },
        ModelInfo {
            id: "small-en".to_string(),
            name: "Small (English)".to_string(),
            description: "Lightweight — 190 MB — GPU accelerated when available".to_string(),
            recommended: !gpu_mode,
            downloaded: dir.join("ggml-small.en-q5_1.bin").exists(),
        },
    ];

    // Distil Large v3.5 — high accuracy fp16, works on CPU and GPU
    models.push(ModelInfo {
        id: "distil-large-v3.5".to_string(),
        name: "Distil Large v3.5".to_string(),
        description: "High accuracy — 513 MB — GPU accelerated when available".to_string(),
        recommended: false,
        downloaded: dir.join("ggml-distil-large-v3.5.bin").exists(),
    });

    // Parakeet TDT fp32 — fast and accurate, supports CUDA, DirectML, and CPU
    models.push(ModelInfo {
        id: "parakeet-tdt-v2-fp32".to_string(),
        name: "Parakeet TDT (fp32)".to_string(),
        description: "Fast and accurate — 2.56 GB — GPU accelerated (CUDA or DirectML)".to_string(),
        recommended: gpu_mode,
        downloaded: crate::download::parakeet_fp32_model_exists(),
    });

    // Moonshine Tiny — fastest for short clips, works on any hardware
    models.push(ModelInfo {
        id: "moonshine-tiny".to_string(),
        name: "Moonshine Tiny".to_string(),
        description: "Fastest — 108 MB — ONNX-based, works on any hardware".to_string(),
        recommended: false,
        downloaded: crate::download::moonshine_tiny_model_exists(),
    });

    Ok(models)
}

/// GPU/inference status info returned to the frontend for display in ModelSection.
#[cfg(feature = "whisper")]
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct GpuInfo {
    gpu_name: String,
    execution_provider: String,
    active_model: String,
    active_engine: String,
}

/// Return current GPU info, execution provider, active engine, and active model.
///
/// Used by ModelSection to display the "Inference Status" indicator.
/// Refreshed whenever the user changes the model or engine selection.
#[cfg(feature = "whisper")]
#[tauri::command]
fn get_gpu_info(app: tauri::AppHandle) -> GpuInfo {
    let detection = app.state::<CachedGpuDetection>();
    let engine_state = app.state::<ActiveEngine>();
    let engine = engine_state.0.lock().unwrap_or_else(|e| e.into_inner());
    let engine_str = match *engine {
        TranscriptionEngine::Whisper => "whisper",
        TranscriptionEngine::Parakeet => "parakeet",
        TranscriptionEngine::Moonshine => "moonshine",
    };
    let settings = read_settings(&app).unwrap_or_else(|_| serde_json::json!({}));
    let active_model = match *engine {
        TranscriptionEngine::Parakeet => settings
            .get("parakeet_model")
            .and_then(|v| v.as_str())
            .unwrap_or("parakeet-tdt-v2-fp32")
            .to_string(),
        TranscriptionEngine::Moonshine => "moonshine-tiny".to_string(),
        _ => settings
            .get("whisper_model_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
    };
    let ep = match engine_str {
        "parakeet" | "moonshine" => detection.0.parakeet_provider.to_uppercase(),
        _ => {
            if detection.0.is_nvidia {
                "CUDA".to_string()
            } else {
                "CPU".to_string()
            }
        }
    };
    GpuInfo {
        gpu_name: detection.0.gpu_name.clone(),
        execution_provider: ep,
        active_model,
        active_engine: engine_str.to_string(),
    }
}

/// Response type for check_first_run — tells the frontend whether setup is needed
/// and which model to recommend based on GPU detection.
#[cfg(feature = "whisper")]
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct FirstRunStatus {
    needs_setup: bool,
    gpu_detected: bool,
    gpu_name: String,
    directml_available: bool,
    recommended_model: String,
}

/// Check whether the app needs first-run model setup.
///
/// Returns needs_setup=true when no model file exists (neither Whisper nor Parakeet).
/// Also surfaces GPU detection so the frontend can pre-select the appropriate model.
/// gpu_name: human-readable GPU name for display in the badge.
/// directml_available: true for non-NVIDIA systems (DirectML enables Parakeet GPU inference).
#[cfg(feature = "whisper")]
#[tauri::command]
fn check_first_run(app: tauri::AppHandle) -> FirstRunStatus {
    use crate::transcribe::{models_dir, ModelMode};
    let cached = app.state::<CachedGpuMode>();
    let gpu_mode = matches!(cached.0, ModelMode::Gpu);
    let detection = app.state::<CachedGpuDetection>();
    let dir = models_dir();
    let large_exists = dir.join("ggml-large-v3-turbo-q5_0.bin").exists();
    let small_exists = dir.join("ggml-small.en-q5_1.bin").exists();
    // Parakeet fp32 is also a valid installed model — skip first-run if it is present
    let parakeet_fp32_exists = crate::download::parakeet_fp32_model_exists();
    // Distil Large v3.5 is also a valid installed model — skip first-run if it is present
    let distil_v35_exists = dir.join("ggml-distil-large-v3.5.bin").exists();
    // Moonshine Tiny is also a valid installed model — skip first-run if it is present
    let moonshine_exists = crate::download::moonshine_tiny_model_exists();
    // directml_available: only true when a discrete non-NVIDIA GPU exists (AMD RX, Intel Arc).
    // Integrated-only GPUs (Intel UHD, AMD APU) cannot run Parakeet at useful speed via DirectML.
    let directml_available = detection.0.has_discrete_gpu && !detection.0.is_nvidia;
    FirstRunStatus {
        needs_setup: !large_exists && !small_exists && !parakeet_fp32_exists && !distil_v35_exists && !moonshine_exists,
        gpu_detected: gpu_mode,
        gpu_name: detection.0.gpu_name.clone(),
        directml_available,
        recommended_model: if gpu_mode {
            "parakeet-tdt-v2-fp32".to_string()
        } else if detection.0.has_discrete_gpu {
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

/// Notify the tray menu whether an update is available.
///
/// When `available == true`, adds an "Update Available" item at the top of the tray menu.
/// When `available == false`, restores the default menu.
/// Fire-and-forget — the frontend calls this after a successful update check.
#[tauri::command]
fn set_update_available(app: tauri::AppHandle, available: bool) {
    tray::set_tray_update_indicator(&app, available);
}

/// Check whether the recording/transcription pipeline is currently active.
///
/// Returns true if audio capture or transcription is in progress (LevelStreamActive flag is set).
/// Used by the frontend's restartNow() to defer relaunch until the user finishes dictating.
#[tauri::command]
fn is_pipeline_active(state: tauri::State<'_, LevelStreamActive>) -> bool {
    state.0.load(std::sync::atomic::Ordering::Relaxed)
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

    // Skip reload if the requested model is already loaded in memory
    {
        let json = read_settings(&app)?;
        if let Some(current) = json.get("whisper_model_id").and_then(|v| v.as_str()) {
            if current == model_id {
                let whisper_mutex = app.state::<WhisperStateMutex>();
                let guard = whisper_mutex.0.lock().map_err(|e| format!("state lock failed: {}", e))?;
                if guard.is_some() {
                    log::info!("Whisper model '{}' already loaded, skipping reload", model_id);
                    return Ok(());
                }
                log::info!("Whisper model '{}' in settings but not in memory, loading now", model_id);
            }
        }
    }

    let path_str = model_path.to_string_lossy().to_string();
    let model_id_clone = model_id.clone();

    // Determine GPU mode based on GPU availability (not model_id)
    let mode = app.state::<CachedGpuMode>().0.clone();

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

    // Full GPU detection for provider selection and UI display.
    // Runs alongside detect_gpu() at the same startup point.
    #[cfg(feature = "whisper")]
    let cached_gpu_detection = {
        let detection = transcribe::detect_gpu_full();
        log::info!("GPU detection full: {:?}", detection);
        detection
    };

    let mut builder = tauri::Builder::default()
        .device_event_filter(tauri::DeviceEventFilter::Always) // HOOK-03: fix tauri#13919
        // single-instance MUST be registered first (before setup)
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Second instance launched — show and focus existing settings window
            if let Some(w) = app.get_webview_window("settings") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_process::init());

    // GPU cache MUST be registered on Builder (before .run()) because webview2
    // COM init pumps Win32 messages, allowing frontend IPC before setup() runs.
    #[cfg(feature = "whisper")]
    {
        builder = builder.manage(CachedGpuMode(cached_gpu));
        builder = builder.manage(CachedGpuDetection(cached_gpu_detection));
    }

    // ActiveEngine MUST be registered on Builder (same reason as CachedGpuMode).
    // setup() will overwrite with the saved value from settings.json.
    builder = builder.manage(ActiveEngine(std::sync::Mutex::new(TranscriptionEngine::Whisper)));

    // HookHandleState starts as None — populated in setup() if hotkey is "ctrl+win".
    // Registered on Builder so it's available for cleanup in the run() callback.
    #[cfg(windows)]
    {
        builder = builder.manage(HookHandleState(std::sync::Mutex::new(None)));
    }

    // HookAvailable MUST be registered on Builder (same reason as CachedGpuMode):
    // webview2 COM init pumps Win32 messages, allowing frontend IPC (get_hook_status)
    // before setup() runs. Starts false; setup() updates via the shared Arc.
    builder = builder.manage(HookAvailable(std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false))));

    // SetupComplete and FrontendReady solve the startup event-emission race:
    // Tauri events are not queued — if the frontend isn't listening when setup() emits,
    // the event is lost. The notify_frontend_ready command coordinates both sides:
    //   - SetupComplete: set at END of setup() after hook routing resolves.
    //   - FrontendReady: set when frontend calls notify_frontend_ready() (listener registered).
    // The emit fires exactly once, whichever side completes last.
    builder = builder.manage(SetupComplete(std::sync::atomic::AtomicBool::new(false)));
    builder = builder.manage(FrontendReady(std::sync::atomic::AtomicBool::new(false)));

    // ParakeetStateMutex starts as None — model is loaded on demand (engine switch)
    // or at startup if saved engine is Parakeet.
    #[cfg(feature = "parakeet")]
    {
        builder = builder.manage(ParakeetStateMutex(std::sync::Mutex::new(None)));
    }

    // MoonshineStateMutex starts as None — model is loaded on demand (engine switch)
    // or at startup if saved engine is Moonshine.
    #[cfg(feature = "moonshine")]
    {
        builder = builder.manage(MoonshineStateMutex(std::sync::Mutex::new(None)));
    }

    builder.invoke_handler(tauri::generate_handler![
            rebind_hotkey,
            unregister_hotkey,
            register_hotkey,
            get_hook_status,
            notify_frontend_ready,
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
            download::download_parakeet_fp32_model,
            download::download_moonshine_tiny_model,
            enable_autostart,
            updater::check_for_update,
            set_update_available,
            is_pipeline_active,
            #[cfg(feature = "whisper")]
            check_first_run,
            #[cfg(feature = "whisper")]
            get_gpu_info,
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

            // Auto-updater plugin — checks GitHub Releases endpoint configured in tauri.conf.json.
            // Must be registered in setup() (needs app handle), same pattern as global-shortcut.
            app.handle().plugin(tauri_plugin_updater::Builder::new().build())?;

            // Configure pill overlay: no focus steal + restore saved position
            if let Some(pill_window) = app.get_webview_window("pill") {
                log::info!("Pill window found — applying configuration");

                // focusable(false) sets WS_EX_NOACTIVATE — pill never steals focus
                let _ = pill_window.set_focusable(false);

                // Disable DWM shadow — rectangular shadow doesn't respect CSS border-radius (tauri#11321)
                let _ = pill_window.set_shadow(false);

                log::info!("Pill overlay window configured (focusable=false, no-shadow)");
            }

            // Determine hotkey to register: use saved setting if present, else default.
            // Default is "ctrl+win" for fresh installs (handled by hook path).
            // Existing users keep their saved hotkey unchanged.
            let hotkey = read_saved_hotkey(app)
                .unwrap_or_else(|| "ctrl+win".to_owned());

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
                use crate::transcribe::ModelMode;
                let cached = app.state::<CachedGpuMode>();
                let gpu_mode = matches!(cached.0, ModelMode::Gpu);
                let saved_engine = read_saved_engine(app, gpu_mode);
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
                        let provider = {
                            let gpu_detection = app.state::<CachedGpuDetection>();
                            gpu_detection.0.parakeet_provider.clone()
                        };
                        match transcribe_parakeet::load_parakeet(&dir_str, &provider) {
                            Ok(p) => {
                                let parakeet_state = app.state::<ParakeetStateMutex>();
                                let inner_arc = std::sync::Arc::new(std::sync::Mutex::new(p));
                                let warmup_arc = inner_arc.clone();
                                {
                                    let mut guard =
                                        parakeet_state.0.lock().unwrap_or_else(|e| e.into_inner());
                                    *guard = Some(inner_arc);
                                }
                                log::info!(
                                    "Parakeet model loaded at startup (variant: {})",
                                    parakeet_model_id
                                );
                                // Warm up in background to avoid blocking UI
                                std::thread::spawn(move || {
                                    let mut guard = warmup_arc.lock().unwrap_or_else(|e| e.into_inner());
                                    transcribe_parakeet::warm_up_parakeet(&mut guard);
                                });
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

            // If saved engine is Moonshine and moonshine feature is enabled, load model at startup.
            #[cfg(feature = "moonshine")]
            {
                let saved_engine = {
                    let engine_state = app.state::<ActiveEngine>();
                    let guard = engine_state.0.lock().unwrap_or_else(|e| e.into_inner());
                    *guard
                };
                if saved_engine == TranscriptionEngine::Moonshine {
                    let model_dir = download::moonshine_tiny_model_dir();
                    if model_dir.exists() {
                        let provider = {
                            #[cfg(feature = "whisper")]
                            {
                                let gpu_detection = app.state::<CachedGpuDetection>();
                                gpu_detection.0.parakeet_provider.clone()
                            }
                            #[cfg(not(feature = "whisper"))]
                            { "cpu".to_string() }
                        };
                        match transcribe_moonshine::load_moonshine(&model_dir, &provider) {
                            Ok(engine_instance) => {
                                let moonshine_state = app.state::<MoonshineStateMutex>();
                                let inner_arc = std::sync::Arc::new(std::sync::Mutex::new(engine_instance));
                                let warmup_arc = inner_arc.clone();
                                {
                                    let mut guard =
                                        moonshine_state.0.lock().unwrap_or_else(|e| e.into_inner());
                                    *guard = Some(inner_arc);
                                }
                                log::info!("Moonshine model loaded at startup");
                                // Warm up in background to avoid blocking UI
                                std::thread::spawn(move || {
                                    let mut guard = warmup_arc.lock().unwrap_or_else(|e| e.into_inner());
                                    transcribe_moonshine::warm_up_moonshine(&mut guard);
                                });
                            }
                            Err(e) => {
                                log::warn!(
                                    "Moonshine startup load failed: {} — falling back to Whisper",
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
                            "Moonshine set as engine but model not found — falling back to Whisper"
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

            // Hook-availability flag — already registered on Builder (before webview creation).
            // Grab the shared Arc so setup() can update it after hook installation.
            let hook_available = app.state::<HookAvailable>().0.clone();

            // Startup hotkey routing — routes based on is_modifier_only predicate:
            // - Modifier-only combo (e.g. "ctrl+win"): attempt WH_KEYBOARD_LL hook installation
            // - Standard combo (e.g. "ctrl+shift+space"): use tauri-plugin-global-shortcut
            // Global-shortcut plugin is always registered (with or without shortcuts) so
            // GlobalShortcutExt methods are available if the user later rebinds to a standard combo.
            {
                // Determine the effective hotkey after possible fallback override
                let effective_hotkey: std::borrow::Cow<str>;

                #[cfg(all(desktop, windows))]
                if is_modifier_only(&hotkey) {
                    // Modifier-only combo: attempt WH_KEYBOARD_LL hook installation
                    match keyboard_hook::install(app.handle().clone()) {
                        Ok(handle) => {
                            hook_available.store(true, std::sync::atomic::Ordering::Relaxed);
                            log::info!("Keyboard hook installed for hotkey: {}", hotkey);
                            let hook_state = app.state::<HookHandleState>();
                            let mut guard = hook_state.0.lock().unwrap_or_else(|e| e.into_inner());
                            *guard = Some(handle);
                            effective_hotkey = std::borrow::Cow::Borrowed("");  // no plugin shortcuts needed
                        }
                        Err(e) => {
                            log::warn!("WH_KEYBOARD_LL install failed: {} — falling back to ctrl+shift+space", e);
                            // hook_available stays false
                            let fallback = "ctrl+shift+space".to_owned();
                            // Persist fallback so frontend displays correct hotkey
                            if let Ok(mut json) = read_settings(app.handle()) {
                                json["hotkey"] = serde_json::Value::String(fallback.clone());
                                let _ = write_settings(app.handle(), &json);
                            }
                            effective_hotkey = std::borrow::Cow::Owned(fallback);
                        }
                    }
                } else {
                    effective_hotkey = std::borrow::Cow::Borrowed(hotkey.as_str());
                }

                #[cfg(all(desktop, not(windows)))]
                {
                    // Non-Windows: modifier-only combos not supported, fall through to standard path
                    effective_hotkey = std::borrow::Cow::Borrowed(hotkey.as_str());
                }

                #[cfg(not(desktop))]
                {
                    // Mobile: no hotkeys
                    effective_hotkey = std::borrow::Cow::Borrowed("");
                }

                // Register global-shortcut plugin.
                // - Hook success path: no shortcuts (plugin registered for runtime rebind availability)
                // - Standard path or fallback path: with hotkey shortcut
                #[cfg(desktop)]
                {
                    if effective_hotkey.is_empty() {
                        // Hook is active — register plugin with no shortcuts (runtime rebind support)
                        app.handle().plugin(
                            tauri_plugin_global_shortcut::Builder::new()
                                .with_handler(|app, _shortcut, event| {
                                    handle_shortcut(app, &event);
                                })
                                .build(),
                        )?;
                    } else {
                        // Standard combo or fallback
                        app.handle().plugin(
                            tauri_plugin_global_shortcut::Builder::new()
                                .with_shortcuts([effective_hotkey.as_ref()])?
                                .with_handler(|app, _shortcut, event| {
                                    handle_shortcut(app, &event);
                                })
                                .build(),
                        )?;
                    }
                }
            }

            // Coordinate hook-status emission with the frontend using a two-flag handshake.
            //
            // Problem: Tauri events are not queued. If setup() emits "hook-status-changed"
            // before the webview's JS listener is registered (listen() round-trip complete),
            // the event is silently dropped (known Tauri limitation, issue #3484).
            //
            // Solution: SetupComplete + FrontendReady flags.
            //   - setup() marks SetupComplete=true here. If frontend already called
            //     notify_frontend_ready (FrontendReady=true), setup() emits immediately.
            //   - If frontend hasn't called notify_frontend_ready yet, setup() does NOT emit;
            //     the notify_frontend_ready command handler will emit when it fires.
            //
            // This guarantees the emit happens exactly once, after BOTH sides are ready.
            {
                use std::sync::atomic::Ordering::Relaxed;
                app.state::<SetupComplete>().0.store(true, Relaxed);
                let frontend_ready = app.state::<FrontendReady>().0.load(Relaxed);
                if frontend_ready {
                    #[cfg(desktop)]
                    {
                        use tauri::Emitter;
                        let hook_ok = hook_available.load(Relaxed);
                        log::debug!("setup: frontend already ready, emitting hook-status-changed={}", hook_ok);
                        if let Some(w) = app.get_webview_window("settings") {
                            w.emit("hook-status-changed", hook_ok).ok();
                        }
                    }
                } else {
                    log::debug!("setup: frontend not yet ready — will emit when notify_frontend_ready fires");
                }
            }

            // HookAvailable registered on Builder — hook_available Arc was cloned
            // from managed state; .store() calls above are visible via app.state().

            // No audio stream at startup — streams are opened on-demand when recording starts.
            // This avoids the Windows microphone privacy indicator appearing when idle.
            app.manage(audio::AudioCaptureMutex(std::sync::Mutex::new(None)));
            log::info!("Audio capture state initialized (on-demand — no stream at startup)");

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
                            // Determine GPU mode based on GPU availability (not model_id)
                            let mode = app.state::<CachedGpuMode>().0.clone();
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
