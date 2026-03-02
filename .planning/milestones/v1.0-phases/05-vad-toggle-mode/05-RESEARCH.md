# Phase 5: VAD + Toggle Mode - Research

**Researched:** 2026-02-28
**Domain:** Silero VAD (Rust), toggle mode state machine, settings persistence
**Confidence:** MEDIUM-HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Silence detection tuning**
- ~1.5 second silence threshold before auto-stop in toggle mode
- Fixed threshold, not user-adjustable (no sensitivity slider)

**Mode switching UX**
- Settings panel toggle (radio button or switch) to choose hold-to-talk vs toggle mode
- Same hotkey for both modes — behavior changes based on selected mode
- Hold-to-talk is the default mode for new installs
- Second tap in toggle mode = instant hard stop, goes straight to transcription (no grace period)

**Pill feedback in toggle mode**
- Same pill visuals for both modes — no mode indicator or badge
- No visual hint before auto-stop — silence detected, immediately transition to processing
- Pill only appears during active recording, not while idle in toggle mode

**Speech gate strictness**
- VAD replaces the current crude 100ms/1600-sample minimum gate entirely
- VAD speech gate applies to both hold-to-talk and toggle modes (prevents whisper hallucination in either)
- ~300ms minimum detected speech required before buffer is sent to whisper — below that, discard (coughs, clicks, breaths)

### Claude's Discretion
- Maximum recording duration safety cap in toggle mode (prevent runaway recordings)
- VAD sensitivity/noise handling approach (use Silero defaults or tune thresholds)
- Silero VAD integration details (ONNX runtime, chunk processing strategy)

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| REC-02 | User can tap the hotkey to start recording and tap again to stop (toggle mode) | Hotkey handler mode-aware branching; PipelineState CAS guards tap-start vs tap-stop |
| REC-03 | In toggle mode, Silero VAD automatically detects silence and stops recording | voice_activity_detector 0.2.1 + VadWorker thread processing 512-sample chunks; silence counter at 1.5s triggers pipeline |
| REC-04 | User can switch between hold-to-talk and toggle mode in settings | `recording_mode` field in settings.json following existing read_saved_hotkey pattern; Tauri command to save/reload |
</phase_requirements>

---

## Summary

Phase 5 adds Silero VAD to gate the whisper pipeline and drives toggle mode (tap-to-start, auto-stop on silence). The primary library is `voice_activity_detector` v0.2.1 — a Silero VAD V5 wrapper using `ort` (ONNX Runtime) for inference. It is the most actively maintained Rust VAD crate, last updated August 2025, with verified Windows support and a clean iterator API.

VAD integration works by spawning a background worker thread that reads 512-sample (32ms at 16kHz) chunks from the existing audio buffer, calls `VoiceActivityDetector::predict()` for each chunk, and maintains a silence frame counter. When the counter crosses ~47 frames (1.5s / 32ms), the worker triggers pipeline execution — the same `run_pipeline()` function currently called on hotkey Release. The crude 1600-sample minimum gate in `run_pipeline()` is replaced by a VAD-based 300ms speech counter (~9 chunks).

Toggle mode requires mode-aware branching in both the `setup()` hotkey handler and `rebind_hotkey()`. In hold-to-talk mode behavior is unchanged. In toggle mode, `Pressed` starts recording (same as before), and `Released` is ignored; the recording stops only via second tap (another `Pressed` when RECORDING) or VAD auto-stop. Settings persistence follows the existing `read_saved_hotkey()` pattern: direct `std::fs` + `serde_json` read in `setup()`, a new Tauri command `set_recording_mode` for saves.

