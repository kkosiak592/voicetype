---
phase: quick-40
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/src/lib.rs
  - src-tauri/src/audio.rs
  - src-tauri/src/pipeline.rs
  - src/components/sections/GeneralSection.tsx
  - src/components/AlwaysListenToggle.tsx
autonomous: true
requirements: [QUICK-40]

must_haves:
  truths:
    - "When always-listen is enabled, hotkey activation has zero mic-init latency"
    - "When always-listen is disabled, behavior is identical to current on-demand flow"
    - "Always-listen setting persists across app restarts"
    - "Windows microphone privacy indicator stays visible while always-listen is active"
    - "User can toggle always-listen on/off from General Settings"
  artifacts:
    - path: "src-tauri/src/audio.rs"
      provides: "Persistent stream management with always-listen flag"
    - path: "src-tauri/src/lib.rs"
      provides: "AlwaysListenActive state, get/set IPC commands, modified hotkey handler"
    - path: "src/components/AlwaysListenToggle.tsx"
      provides: "Toggle switch UI component"
    - path: "src/components/sections/GeneralSection.tsx"
      provides: "Always-listen toggle wired into settings page"
  key_links:
    - from: "src-tauri/src/lib.rs (handle_hotkey_event)"
      to: "audio::AudioCaptureMutex"
      via: "Skip open_recording_stream when always-listen stream already exists"
      pattern: "AlwaysListenActive.*load"
    - from: "src/components/AlwaysListenToggle.tsx"
      to: "src-tauri/src/lib.rs"
      via: "invoke('set_always_listen') / invoke('get_always_listen')"
---

<objective>
Add an "always listen" mode where the microphone stream stays open continuously, eliminating the mic initialization latency (~100-300ms) on hotkey activation.

Purpose: The current on-demand mic flow (open stream -> start recording) adds noticeable latency every time the user activates recording. Always-listen keeps the cpal stream alive and discarding samples until recording begins, so audio capture starts instantly.

Output: Backend always-listen state + persistent stream management, frontend toggle in General Settings, modified hotkey handler that reuses existing stream.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src-tauri/src/audio.rs (AudioCapture, AudioCaptureMutex, open_stream_with_device, build_stream_from_device)
@src-tauri/src/lib.rs (handle_hotkey_event, open_recording_stream, RecordingMode, Mode, get_all_caps/set_all_caps pattern)
@src-tauri/src/pipeline.rs (run_pipeline — drops stream after processing via `*guard = None`)
@src/components/AllCapsToggle.tsx (reference pattern for toggle component)
@src/components/sections/GeneralSection.tsx (where toggle gets added)

<interfaces>
<!-- From src-tauri/src/audio.rs -->
```rust
pub struct AudioCapture {
    _stream: SendStream,
    pub recording: Arc<AtomicBool>,
    pub buffer: Arc<Mutex<Vec<f32>>>,
    resampling: Arc<Mutex<ResamplingState>>,
}

pub struct AudioCaptureMutex(pub std::sync::Mutex<Option<AudioCapture>>);

pub fn open_stream_with_device(device: cpal::Device) -> Result<AudioCapture, ...>;
pub fn resolve_device_by_name(name: &str) -> Result<cpal::Device, ...>;
```

<!-- From src-tauri/src/lib.rs -->
```rust
pub(crate) fn handle_hotkey_event(app: &tauri::AppHandle, pressed: bool);
fn open_recording_stream(app: &tauri::AppHandle) -> Option<Arc<Mutex<Vec<f32>>>>;

// Pattern for managed state + settings toggle (from all_caps):
#[tauri::command]
fn get_all_caps(app: tauri::AppHandle) -> Result<bool, String>;
#[tauri::command]
fn set_all_caps(app: tauri::AppHandle, enabled: bool) -> Result<(), String>;
```

<!-- From src-tauri/src/pipeline.rs line 73-91 — run_pipeline drops the stream -->
```rust
// In run_pipeline():
let (sample_count, samples) = {
    let audio_mutex = app.state::<crate::audio::AudioCaptureMutex>();
    let mut guard = audio_mutex.0.lock().unwrap_or_else(|e| e.into_inner());
    match guard.as_ref() {
        Some(audio) => {
            let count = audio.flush_and_stop();
            let buf = audio.get_buffer();
            *guard = None; // <-- drops stream, releases mic
            (count, buf)
        }
        // ...
    }
};
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Backend always-listen state, IPC commands, persistent stream, and hotkey handler integration</name>
  <files>src-tauri/src/lib.rs, src-tauri/src/audio.rs, src-tauri/src/pipeline.rs</files>
  <action>
**1. Add AlwaysListenActive managed state in lib.rs:**

Create a new managed state struct (following the RecordingMode/AtomicBool pattern):

```rust
pub struct AlwaysListenActive(pub std::sync::atomic::AtomicBool);
```

Register in `setup()` after AudioCaptureMutex initialization. Load initial value from settings.json key `always_listen` (default false). If true on startup, call `open_recording_stream_persistent(app)` (see below) to pre-open the mic.

**2. Add get/set IPC commands (following get_all_caps/set_all_caps pattern):**

```rust
#[tauri::command]
fn get_always_listen(app: tauri::AppHandle) -> Result<bool, String> {
    let state = app.state::<AlwaysListenActive>();
    Ok(state.0.load(Ordering::Relaxed))
}

