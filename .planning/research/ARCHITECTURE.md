# Architecture Research

**Domain:** Local voice-to-text desktop tool (Tauri 2.0 + Rust + React)
**Researched:** 2026-02-27
**Confidence:** HIGH — Stack validated by BridgeVoice in production; threading patterns verified against official Tauri and cpal documentation

## Standard Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        React Frontend (WebView2)                 │
│  ┌───────────┐  ┌─────────────────┐  ┌──────────────────────┐   │
│  │ Pill.tsx  │  │  Settings.tsx   │  │  CorrectionEditor.tsx│   │
│  │ (overlay) │  │  (config panel) │  │  (dictionary editor) │   │
│  └─────┬─────┘  └────────┬────────┘  └──────────┬───────────┘   │
│        │                 │                      │               │
│        └─────────────────┴──────────────────────┘               │
│                    Tauri invoke() / listen()                     │
└──────────────────────────┬──────────────────────────────────────┘
                           │  IPC: JSON-RPC commands + events
┌──────────────────────────▼──────────────────────────────────────┐
│                     Rust Backend (main.rs)                       │
│  ┌──────────────────────────────────────────────────────────┐    │
│  │                   AppState (Arc<Mutex<T>>)                │    │
│  │  recording: bool | audio_buf: Vec<f32> | profile: Profile │    │
│  └──────────────────────────────────────────────────────────┘    │
│                                                                  │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────────────┐   │
│  │ hotkeys.rs  │  │  audio.rs    │  │    transcribe.rs       │   │
│  │ (global-    │  │  (cpal WASAPI│  │    (whisper-rs FFI)    │   │
│  │  shortcut)  │  │   callbacks) │  │    spawn_blocking()    │   │
│  └──────┬──────┘  └──────┬───────┘  └──────────┬────────────┘   │
│         │                │                     │                │
│  ┌──────▼──────┐  ┌──────▼───────┐  ┌──────────▼────────────┐   │
│  │  Audio      │  │  VAD Thread  │  │   Post-processor       │   │
│  │  Thread     │  │  (silero-vad │  │   corrections.rs       │   │
│  │  (cpal, OS- │  │   30ms chunks│  │   (find/replace dict)  │   │
│  │  managed)   │  │   via mpsc)  │  └──────────┬────────────┘   │
│  └─────────────┘  └──────────────┘             │                │
│                                       ┌─────────▼────────────┐   │
│                                       │    injector.rs        │   │
│                                       │    (enigo + Win32     │   │
│                                       │     clipboard paste)  │   │
│                                       └──────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Typical Implementation |
|-----------|----------------|------------------------|
| Pill.tsx | Floating recording indicator, frequency bars visualizer | React, always-on-top transparent window |
| Settings.tsx | Hotkey config, profile selection, model picker | React form, tauri-plugin-store reads/writes |
| CorrectionEditor.tsx | User-editable find/replace dictionary | React, reads JSON correction file |
| hotkeys.rs | Register and dispatch global shortcuts system-wide | tauri-plugin-global-shortcut, ShortcutState::Pressed/Released |
| audio.rs | Capture mic at 16kHz via WASAPI, maintain sample buffer | cpal stream, OS-managed audio thread, mpsc sender |
| vad.rs | Detect speech start/end in 30ms chunks | silero-vad-rust crate (ships ONNX model), <1ms per chunk |
| transcribe.rs | Run whisper.cpp inference on completed audio buffer | whisper-rs, tokio::task::spawn_blocking (CPU-bound) |
| corrections.rs | Apply find/replace corrections after transcription | JSON/TOML dictionary, regex patterns, case normalization |
| injector.rs | Write text into active application | enigo + Win32 clipboard API, clipboard save/restore |
| AppState | Single source of truth for runtime state | Arc<Mutex<AppState>> managed by Tauri, accessed in commands |

## Recommended Project Structure