**Primary recommendation:** Use `voice_activity_detector = "0.2.1"` with default features (prebuilt ort download). Implement VAD as a `VadWorker` struct spawned when recording starts, killed when recording ends, that posts silence events back via an `Arc<AtomicBool>` or mpsc channel to the hotkey handler.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| voice_activity_detector | 0.2.1 | Silero VAD V5 inference; `predict()` per 512-sample chunk | Most current Rust Silero V5 crate, Windows verified, no model download step needed (ort downloads prebuilt runtime), clean `Send + Sync` struct |
| ort | 2.0.0-rc.10 (transitive via voice_activity_detector) | ONNX Runtime bindings; pulled in by voice_activity_detector | No direct dep needed — voice_activity_detector manages it |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tokio (already in Cargo.toml) | 1 | Async VAD worker loop via `tauri::async_runtime::spawn` | Already a project dep with `time` feature — use for the VAD polling loop |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| voice_activity_detector 0.2.1 | silero-vad-rs 0.1.2 | silero-vad-rs is older (Apr 2025 last update vs Aug 2025), fewer downloads, similar ort dependency, requires model file download separately; voice_activity_detector bundles model download via ort automatically |
| voice_activity_detector 0.2.1 | silero-vad-rust | Less documented, unclear maintenance status; crates.io page fails to load |
| ort default (prebuilt download) | load-dynamic feature | load-dynamic requires shipping onnxruntime.dll separately alongside the exe; more complex for Phase 7 installer. Default prebuilt download is simpler for development — revisit at Phase 7 |

**Installation:**
```bash
# Add to src-tauri/Cargo.toml [dependencies]
voice_activity_detector = "0.2.1"
```

No other direct dependencies needed — ort is pulled transitively and its prebuilt ONNX Runtime binary is downloaded automatically at build time.

---

## Architecture Patterns

### Recommended Project Structure

```
src-tauri/src/
├── audio.rs         # Existing — no changes needed for VAD hook
├── vad.rs           # NEW — VadWorker struct, silence detection, VAD gate logic
├── pipeline.rs      # MODIFY — replace 1600-sample gate with VAD gate fn
├── lib.rs           # MODIFY — mode-aware hotkey handler, RecordingMode state
└── settings.rs      # OPTIONAL — extract settings read/write to own module (or keep in lib.rs)
```

### Pattern 1: VadWorker — Chunk-Based Silence Detection

**What:** A background task that reads the audio buffer in 512-sample chunks (32ms), runs `predict()`, accumulates a silence frame counter, and fires a stop signal when silence exceeds threshold.

**When to use:** Spawned when RECORDING state starts in toggle mode; polled continuously; cancelled when recording ends for any reason (second tap or VAD auto-stop).

**Key design:** The VAD model processes chunks independently. At 16kHz, 512 samples = 32ms per chunk. For 1.5s silence: 1500ms / 32ms ≈ 47 consecutive below-threshold frames. For 300ms minimum speech gate: 300ms / 32ms ≈ 9 chunks must have been classified as speech before the buffer is sent to whisper.

**Example:**
```rust
// Source: voice_activity_detector 0.2.1 docs + project pattern
use voice_activity_detector::VoiceActivityDetector;

pub struct VadWorker {
    stop_tx: tokio::sync::oneshot::Sender<()>,
}

impl VadWorker {
    pub fn spawn(
        app: tauri::AppHandle,
        buffer: Arc<Mutex<Vec<f32>>>,
        auto_stop: Arc<AtomicBool>, // set true to trigger pipeline
    ) -> Self {
        let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel::<()>();

        tauri::async_runtime::spawn(async move {
            let mut vad = VoiceActivityDetector::builder()
                .sample_rate(16000u32)
                .chunk_size(512usize)
                .build()
                .expect("VAD init failed");

            const SPEECH_THRESHOLD: f32 = 0.5;
            const SILENCE_FRAMES_THRESHOLD: u32 = 47; // ~1.5s at 32ms/chunk
            const MIN_SPEECH_FRAMES: u32 = 9;         // ~300ms minimum speech gate

            let mut cursor: usize = 0;      // position in buffer already processed
            let mut silence_frames: u32 = 0;
            let mut speech_frames: u32 = 0;
            let mut ever_spoke = false;

            loop {
                // Check for cancellation (second tap in toggle mode)
                if stop_rx.try_recv().is_ok() {
                    break;
                }

                // Read new chunk from buffer
                let chunk: Option<Vec<f32>> = {
                    if let Ok(buf) = buffer.try_lock() {
                        if buf.len() >= cursor + 512 {
                            Some(buf[cursor..cursor + 512].to_vec())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };

                if let Some(samples) = chunk {
                    cursor += 512;
                    let prob = vad.predict(samples);

                    if prob >= SPEECH_THRESHOLD {
                        speech_frames += 1;
                        silence_frames = 0;
                        ever_spoke = true;
                    } else if ever_spoke {
                        silence_frames += 1;
                        if silence_frames >= SILENCE_FRAMES_THRESHOLD {
                            // Auto-stop: only if minimum speech was detected
                            if speech_frames >= MIN_SPEECH_FRAMES {
                                auto_stop.store(true, Ordering::Relaxed);
                            } else {
                                // Discard — cough/click/breath
                                auto_stop.store(false, Ordering::Relaxed);
                            }
                            // Signal pipeline to stop and run (or discard)
                            // Emit event or set flag; hotkey handler checks
                            break;
                        }
                    }
                } else {
                    // No new chunk yet — yield to avoid spinning
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }
        });

        VadWorker { stop_tx }
    }

    /// Cancel VAD worker (called on second tap or hold-to-talk release)
    pub fn cancel(self) {
        let _ = self.stop_tx.send(());
    }
}
```