#[tauri::command]
fn set_always_listen(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let state = app.state::<AlwaysListenActive>();
    state.0.store(enabled, Ordering::Relaxed);

    // Persist to settings.json
    let mut json = read_settings(&app)?;
    json["always_listen"] = serde_json::Value::Bool(enabled);
    write_settings(&app, &json)?;

    if enabled {
        // Open persistent stream (mic stays open, discarding samples until recording)
        open_always_listen_stream(&app);
    } else {
        // Drop the stream to release the mic (only if not currently recording)
        let pipeline = app.state::<pipeline::PipelineState>();
        if pipeline.current() == pipeline::Phase::Idle {
            let audio_mutex = app.state::<audio::AudioCaptureMutex>();
            let mut guard = audio_mutex.0.lock().unwrap_or_else(|e| e.into_inner());
            *guard = None;
        }
    }

    log::info!("Always-listen set to {}", enabled);
    Ok(())
}
```

Register both commands in the `invoke_handler` list.

**3. Add `open_always_listen_stream` helper in lib.rs:**

Similar to `open_recording_stream` but does NOT set `recording=true` — the stream stays open and discards samples (the cpal callback already has `if !recording_cb.load(Relaxed) { return; }`). This means the mic is hot but no audio is buffered.

```rust
fn open_always_listen_stream(app: &tauri::AppHandle) {
    let audio_mutex = app.state::<audio::AudioCaptureMutex>();
    let mut guard = audio_mutex.0.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_some() {
        return; // Stream already open
    }

    let device_name = read_settings(app)
        .ok()
        .and_then(|json| json.get("microphone_device")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty() && *s != "System Default")
            .map(|s| s.to_owned()));

    let device = match audio::resolve_device_by_name(device_name.as_deref().unwrap_or("")) {
        Ok(d) => d,
        Err(e) => {
            log::error!("Always-listen: cannot resolve microphone: {}", e);
            return;
        }
    };

    match audio::open_stream_with_device(device) {
        Ok(capture) => {
            // recording stays false — stream discards samples until hotkey activates
            *guard = Some(capture);
            log::info!("Always-listen: persistent stream opened (mic hot, not recording)");
        }
        Err(e) => {
            log::error!("Always-listen: cannot open stream: {}", e);
        }
    }
}
```

**4. Modify `open_recording_stream` to reuse existing always-listen stream:**

At the top of `open_recording_stream`, check if AudioCaptureMutex already has a stream (from always-listen). If so, reuse it instead of opening a new one:

```rust
fn open_recording_stream(app: &tauri::AppHandle) -> Option<Arc<std::sync::Mutex<Vec<f32>>>> {
    let audio_mutex = app.state::<audio::AudioCaptureMutex>();
    let mut guard = audio_mutex.0.lock().unwrap_or_else(|e| e.into_inner());

    // Reuse existing always-listen stream if available
    if let Some(ref capture) = *guard {
        capture.clear_buffer();
        capture.recording.store(true, std::sync::atomic::Ordering::Relaxed);
        return Some(capture.buffer.clone());
    }

    // No existing stream — open on-demand (original flow)
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
    *guard = Some(capture);
    Some(buffer_clone)
}
```

Note: the guard is already acquired at the top, so the existing code that acquires it separately must be restructured to avoid double-lock.

**5. Modify `run_pipeline` in pipeline.rs to NOT drop the stream when always-listen is active:**

Change the stream cleanup in `run_pipeline()` (lines 73-91). Instead of unconditionally setting `*guard = None`, check always-listen state:

```rust
let (sample_count, samples) = {
    let audio_mutex = app.state::<crate::audio::AudioCaptureMutex>();
    let mut guard = audio_mutex.0.lock().unwrap_or_else(|e| e.into_inner());
    match guard.as_mut() {
        Some(audio) => {
            let count = audio.flush_and_stop();
            let buf = audio.get_buffer();
            // Only drop the stream if always-listen is OFF
            let always_listen = app.state::<crate::AlwaysListenActive>();
            if !always_listen.0.load(std::sync::atomic::Ordering::Relaxed) {
                *guard = None; // Release mic
            }
            // else: keep stream open for next activation
            (count, buf)
        }
        None => {
            // ... existing error handling
        }
    }
};
```

**6. Re-open always-listen stream on mic device change:**

Find the existing `set_microphone_device` or equivalent command. After it changes the device, if always-listen is active, drop and re-open the stream with the new device. If no such command exists, skip this — the user would need to toggle always-listen off/on after changing mic.
  </action>
  <verify>
    <automated>cd src-tauri && cargo check 2>&1 | tail -5</automated>
  </verify>
  <done>
    - AlwaysListenActive managed state registered and loaded from settings
    - get_always_listen / set_always_listen IPC commands work
    - When always-listen enabled: stream stays open, hotkey reuses it (no mic init delay)
    - When always-listen disabled: original on-demand flow unchanged
    - run_pipeline preserves stream when always-listen is active
    - Setting persists to settings.json key `always_listen`
  </done>
</task>

<task type="auto">
  <name>Task 2: Frontend always-listen toggle in General Settings</name>
  <files>src/components/AlwaysListenToggle.tsx, src/components/sections/GeneralSection.tsx</files>
  <action>
**1. Create `src/components/AlwaysListenToggle.tsx`:**

Clone the AllCapsToggle pattern exactly — same switch styling, same invoke pattern:

```tsx
import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';