```
voice-to-text/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs           # App entry, Tauri builder, command registration
│   │   ├── audio.rs          # cpal audio capture, 16kHz buffer accumulation
│   │   ├── transcribe.rs     # whisper-rs inference, model loading, GPU/CPU detection
│   │   ├── vad.rs            # Silero VAD via silero-vad-rust crate
│   │   ├── injector.rs       # Clipboard + enigo text injection, save/restore
│   │   ├── corrections.rs    # Word correction dictionary, post-processing pipeline
│   │   ├── hotkeys.rs        # Global shortcut registration and event routing
│   │   ├── profiles.rs       # Vocabulary profile loading, initial prompt selection
│   │   └── state.rs          # AppState struct definition, default values
│   ├── Cargo.toml
│   └── tauri.conf.json       # Window config: transparent, always-on-top, frameless
├── src/
│   ├── App.tsx               # React root, window routing (pill vs settings)
│   ├── components/
│   │   ├── Pill.tsx          # Floating indicator, frequency bars
│   │   ├── Settings.tsx      # Full settings panel
│   │   └── CorrectionEditor.tsx  # Dictionary CRUD UI
│   ├── stores/
│   │   └── settings.ts       # Zustand or useState for UI state
│   └── hooks/
│       └── useRecordingState.ts  # Tauri event listener hook
├── models/                   # Downloaded whisper GGML files (gitignored)
├── corrections/              # User correction dictionaries (JSON)
│   ├── general.json
│   └── structural-engineering.json
└── scripts/
    └── download-models.sh    # First-run model download helper
```

### Structure Rationale

- **src-tauri/src/:** One file per component boundary — matches BridgeVoice pattern, makes boundaries explicit
- **models/:** Gitignored, downloaded on first run; NSIS installer fails above 2GB so models must stay separate
- **corrections/:** User-writable JSON files, not embedded in binary so users can edit without rebuilding
- **src/hooks/:** Isolates Tauri event subscriptions from React render logic

## Architectural Patterns

### Pattern 1: OS Audio Thread → mpsc Channel → Processing Thread

**What:** cpal's audio callback runs on an OS-managed, high-priority audio thread. Audio samples are sent through a `std::sync::mpsc` channel to a separate processing thread rather than doing any work inside the callback.

**When to use:** Always — the audio callback must return in microseconds or WASAPI will buffer underrun. Never lock a mutex or do I/O inside the callback.

**Trade-offs:** Adds one channel hop but is mandatory; any blocking in the audio callback causes audible glitches.

**Example:**
```rust
// audio.rs
let (tx, rx) = std::sync::mpsc::channel::<Vec<f32>>();

let stream = device.build_input_stream(
    &config,
    move |data: &[f32], _| {
        // Callback — must be instant. Only send, never block.
        let _ = tx.send(data.to_vec());
    },
    |err| eprintln!("audio error: {err}"),
    None,
)?;

// Separate thread reads from rx and accumulates buffer
std::thread::spawn(move || {
    let mut buffer: Vec<f32> = Vec::new();
    while let Ok(chunk) = rx.recv() {
        buffer.extend_from_slice(&chunk);
        // Pass chunk to VAD on each 30ms frame
    }
});
```

### Pattern 2: spawn_blocking for whisper-rs Inference

**What:** whisper.cpp inference is a synchronous, CPU-bound blocking call that takes 300–500ms. Wrap it in `tokio::task::spawn_blocking` so it doesn't block the async Tauri command executor.

**When to use:** Whenever calling whisper-rs `full()` method — it will block the calling thread for hundreds of milliseconds.

**Trade-offs:** Adds thread-pool overhead (~1ms) but is necessary; blocking an async command executor stalls all other Tauri commands.

**Example:**
```rust
// transcribe.rs
#[tauri::command]
async fn transcribe(audio: Vec<f32>, state: State<'_, AppState>) -> Result<String, String> {
    let ctx = state.whisper_ctx.clone(); // Arc<WhisperContext>
    tokio::task::spawn_blocking(move || {
        let mut s = ctx.create_state().map_err(|e| e.to_string())?;
        let params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        s.full(params, &audio).map_err(|e| e.to_string())?;
        let text = (0..s.full_n_segments()?)
            .map(|i| s.full_get_segment_text(i))
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.join(" "))
            .map_err(|e| e.to_string())?;
        Ok(text)
    })
    .await
    .map_err(|e| e.to_string())?
}
```

### Pattern 3: Tauri Events for Real-Time UI State

**What:** Use `app_handle.emit()` for one-way Rust → frontend data pushes (recording state, audio level, transcription result). Use Tauri commands (invoke) for frontend → Rust requests (start recording, change profile).

**When to use:** Any time the backend needs to push state the frontend didn't explicitly request — audio meter levels, recording started/stopped, transcription complete.