### Pattern 2: Mode-Aware Hotkey Handler

**What:** The hotkey `Pressed` handler branches on `RecordingMode` managed state. Hold-to-talk behavior is unchanged. Toggle mode treats `Pressed` as start-if-idle or stop-if-recording.

**When to use:** Replace both `setup()` and `rebind_hotkey()` hotkey handler match arms.

**Example:**
```rust
// In lib.rs — mode-aware Pressed handler
ShortcutState::Pressed => {
    let mode = app.state::<RecordingMode>().get();

    match mode {
        Mode::HoldToTalk => {
            // Existing behavior: start on press (release stops)
            if pipeline.transition(pipeline::IDLE, pipeline::RECORDING) {
                start_recording_with_vad(app);
            }
        }
        Mode::Toggle => {
            if pipeline.transition(pipeline::IDLE, pipeline::RECORDING) {
                // First tap: start
                start_recording_with_vad(app);
            } else if pipeline.transition(pipeline::RECORDING, pipeline::PROCESSING) {
                // Second tap: instant hard stop, go straight to pipeline
                stop_recording_run_pipeline(app);
            }
            // If PROCESSING, ignore tap
        }
    }
}
ShortcutState::Released => {
    let mode = app.state::<RecordingMode>().get();
    match mode {
        Mode::HoldToTalk => {
            // Existing behavior: release stops
            if pipeline.transition(pipeline::RECORDING, pipeline::PROCESSING) {
                stop_recording_run_pipeline(app);
            }
        }
        Mode::Toggle => {
            // Toggle mode: release is ignored — VAD or second tap stops
        }
    }
}
```

### Pattern 3: RecordingMode Managed State

**What:** An AtomicU8 (or simple Mutex<Mode>) stored as Tauri managed state, loaded from settings.json at startup, saved via a Tauri command.

**Example:**
```rust
// In lib.rs
#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    HoldToTalk = 0,
    Toggle = 1,
}

pub struct RecordingMode(pub AtomicU8);

impl RecordingMode {
    pub fn new(mode: Mode) -> Self {
        RecordingMode(AtomicU8::new(mode as u8))
    }
    pub fn get(&self) -> Mode {
        match self.0.load(Ordering::Relaxed) {
            1 => Mode::Toggle,
            _ => Mode::HoldToTalk, // default
        }
    }
    pub fn set(&self, mode: Mode) {
        self.0.store(mode as u8, Ordering::Relaxed);
    }
}

// read at startup same as read_saved_hotkey():
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
        Ok(v) => v,
        Err(_) => return Mode::HoldToTalk,
    };
    match json.get("recording_mode").and_then(|v| v.as_str()) {
        Some("toggle") => Mode::Toggle,
        _ => Mode::HoldToTalk,
    }
}
```

### Pattern 4: VAD Gate Replacing 1600-Sample Gate in run_pipeline()

**What:** Instead of checking `samples.len() < 1600`, run the full buffer through VAD post-hoc to count detected speech frames. If fewer than ~9 frames (~300ms) classified as speech, discard.

**When to use:** Both hold-to-talk and toggle modes — VAD gate applies in both.

**Note:** For hold-to-talk, VAD runs synchronously in `run_pipeline()` as a post-processing gate (the buffer is already complete). For toggle mode, the VAD worker runs streaming during recording and the result determines whether to call `run_pipeline()` or discard.

