---
phase: quick-30
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/src/audio.rs
  - src-tauri/src/lib.rs
  - src-tauri/src/pipeline.rs
autonomous: true
requirements: [QUICK-30]
must_haves:
  truths:
    - "Windows microphone privacy indicator (tray icon) only appears while user is actively recording"
    - "Recording start/stop works identically to before (hold-to-talk and toggle modes)"
    - "Saved microphone preference is respected when opening stream on demand"
    - "App launches without opening any audio stream"
    - "Switching microphone in settings just saves preference, no stream opened"
  artifacts:
    - path: "src-tauri/src/audio.rs"
      provides: "On-demand stream open functions"
      contains: "open_stream"
    - path: "src-tauri/src/lib.rs"
      provides: "On-demand stream lifecycle in hotkey handler and set_microphone"
    - path: "src-tauri/src/pipeline.rs"
      provides: "Stream drop after buffer extraction"
  key_links:
    - from: "src-tauri/src/lib.rs (handle_hotkey_event)"
      to: "audio::open_stream / open_stream_with_device"
      via: "resolve device from settings, open stream, store in mutex"
      pattern: "open_stream"
    - from: "src-tauri/src/pipeline.rs (run_pipeline)"
      to: "AudioCaptureMutex"
      via: "set Option to None after buffer extraction to drop stream"
      pattern: "\\*guard = None"
---

<objective>
Switch audio capture from persistent stream (open at startup, always running) to on-demand open/close (open when recording starts, drop when recording ends). This eliminates the Windows microphone privacy indicator icon in the system tray when not recording.

Purpose: Users see a persistent microphone icon in the Windows tray even when not recording, which is confusing and implies always-listening behavior. On-demand capture removes this.
Output: Modified audio.rs, lib.rs, pipeline.rs — stream only exists during active recording.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src-tauri/src/audio.rs
@src-tauri/src/lib.rs
@src-tauri/src/pipeline.rs
@src-tauri/src/vad.rs

<interfaces>
<!-- Current audio.rs public API (to be refactored) -->

```rust
// audio.rs — current public surface
pub struct AudioCapture {
    _stream: cpal::Stream,
    pub recording: Arc<AtomicBool>,
    pub buffer: Arc<Mutex<Vec<f32>>>,
    resampling: Arc<Mutex<ResamplingState>>,
}

pub struct AudioCaptureMutex(pub std::sync::Mutex<Option<AudioCapture>>);

pub fn start_persistent_stream() -> Result<AudioCapture, Box<dyn Error + Send + Sync>>;
pub fn start_persistent_stream_with_device(device: cpal::Device) -> Result<AudioCapture, Box<dyn Error + Send + Sync>>;
pub fn write_wav(path: &str, samples: &[f32]) -> Result<(), Box<dyn Error + Send + Sync>>;

impl AudioCapture {
    pub fn clear_buffer(&self);
    pub fn flush_and_stop(&self) -> usize;
    pub fn get_buffer(&self) -> Vec<f32>;
}
```

<!-- lib.rs — read_saved_mic signature (only takes &App, not &AppHandle) -->
```rust
fn read_saved_mic(app: &tauri::App) -> Option<String> { ... }
// read_settings takes &AppHandle — already usable at runtime
fn read_settings(app_handle: &tauri::AppHandle) -> Result<serde_json::Value, String> { ... }
```