**Trade-offs:** Events are fire-and-forget (no return value, not type-safe). Use channels instead of events for high-frequency data (>10Hz) like audio waveform samples.

**Example:**
```rust
// hotkeys.rs — emit state change when hotkey pressed
app_handle.emit("recording-state-changed", RecordingPayload {
    recording: true,
    mode: "hold-to-talk",
})?;

// Pill.tsx — listen for state change
useEffect(() => {
    const unlisten = listen<RecordingPayload>("recording-state-changed", (event) => {
        setRecording(event.payload.recording);
    });
    return () => { unlisten.then(f => f()); };
}, []);
```

### Pattern 4: Tauri Managed State with Arc<Mutex<T>>

**What:** Register a single `AppState` struct at app startup using `app.manage()`. All Tauri commands receive it via `State<'_, AppState>` injection. For threads spawned outside the command system, clone an `AppHandle` to access state via `app_handle.state::<AppState>()`.

**When to use:** All shared mutable state — recording flag, audio buffer, active profile, whisper context.

**Trade-offs:** Standard `std::sync::Mutex` is sufficient (no async mutex needed) as long as you don't hold the lock across await points.

**Example:**
```rust
// state.rs
pub struct AppState {
    pub recording: Mutex<bool>,
    pub audio_buffer: Mutex<Vec<f32>>,
    pub active_profile: Mutex<Profile>,
    pub whisper_ctx: Arc<WhisperContext>,  // Expensive init, share without re-locking
}

// main.rs
tauri::Builder::default()
    .setup(|app| {
        let ctx = load_whisper_model()?;
        app.manage(AppState {
            recording: Mutex::new(false),
            audio_buffer: Mutex::new(Vec::new()),
            active_profile: Mutex::new(Profile::default()),
            whisper_ctx: Arc::new(ctx),
        });
        Ok(())
    })
```

## Data Flow

### Hold-to-Talk Flow

```
[Hotkey Press (ShortcutState::Pressed)]
    ↓
hotkeys.rs sets AppState.recording = true
    ↓
hotkeys.rs emits "recording-state-changed" → Pill.tsx shows active state
    ↓
audio.rs cpal stream starts (or was already running — stream stays open)
    ↓
Audio samples accumulate in AppState.audio_buffer via mpsc channel
    ↓
[Hotkey Release (ShortcutState::Released)]
    ↓
hotkeys.rs sets AppState.recording = false, takes audio_buffer snapshot
    ↓
hotkeys.rs invokes transcribe(audio_snapshot) via spawn_blocking
    ↓
whisper-rs runs inference (~300-500ms GPU / ~2-4s CPU)
    ↓
transcribe.rs returns raw text string
    ↓
corrections.rs applies find/replace dictionary
    ↓
injector.rs: save clipboard → write text → simulate Ctrl+V → restore clipboard (after 50-100ms)
    ↓
Text appears at cursor in active application
    ↓
app_handle.emit("transcription-complete", { text }) → Pill.tsx shows brief confirmation
```

### Toggle Mode Flow (VAD-Driven)

```
[Hotkey Press (toggle on)]
    ↓
hotkeys.rs sets recording = true, starts VAD pipeline
    ↓
VAD thread receives 30ms audio chunks from mpsc
    ↓
silero-vad-rust processes each chunk (<1ms per chunk)
    ↓ (while speech detected)
Audio accumulates in buffer
    ↓ (VAD detects silence > threshold)
VAD triggers transcription of completed speech segment
    ↓
Same transcribe → corrections → inject pipeline as hold-to-talk
    ↓
Buffer reset, VAD continues listening
    ↓
[Hotkey Press again (toggle off)] → recording = false, VAD stops
```

### Settings Flow

```
[User opens Settings]
    ↓
React calls invoke("get_settings") → returns JSON from tauri-plugin-store
    ↓
Settings.tsx renders form with current values
    ↓
[User changes profile]
    ↓
React calls invoke("set_profile", { profile: "structural-engineering" })
    ↓
Rust loads corrections/structural-engineering.json
    ↓
AppState.active_profile updated
    ↓
Next transcription uses new profile's initial_prompt + corrections dict
```

### State Management

```
AppState (single Arc<Mutex<T>> per field)
    ↓ (inject via State<'_>)
Tauri Commands ←→ Frontend (invoke/listen)
    ↑
AppHandle.state::<AppState>()  ← Background threads (audio, VAD)
```