**Example:**
```rust
// In pipeline.rs — replaces the current 1600-sample gate
const MIN_SPEECH_FRAMES: usize = 9; // ~300ms at 32ms/chunk

fn vad_speech_frame_count(samples: &[f32]) -> usize {
    // Run VAD on completed buffer synchronously
    // Called from run_pipeline() for hold-to-talk mode
    // For toggle mode, the VadWorker tracked this during recording
    let mut vad = VoiceActivityDetector::builder()
        .sample_rate(16000u32)
        .chunk_size(512usize)
        .build()
        .expect("VAD init");

    let speech_frames = samples
        .chunks(512)
        .filter(|chunk| vad.predict(chunk.to_vec()) >= 0.5)
        .count();

    speech_frames
}

// In run_pipeline(), replace lines 54-65:
if vad_speech_frame_count(&samples) < MIN_SPEECH_FRAMES {
    log::info!("Pipeline: VAD gate — insufficient speech ({} frames), discarding", ...);
    app.emit_to("pill", "pill-result", "error").ok();
    reset_to_idle(&app);
    return;
}
```

### Anti-Patterns to Avoid

- **Calling `vad.predict()` inside the cpal audio callback:** The cpal callback thread cannot block. Instantiate VoiceActivityDetector outside the callback and call predict() in a separate spawned task. The audio buffer is already accumulated via the existing callback.
- **Sharing `VoiceActivityDetector` across threads with `Arc<Mutex<>>` naively:** The struct is `Send + Sync`, but locking in the callback risks deadlock. Keep VAD on its own task thread only.
- **Running VAD on every single sample:** VAD requires exactly 512-sample chunks. Feed it 512 samples at a time, not individual samples.
- **Not resetting VadWorker state on new recording:** The Silero model has internal LSTM state. Create a fresh `VoiceActivityDetector` for each recording session (or call `.reset()` if available). Stale state from previous recording bleeds through.
- **Forgetting to cancel VadWorker on pipeline completion:** If run_pipeline() is called and the VAD worker is still polling, it must be cancelled to avoid a second auto-stop trigger after processing completes.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Speech probability scoring | Custom energy/RMS threshold | voice_activity_detector + Silero V5 model | Energy-based VAD false-positives on keyboard clicks, HVAC noise, desk thumps; neural model (Silero) handles these correctly |
| Silence duration tracking | Time-based sleep loop | Frame counter (silence_frames >= 47) | Audio buffer is the ground truth; frame counter is deterministic and matches actual audio processed |
| ONNX model inference | Custom neural network runner | ort crate (via voice_activity_detector dep) | ONNX Runtime is 3000+ SLOC of platform-specific inference optimization; not hand-rollable |
| Settings persistence | Custom file format | Extend existing settings.json with `recording_mode` key | Pattern already established with hotkey; consistency, no new file needed |

**Key insight:** Silero VAD's neural model specifically distinguishes speech from non-speech noise (keyboard, breathing, clicks) at a quality level unreachable with simple energy/RMS thresholding. The 300ms minimum speech gate is the backstop against brief false positives that sneak past the model.

---

## Common Pitfalls

### Pitfall 1: ort DLL Shadowing on Windows

**What goes wrong:** An old `onnxruntime.dll` in `C:\Windows\System32` (some Windows installs ship one) is found before the version ort downloads, causing runtime assertion errors or version mismatches.

**Why it happens:** Windows DLL search order finds System32 before the exe directory unless the exe's own directory is searched first (which it is for the main exe — but only if the DLL is in the same directory).

**How to avoid:** The `copy-dylibs` feature (enabled by default in ort 2.0) copies onnxruntime.dll to the Cargo target directory. For development this works. For Tauri, ensure the DLL ends up next to the compiled executable. Add `onnxruntime.dll` to Tauri's `resources` list in `tauri.conf.json` if the installer path is different.

**Warning signs:** Runtime panic with "ONNX Runtime version mismatch" or `OrtStatus` error on `VoiceActivityDetector::builder().build()`.

### Pitfall 2: VoiceActivityDetector Build Download Failure in Air-Gapped/CI Environments

**What goes wrong:** `voice_activity_detector` uses `ort` with `download-binaries` default feature, which fetches prebuilt ONNX Runtime from pyke's CDN at build time. In a locked-down CI or offline build, this fails silently or with an opaque linker error.