<!-- pipeline.rs — run_pipeline buffer extraction (lines 73-89) -->
```rust
pub async fn run_pipeline(app: tauri::AppHandle) {
    let (sample_count, samples) = {
        let audio_mutex = app.state::<crate::audio::AudioCaptureMutex>();
        let guard = audio_mutex.0.lock().unwrap_or_else(|e| e.into_inner());
        match guard.as_ref() {
            Some(audio) => {
                let count = audio.flush_and_stop();
                let buf = audio.get_buffer();
                (count, buf)
            }
            None => { /* error path */ }
        }
    };
    // ... rest of pipeline
}
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Refactor audio.rs to on-demand API and add device resolution helper</name>
  <files>src-tauri/src/audio.rs</files>
  <action>
Rename the public stream functions from persistent to on-demand semantics:

1. Rename `start_persistent_stream()` to `open_stream()`. Update the doc comment: "Open an on-demand microphone capture stream using the system default device. Stream is active immediately — caller is responsible for dropping the AudioCapture when done to release the microphone." Remove the "persistent" / "runs continuously" language.

2. Rename `start_persistent_stream_with_device(device)` to `open_stream_with_device(device)`. Same doc comment update.

3. Update the log line in `build_stream_from_device` (line 174-178) — change "(persistent)" to "(on-demand)".

4. Add a new public function `resolve_device_by_name(name: &str)` that encapsulates the device-lookup logic currently duplicated in `set_microphone` and the startup handler:

```rust
/// Resolve an audio input device by name.
///
/// If `name` is empty or "System Default", returns the default input device.
/// Otherwise searches available devices by description name.
pub fn resolve_device_by_name(name: &str) -> Result<cpal::Device, Box<dyn std::error::Error + Send + Sync>> {
    use cpal::traits::{DeviceTrait, HostTrait};
    let host = cpal::default_host();
    if name.is_empty() || name == "System Default" {
        host.default_input_device()
            .ok_or_else(|| "No default input device found".into())
    } else {
        host.input_devices()?
            .find(|d| {
                d.description()
                    .map(|desc| desc.name() == name)
                    .unwrap_or(false)
            })
            .ok_or_else(|| format!("Input device '{}' not found", name).into())
    }
}
```

5. Set `recording` to `true` immediately in `build_stream_from_device` (change line 135 from `AtomicBool::new(false)` to `AtomicBool::new(true)`). Since streams are now only opened when recording starts, they should be recording-active immediately. This removes the need for callers to separately set the recording flag after opening the stream. Update the doc comment on `AudioCapture::recording` accordingly.

6. Remove the early-discard check in the audio callback (lines 146-149 `if !recording_cb.load(Ordering::Relaxed) { return; }`). Since the stream only exists during recording, every sample matters. The `recording` flag is still needed for `flush_and_stop()` to signal the callback to stop accumulating, so keep the flag but remove the callback guard.

Wait — actually, keep the `recording` flag check in the callback. Here's why: `flush_and_stop()` sets `recording=false` and then flushes. Between setting false and actually dropping the stream, the callback could still fire. The guard prevents accumulating post-flush samples. So:
- Keep the callback guard `if !recording_cb.load(...) { return; }`
- Keep `AtomicBool::new(false)` (not true) — callers set it to true after clear_buffer
- This preserves the exact same recording flag semantics as before

Actually, re-examining the flow: callers do `clear_buffer()` then `recording.store(true)`. If we set `new(true)` in the constructor, samples would accumulate between stream open and `clear_buffer()`, which is wrong. Keep `new(false)`.

Summary of actual changes to audio.rs:
- Rename `start_persistent_stream` -> `open_stream`
- Rename `start_persistent_stream_with_device` -> `open_stream_with_device`
- Update doc comments and log line to say "on-demand" not "persistent"
- Add `resolve_device_by_name()` function
- No changes to callback logic, AtomicBool init, or buffer handling
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && cargo check -p voice-to-text 2>&1 | tail -5</automated>
  </verify>
  <done>audio.rs exports open_stream(), open_stream_with_device(), and resolve_device_by_name(). No persistent stream language remains. Cargo check passes (will have warnings about unused old names in lib.rs until Task 2).</done>
</task>

<task type="auto">
  <name>Task 2: Update lib.rs, pipeline.rs to open/drop stream on demand</name>
  <files>src-tauri/src/lib.rs, src-tauri/src/pipeline.rs</files>
  <action>
**lib.rs — Startup (lines 1894-1928):**

Remove all stream creation from the setup handler. Replace the entire block (lines 1894-1928) with:

```rust
// No audio stream at startup — streams are opened on-demand when recording starts.
// This avoids the Windows microphone privacy indicator appearing when idle.
app.manage(audio::AudioCaptureMutex(std::sync::Mutex::new(None)));
log::info!("Audio capture state initialized (on-demand — no stream at startup)");
```

Delete the `read_saved_mic()` function entirely (lines 1002-1013). Its logic is replaced by reading `microphone_device` from settings.json via `read_settings()` at recording time.

**lib.rs — `handle_hotkey_event` (lines 405-526):**

In both HoldToTalk and Toggle recording-start branches, replace the pattern:
```rust
let audio_mutex = app.state::<audio::AudioCaptureMutex>();
let guard = audio_mutex.0.lock().unwrap_or_else(|e| e.into_inner());
let audio = match guard.as_ref() { ... };
audio.clear_buffer();
audio.recording.store(true, Ordering::Relaxed);
let buffer_clone = audio.buffer.clone();
drop(guard);
```

With on-demand stream opening:
```rust
// Resolve device from saved preference
let device_name = read_settings(app)
    .ok()
    .and_then(|json| json.get("microphone_device")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty() && *s != "System Default")
        .map(|s| s.to_owned()));

let device = match audio::resolve_device_by_name(device_name.as_deref().unwrap_or("")) {
    Ok(d) => d,
    Err(e) => {
        log::error!("Cannot resolve microphone: {} — cannot record", e);
        pipeline.reset_to_idle();
        return;
    }
};

let capture = match audio::open_stream_with_device(device) {
    Ok(c) => c,
    Err(e) => {
        log::error!("Cannot open audio stream: {} — cannot record", e);
        pipeline.reset_to_idle();
        return;
    }
};

capture.clear_buffer();
capture.recording.store(true, std::sync::atomic::Ordering::Relaxed);
let buffer_clone = capture.buffer.clone();