## Scaling Considerations

This is a single-user local desktop app. Traditional "scaling" doesn't apply. Instead, the relevant concerns are:

| Concern | Approach |
|---------|----------|
| Long audio sessions (5-30 min) | Cap audio buffer at ~30s (whisper accuracy degrades beyond this); auto-segment on VAD silence |
| Many correction rules (100+ entries) | Keep as HashMap<String, String> in memory — negligible cost |
| Multiple simultaneous windows | Pill window (transparent overlay) + Settings window are separate Tauri windows, not separate processes |
| First-run model download | Progress events via Tauri channel, not polling |
| Multiple users on same machine | Profile config in per-user AppData, not global registry |

## Anti-Patterns

### Anti-Pattern 1: Blocking Inside the cpal Audio Callback

**What people do:** Lock a mutex, write to a file, or call `whisper-rs` directly inside the cpal data callback closure.

**Why it's wrong:** The audio callback runs on an OS-managed high-priority thread. Any blocking causes WASAPI buffer underruns, producing audible clicks and glitches. The callback must return in microseconds.

**Do this instead:** Only send audio samples through an `mpsc::Sender` inside the callback. All processing happens on a separate thread that reads from the channel receiver.

### Anti-Pattern 2: Running whisper-rs Synchronously on the Async Executor

**What people do:** Call `whisper_state.full(params, &audio)` directly inside an `async fn` Tauri command without `spawn_blocking`.

**Why it's wrong:** whisper inference blocks for 300ms–4s. Blocking the tokio async executor thread stalls all other Tauri commands and IPC during that window — the UI freezes.

**Do this instead:** Always wrap whisper-rs `full()` calls in `tokio::task::spawn_blocking`. The inference runs on the blocking thread pool, the async executor stays responsive.

### Anti-Pattern 3: Bundling Model Files in the Installer

**What people do:** Include whisper GGML model files (500MB–3GB) in the Tauri NSIS/WiX installer bundle.

**Why it's wrong:** NSIS installer generation fails for bundles exceeding 2GB. Even below 2GB, a 500MB installer is hostile UX, and models need updating independently of the app.