**Why it happens:** The `ort` build script makes an HTTPS request during compilation.

**How to avoid:** For development builds, ensure internet access during first build (binary is cached). For CI, either pre-download and cache the `ort` download artifacts, or switch to `voice_activity_detector = { version = "0.2.1", features = ["load-dynamic"] }` with `ORT_DYLIB_PATH` pointed at a pre-downloaded DLL. This is a Phase 7 concern for the installer; don't block Phase 5 on it.

**Warning signs:** Build failure mentioning download or TLS, or linker error about missing `onnxruntime` symbols.

### Pitfall 3: VAD Worker Not Cancelled When Second Tap Fires

**What goes wrong:** User taps to stop early. Hotkey Pressed fires, transitions RECORDING -> PROCESSING, calls run_pipeline(). Meanwhile VadWorker is still running its polling loop and fires auto_stop after silence_frames threshold, attempting a second pipeline run.

**Why it happens:** VadWorker is a separate async task; it doesn't know the recording stopped via second tap.

**How to avoid:** Store `VadWorker` (or its cancel handle) in managed state. On second tap (and on VAD auto-stop), call `vad_worker.cancel()` before spawning `run_pipeline()`. Use `PipelineState` CAS as the authoritative guard — `run_pipeline()` only starts if CAS succeeds, so even if a second signal arrives late, it won't double-execute.

**Warning signs:** Double transcription output, or two whisper runs overlapping (high CPU spike after auto-stop).

### Pitfall 4: Silero Model Internal State Not Reset Between Recordings

**What goes wrong:** After a recording session, the Silero LSTM model has internal hidden state from the previous audio. The next recording starts with stale activations, causing the first 200-500ms to be misclassified.

**Why it happens:** The Silero model is stateful between `predict()` calls by design (for streaming). A new recording is a new audio context.

**How to avoid:** Create a fresh `VoiceActivityDetector` instance for each recording session. The cost is minimal (just ONNX session initialization, not model file re-load — the model weights are cached by ort). Alternatively, call `vad.reset()` if the crate exposes it — as of 0.2.1 the public API includes `reset()`.

**Warning signs:** First tap always correctly detected as speech, but second recording after a silent gap is immediately auto-stopped even though user is speaking.

### Pitfall 5: try_lock() Contention at 512-Sample Poll Rate

**What goes wrong:** VAD worker tries to read 512 samples from `buffer` at ~30Hz poll rate. The audio callback also `try_lock()`s the same `buffer` mutex. Under high CPU load both can fail simultaneously, causing VAD to starve.

**Why it happens:** `try_lock()` is non-blocking — if audio callback holds the lock, VAD gets None and skips.

**How to avoid:** VAD reads by cursor position (read 512 from cursor, advance cursor) rather than polling the whole buffer. This means it can read stale data while the callback is actively writing — which is fine for VAD purposes. Use `buffer.lock()` (blocking) in the VAD worker thread since it's not a real-time thread. Only the cpal callback uses `try_lock()`.

### Pitfall 6: Hold-to-Talk Keydown Repeat Events

**What goes wrong:** OS key repeat sends multiple `Pressed` events when the hotkey is held. In hold-to-talk mode, the current `transition(IDLE, RECORDING)` CAS prevents double-starts. In toggle mode, repeated `Pressed` events could trigger multiple stop attempts.

**Why it happens:** OS keyboard repeat is typically 300ms initial delay, then 30ms repeat — at 30Hz, about 30 events per second.

**How to avoid:** The `pipeline.transition(RECORDING, PROCESSING)` CAS in the toggle stop path already guards against this — only the first successful CAS runs the pipeline. No additional debouncing needed. Confirm behavior with actual hardware testing.

---

## Code Examples

### Building VoiceActivityDetector for 16kHz

```rust
// Source: https://docs.rs/voice_activity_detector/0.2.1/voice_activity_detector/
use voice_activity_detector::VoiceActivityDetector;

let mut vad = VoiceActivityDetector::builder()
    .sample_rate(16000u32)
    .chunk_size(512usize)  // 32ms at 16kHz — required fixed size for V5 model
    .build()
    .expect("VAD initialization failed");

// predict() accepts Vec<f32> or &[f32] (any iterable of samples)
let chunk: Vec<f32> = audio_buffer[0..512].to_vec();
let probability: f32 = vad.predict(chunk);
// probability >= 0.5 => speech; < 0.5 => silence/noise
```