{
    let audio_mutex = app.state::<audio::AudioCaptureMutex>();
    let mut guard = audio_mutex.0.lock().unwrap_or_else(|e| e.into_inner());
    *guard = Some(capture);
}
```

This pattern appears twice (HoldToTalk and Toggle). Extract it into a helper function to avoid duplication:

```rust
/// Open an on-demand audio stream using the saved microphone preference.
/// Returns the buffer Arc for level streaming, or None if the stream could not be opened.
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
```

Then in each HoldToTalk/Toggle recording-start branch, replace the stream access block with:
```rust
let buffer_clone = match open_recording_stream(app) {
    Some(b) => b,
    None => {
        pipeline.reset_to_idle();
        return;
    }
};
```

The rest of each branch (tray state, pill, level stream, VAD) remains unchanged.

**lib.rs — `set_microphone` (lines 1037-1072):**

Remove the stream creation. The function should only persist the preference:

```rust
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
```

Remove the `use cpal::traits::{DeviceTrait, HostTrait};` import inside set_microphone since device resolution is now in audio.rs.

**lib.rs — `start_recording` command (lines 736-743):**

This command is used by the frontend test UI. Update to open a stream on demand if none exists:

```rust
#[tauri::command]
fn start_recording(app: tauri::AppHandle, state: tauri::State<'_, audio::AudioCaptureMutex>) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| format!("state lock failed: {}", e))?;
    if guard.is_none() {
        // Open stream on demand
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
```

**lib.rs — `stop_recording` command (lines 748-755):**

After flushing, drop the stream:

```rust
#[tauri::command]
fn stop_recording(state: tauri::State<'_, audio::AudioCaptureMutex>) -> Result<usize, String> {
    let mut guard = state.0.lock().map_err(|e| format!("state lock failed: {}", e))?;
    let audio = guard.as_ref().ok_or("No microphone available")?;
    let n = audio.flush_and_stop();
    let seconds = n as f32 / 16000.0;
    log::info!("Recording stopped: {} samples ({:.1}s) — dropping stream", n, seconds);
    // Do NOT drop here — get_buffer still needs the data. Stream will be dropped
    // when the next caller sets *guard = None or when pipeline extracts the buffer.
    Ok(n)
}
```

Actually, `stop_recording` should NOT drop the stream yet because `save_test_wav` may call `get_buffer` afterward. Leave stop_recording as-is for now. The stream drop happens in `run_pipeline` (see below) or can be done by the frontend calling a new cleanup path. For the hotkey flow, `run_pipeline` handles the drop.

**pipeline.rs — `run_pipeline` (lines 73-89):**

After extracting the buffer, drop the AudioCapture to release the microphone:

```rust
let (sample_count, samples) = {
    let audio_mutex = app.state::<crate::audio::AudioCaptureMutex>();
    let mut guard = audio_mutex.0.lock().unwrap_or_else(|e| e.into_inner());
    match guard.as_ref() {
        Some(audio) => {
            let count = audio.flush_and_stop();
            let buf = audio.get_buffer();
            // Drop the stream to release the microphone (removes tray icon)
            *guard = None;
            (count, buf)
        }
        None => {
            log::error!("Pipeline: no microphone available — cannot process");
            app.emit_to("pill", "pill-result", "error").ok();
            reset_to_idle(&app);
            return;
        }
    }
};
```

The key change is `*guard = None;` after `get_buffer()`. This drops the `AudioCapture` struct, which drops the `cpal::Stream`, which releases the microphone.

Change the `guard` binding from immutable to `let mut guard` to allow the assignment.

**lib.rs — `save_test_wav` command:**

This calls `audio.get_buffer()` on the existing stream. Since pipeline drops the stream after extraction, `save_test_wav` would fail if called after pipeline. This is acceptable — `save_test_wav` is a debug/test command used BEFORE pipeline runs. No change needed.
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && cargo build -p voice-to-text 2>&1 | tail -10</automated>
  </verify>
  <done>
    - App starts with no audio stream (AudioCaptureMutex contains None)
    - handle_hotkey_event opens stream on recording start via open_recording_stream() helper
    - run_pipeline sets AudioCapture to None after extracting buffer, dropping stream
    - set_microphone only saves preference to settings.json, no stream manipulation
    - start_recording command opens stream on demand if none exists
    - Cargo build succeeds with no errors
  </done>
</task>

</tasks>

<verification>
1. `cargo build -p voice-to-text` succeeds with no errors
2. Launch app — no microphone icon in Windows tray on startup
3. Press hotkey to record — microphone icon appears in tray
4. Release hotkey — after transcription completes, microphone icon disappears from tray
5. Switch microphone in settings — no stream opened, preference saved
6. Record again — new microphone preference used
7. Toggle mode: start recording (icon appears), stop recording (icon disappears after pipeline)
</verification>

<success_criteria>
- Windows microphone privacy indicator only visible during active recording
- All recording modes (hold-to-talk, toggle) work correctly
- Saved microphone preference respected on each recording start
- No regression in transcription quality or latency
- App startup does not touch any audio device
</success_criteria>

<output>
After completion, create `.planning/quick/30-switch-audio-capture-from-persistent-str/30-SUMMARY.md`
</output>