**Do this instead:** Distribute a small app installer (~2.5MB). On first run, detect missing model and download it from a CDN (Hugging Face works). Show progress in the Pill overlay. Store models in `%APPDATA%\voicetype\models\`.

### Anti-Pattern 4: Using a Single Transparent Window for Both Pill and Settings

**What people do:** Toggle the pill overlay to show a settings panel within the same window by changing CSS/React state.

**Why it's wrong:** The pill window is `transparent: true`, `always_on_top: true`, `decorations: false`, `skip_taskbar: true`. A settings panel needs normal window decorations, taskbar presence, and is not always-on-top. Mixing these in one window requires hacky resize/restyle logic.

**Do this instead:** Two separate Tauri windows. The pill window has overlay properties. A separate settings window with standard chrome opens on demand. Communicate between them via `app_handle.emit_to()` targeting each window by label.

### Anti-Pattern 5: Clipboard Race Condition on Text Injection

**What people do:** Write to clipboard and immediately simulate Ctrl+V without delay.

**Why it's wrong:** Some applications (Chrome, Electron apps) have not yet processed the clipboard write by the time the paste keystroke arrives. The old clipboard contents get pasted instead.

**Do this instead:** After writing to clipboard, call `GetOpenClipboardWindow` to verify no other app owns the clipboard, then wait 50–100ms before simulating Ctrl+V. After injection, restore original clipboard after another 100ms delay.

## Integration Points

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| audio.rs → vad.rs | `mpsc::Sender<Vec<f32>>` | 30ms chunks forwarded per frame; VAD runs on receiving thread |
| audio.rs → transcribe.rs | `Vec<f32>` snapshot passed at recording end | Full buffer taken atomically when hotkey released |
| hotkeys.rs → audio.rs | `AppState.recording: Mutex<bool>` flag | Audio thread checks flag; cpal stream stays open always |
| transcribe.rs → corrections.rs | `String` (raw transcript) | Synchronous function call, no threading needed |
| corrections.rs → injector.rs | `String` (corrected text) | Synchronous function call |
| Rust backend → React frontend | `app_handle.emit()` events | recording-state-changed, transcription-complete, audio-level |
| React frontend → Rust backend | `invoke()` commands | start_recording, stop_recording, set_profile, get_settings, save_settings |
| transcribe.rs → Tauri async executor | `tokio::task::spawn_blocking` | Prevents inference blocking IPC |
| Settings window ↔ Pill window | `app_handle.emit_to("pill", ...)` | Cross-window communication via Tauri event system |

### External Inputs

| Input | Source | Notes |
|-------|--------|-------|
| Whisper GGML model | Hugging Face CDN (first run) | ggml-large-v3-turbo.bin (~1.5GB) or ggml-small.bin (~466MB) |
| Silero VAD ONNX model | Bundled in silero-vad-rust crate | 1.8MB, MIT licensed, ships with the crate |
| System microphone | cpal → WASAPI | Default input device; no device selection in v1 |
| Active application clipboard | Win32 OpenClipboard/GetClipboardData | Saved before injection, restored after |

## Build Order Implications

The architecture has a clear dependency chain. Build in this order to validate each layer before adding the next:

1. **Tauri scaffold + global hotkey** — proves framework wires up; hotkey prints to console. No audio yet. Validates Tauri 2.0 setup, window creation, and IPC.

2. **Audio capture (cpal)** — proves mic input at 16kHz with correct threading. Validates WASAPI access and the mpsc channel pattern. Record to WAV and play back to verify.

3. **Whisper integration (whisper-rs)** — prove transcription works against a test WAV file before connecting audio. Validates CUDA build flags, model loading, and spawn_blocking pattern.

4. **End-to-end pipeline (hotkey → audio → whisper → console)** — wire steps 1–3 together. No UI, no injection. Validates the full data flow timing (target: <500ms after hotkey release).

5. **Text injection (injector.rs)** — clipboard paste into Notepad first, then test in Chrome/VSCode/terminal. Validates enigo + Win32 clipboard with the save/restore race condition mitigation.

6. **Pill overlay UI** — floating transparent window with recording state. Validates Tauri transparent window workaround (Issue #13270). Add frequency bar visualizer.

7. **Silero VAD (toggle mode)** — add 30ms chunk processing on the VAD thread. Validates silero-vad-rust integration and silence detection threshold tuning.

8. **Corrections + Profiles system** — post-processing dictionary, initial prompt selection, profile switching. No new infrastructure needed, pure Rust logic.

9. **Settings panel** — separate Tauri window with full config UI. Validates two-window architecture and cross-window events.

10. **System tray** — background presence, context menu. Validates tauri-plugin-tray-icon and app lifecycle (hide to tray on close).

11. **Model download + first-run UX** — download progress events, model selection. Validates channel-based progress reporting.

12. **NSIS packaging** — end-to-end installer. Validates that models are excluded from bundle and the 2.5MB installer works on a clean Windows machine.

## Sources

- [Tauri 2.0 — Calling Rust from Frontend](https://v2.tauri.app/develop/calling-rust/) — HIGH confidence, official docs
- [Tauri 2.0 — Calling Frontend from Rust](https://v2.tauri.app/develop/calling-frontend/) — HIGH confidence, official docs
- [Tauri 2.0 — State Management](https://v2.tauri.app/develop/state-management/) — HIGH confidence, official docs
- [cpal — Audio Input and Processing (DeepWiki)](https://deepwiki.com/RustAudio/cpal/5.2-audio-input-and-processing) — MEDIUM confidence, verified against cpal docs.rs
- [cpal — docs.rs](https://docs.rs/cpal) — HIGH confidence, official crate documentation
- [Tokio — spawn_blocking](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html) — HIGH confidence, official docs
- [silero-vad-rust crate](https://crates.io/crates/silero-vad-rust) — MEDIUM confidence, community crate shipping bundled ONNX weights
- [Tauri transparent window issue #13270](https://github.com/tauri-apps/tauri/issues/13270) — HIGH confidence, official GitHub issue with workaround
- [BridgeVoice — primary reference implementation](https://docs.bridgemind.ai/docs/bridgevoice) — MEDIUM confidence (closed source, architectural details inferred from documentation)
- [Keyless reference project](https://github.com/hate/keyless) — MEDIUM confidence, open-source Tauri v2 voice-to-text reference
- [Voquill reference project](https://github.com/josiahsrc/voquill) — MEDIUM confidence, open-source Tauri + React voice-to-text

---
*Architecture research for: local voice-to-text desktop tool (VoiceType)*
*Researched: 2026-02-27*