### Silence Counter for Auto-Stop

```rust
// Source: research — standard pattern from Silero VAD documentation
// At 16kHz, 512 samples = 32ms per chunk
// 1500ms / 32ms ≈ 47 frames for silence threshold
// 300ms / 32ms ≈ 9 frames for minimum speech gate

const SILENCE_FRAMES_THRESHOLD: u32 = 47;  // ~1.5s
const MIN_SPEECH_FRAMES: u32 = 9;          // ~300ms
const SPEECH_PROBABILITY_THRESHOLD: f32 = 0.5;  // Silero default

let mut silence_frames: u32 = 0;
let mut speech_frames: u32 = 0;
let mut ever_spoke = false;

// Per-chunk in the VAD worker loop:
let prob = vad.predict(chunk);
if prob >= SPEECH_PROBABILITY_THRESHOLD {
    speech_frames += 1;
    silence_frames = 0;
    ever_spoke = true;
} else if ever_spoke {
    silence_frames += 1;
    if silence_frames >= SILENCE_FRAMES_THRESHOLD {
        // Trigger auto-stop
        if speech_frames >= MIN_SPEECH_FRAMES {
            // Enough speech detected — run pipeline
        } else {
            // Insufficient speech — discard buffer
        }
        break;
    }
}
// If !ever_spoke: keep waiting (silence before first speech doesn't count)
```

### Settings Persistence for recording_mode

```rust
// Source: existing read_saved_hotkey() pattern in lib.rs

// Read at startup (sync, same as hotkey):
fn read_saved_mode(app: &tauri::App) -> Mode {
    let Ok(data_dir) = app.path().app_data_dir() else { return Mode::HoldToTalk };
    let settings_path = data_dir.join("settings.json");
    let Ok(contents) = std::fs::read_to_string(&settings_path) else { return Mode::HoldToTalk };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&contents) else { return Mode::HoldToTalk };
    match json.get("recording_mode").and_then(|v| v.as_str()) {
        Some("toggle") => Mode::Toggle,
        _ => Mode::HoldToTalk,
    }
}

// Save via Tauri command (same pattern as existing hotkey save in frontend):
#[tauri::command]
fn set_recording_mode(app: tauri::AppHandle, mode: String) -> Result<(), String> {
    // 1. Update managed state immediately
    let recording_mode = app.state::<RecordingMode>();
    match mode.as_str() {
        "toggle" => recording_mode.set(Mode::Toggle),
        _ => recording_mode.set(Mode::HoldToTalk),
    }

    // 2. Persist to settings.json (merge into existing JSON)
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let settings_path = data_dir.join("settings.json");
    let mut json: serde_json::Value = std::fs::read_to_string(&settings_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    json["recording_mode"] = serde_json::Value::String(mode);
    std::fs::write(&settings_path, serde_json::to_string_pretty(&json).unwrap())
        .map_err(|e| e.to_string())
}
```

### Maximum Recording Duration Safety Cap (Claude's Discretion)

The context left this to Claude. Recommended approach: 60-second hard cap. After 60 seconds in RECORDING state, treat as VAD auto-stop (if speech detected, run pipeline; if not, discard).