export function AlwaysListenToggle() {
  const [enabled, setEnabled] = useState(false);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    invoke<boolean>('get_always_listen').then((val) => {
      setEnabled(val);
      setLoading(false);
    }).catch(err => {
      console.error('Failed to check always_listen:', err);
      setLoading(false);
    });
  }, []);

  async function handleToggle() {
    const next = !enabled;
    await invoke('set_always_listen', { enabled: next });
    setEnabled(next);
  }

  if (loading) {
    return (
      <div className="h-6 w-11 animate-pulse rounded-full bg-gray-200 dark:bg-gray-600" />
    );
  }

  return (
    <button
      onClick={handleToggle}
      className={[
        'relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:ring-offset-2 dark:focus:ring-offset-gray-900',
        enabled ? 'bg-emerald-500' : 'bg-gray-200 dark:bg-gray-700',
      ].join(' ')}
      role="switch"
      aria-checked={enabled}
    >
      <span className="sr-only">Toggle always listen</span>
      <span
        aria-hidden="true"
        className={[
          'pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out',
          enabled ? 'translate-x-5' : 'translate-x-0',
        ].join(' ')}
      />
    </button>
  );
}
```

**2. Add to GeneralSection.tsx:**

Import AlwaysListenToggle. Add a new row inside Card 1 (Activation card), below the Recording Mode section, separated by a divider. Follow the same layout pattern used for ALL CAPS in Card 2:

```tsx
<div className="my-5 border-t border-gray-100 dark:border-gray-800" />

<section>
  <div className="flex items-center justify-between">
    <div>
      <p className="text-sm font-medium text-gray-900 dark:text-gray-100">Always Listen</p>
      <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
        Keep microphone open to eliminate activation delay. Uses more resources.
      </p>
    </div>
    <AlwaysListenToggle />
  </div>
</section>
```

Place it inside Card 1 after the Recording Mode section, since it's related to activation behavior (not output).
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && npx tsc --noEmit 2>&1 | tail -5</automated>
  </verify>
  <done>
    - AlwaysListenToggle component renders a switch matching AllCapsToggle styling
    - Toggle appears in General Settings under Card 1 (Activation) below Recording Mode
    - Toggle calls get_always_listen on mount and set_always_listen on click
    - Description warns about resource usage
  </done>
</task>

</tasks>

<verification>
1. `cargo check` passes in src-tauri (no compile errors)
2. `npx tsc --noEmit` passes (no TypeScript errors)
3. Manual test: Launch app, open Settings > General, verify "Always Listen" toggle appears
4. Manual test: Enable always-listen, observe Windows mic indicator appears immediately
5. Manual test: With always-listen on, press hotkey — recording starts with no perceptible delay
6. Manual test: Disable always-listen, observe Windows mic indicator disappears (when idle)
7. Manual test: Close and reopen app — always-listen state persists from settings.json
</verification>

<success_criteria>
- Hotkey-to-recording latency is effectively zero when always-listen is enabled (mic stream already open)
- When always-listen is disabled, behavior is identical to current on-demand flow
- Setting persists across app restarts via settings.json
- Frontend toggle is visible in General Settings with resource usage warning
- Windows microphone privacy indicator correctly reflects stream state
</success_criteria>

<output>
After completion, create `.planning/quick/40-add-always-listen-mode-to-reduce-activat/40-SUMMARY.md`
</output>