```rust
const MAX_RECORDING_FRAMES: u32 = 1875; // 60s / 32ms per chunk

let mut total_frames: u32 = 0;

// In VAD worker loop, after processing each chunk:
total_frames += 1;
if total_frames >= MAX_RECORDING_FRAMES {
    log::warn!("VAD: 60s safety cap reached, auto-stopping");
    // Treat as auto-stop
    break;
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Energy/RMS threshold for silence detection | Neural VAD (Silero V5) | 2023+ | 10-100x fewer false positives on keyboard noise, breathing, HVAC |
| Monolithic VAD + transcription libraries | Separate composable crates (voice_activity_detector + whisper-rs) | 2024 | Can swap VAD without changing transcription pipeline |
| Silero V4 ONNX model (512/1024/1536 chunk sizes) | Silero V5 (fixed 512 at 16kHz only) | 2024 | V5 is more accurate but removes chunk size flexibility — must use exactly 512 |
| Manual ONNX runtime linking | ort 2.0 with download-binaries default | 2024 | Zero-config build for development; DLL managed automatically |

**Deprecated/outdated:**
- silero-vad-rs 0.1.x: Last updated April 2025, uses ort 2.0.0-rc.9 (vs rc.10 in voice_activity_detector), less actively maintained. Do not use.
- Custom 1600-sample minimum gate in pipeline.rs (lines 54-65): Replaced entirely by VAD gate.

---

## Open Questions

1. **ort prebuilt download at build time — what does it cache?**
   - What we know: ort downloads ONNX Runtime binaries from pyke's CDN during `cargo build`. The copy-dylibs feature copies the DLL to the target directory.
   - What's unclear: Whether subsequent `cargo build` runs re-download or use a cached copy. If re-downloaded each clean build, CI will be slow.
   - Recommendation: Test with `cargo build` twice — if the second build is fast, it's cached. This affects CI setup but not Phase 5 development.

2. **VoiceActivityDetector::reset() availability**
   - What we know: API docs mention a `reset()` method on the struct. Struct is `Send + Sync`.
   - What's unclear: Whether reset() clears LSTM hidden state or just counters.
   - Recommendation: Create a fresh `VoiceActivityDetector` per recording session (Pattern 4) rather than relying on reset() — fresh instance is guaranteed correct state.

3. **Optimal speech probability threshold for this use case**
   - What we know: Silero default is 0.5. The model is trained for broad speech detection.
   - What's unclear: Performance on heavy-accent speech, whispered speech, or noisy office environments.
   - Recommendation: Start with 0.5 (Silero default). The context decision says no user-adjustable slider, so Claude's discretion applies. 0.5 is the right starting point; tune to 0.4 if testing reveals missed speech.

4. **VadWorker ownership and managed state design**
   - What we know: Tauri managed state uses `Arc<T>` and requires `Send + Sync`. A oneshot channel sender (`tokio::sync::oneshot::Sender`) is `Send` but not `Clone`.
   - What's unclear: How to store the cancel handle in managed state when it's a move-only type.
   - Recommendation: Use `Arc<Mutex<Option<VadWorkerHandle>>>` as managed state, where `VadWorkerHandle` contains the oneshot sender. Take the Option on second tap (replace with None) to cancel.

---

## Sources

### Primary (HIGH confidence)
- `https://docs.rs/voice_activity_detector/0.2.1/voice_activity_detector/` — API reference, chunk size requirements, predict() method, reset(), Send+Sync guarantees
- `https://github.com/nkeenan38/voice_activity_detector` — README, feature flags, Windows verification, Silero V5 model
- Project source files (audio.rs, pipeline.rs, lib.rs, pill.rs) — existing integration points, buffer patterns, try_lock usage, managed state patterns

### Secondary (MEDIUM confidence)
- `https://ort.pyke.io/setup/linking` — ort 2.0 linking strategies, copy-dylibs behavior, Windows DLL placement
- `https://deepwiki.com/pykeio/ort/2.1-installation-and-setup` — ort 2.0 default feature set, download-binaries, Tauri distribution consideration
- `https://github.com/tauri-apps/tauri/discussions/11687` — DLL bundling via resources workaround for Tauri (confirmed by maintainer)
- `https://github.com/snakers4/silero-vad` — Silero VAD parameters (min_silence_duration_ms, threshold=0.5 default, chunk sizes)

### Tertiary (LOW confidence — needs validation)
- WebSearch results on silero-vad-rs 0.1.2 version/date — confirmed April 2025 last update via lib.rs
- WebSearch on ort System32 DLL shadowing pitfall — multiple sources mention this; needs verification on target machine

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — voice_activity_detector 0.2.1 verified via official docs/GitHub; Windows support confirmed; API verified
- Architecture: MEDIUM — patterns are derived from existing project code + library API; actual integration untested
- Pitfalls: MEDIUM — DLL shadowing and VAD worker cancellation are verified patterns from multiple sources; threshold numbers are mathematically derived (32ms * 47 = 1504ms ≈ 1.5s), confirmed by Silero docs

**Research date:** 2026-02-28
**Valid until:** 2026-03-30 (voice_activity_detector is stable; ort 2.0 is RC but stable enough for this use case)
