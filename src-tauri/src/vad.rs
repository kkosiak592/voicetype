use std::sync::{Arc, Mutex};
use tauri::Emitter;
use tauri::Manager;
use tokio::time::Duration;
use voice_activity_detector::VoiceActivityDetector;

// --- Constants ---

/// Silero default speech probability threshold.
/// Predictions >= this value are classified as speech.
const SPEECH_PROBABILITY_THRESHOLD: f32 = 0.5;

/// Number of consecutive below-threshold (silence) chunks before auto-stop in toggle mode.
/// At 32ms per chunk: 94 chunks * 32ms = 3008ms ≈ 3.0 seconds.
const SILENCE_FRAMES_THRESHOLD: u32 = 94;

/// Minimum number of chunks classified as speech before the buffer is eligible
/// for whisper inference. Below this, the buffer is discarded as noise.
/// At 32ms per chunk: 9 chunks * 32ms = 288ms ≈ 300ms.
const MIN_SPEECH_FRAMES: u32 = 9;

/// Safety cap for toggle mode recordings. After this many chunks, auto-stop
/// regardless of silence/speech status (prevents runaway recording).
/// At 32ms per chunk: 1875 chunks * 32ms = 60,000ms = 60 seconds.
const MAX_RECORDING_FRAMES: u32 = 1875;

/// Silero V5 fixed chunk size at 16kHz. The VAD model processes exactly 512
/// samples per predict() call — not a tunable parameter.
const CHUNK_SIZE: usize = 512;

// --- Post-hoc VAD gate (hold-to-talk mode) ---

/// Synchronous post-hoc VAD gate for completed audio buffers.
///
/// Called from `run_pipeline()` after recording stops. Runs Silero VAD over the
/// entire buffer, counting chunks classified as speech. Returns `true` if enough
/// speech was detected for transcription, `false` if the buffer should be
/// discarded (silence, coughs, clicks, breaths under 300ms).
///
/// Creates a fresh `VoiceActivityDetector` per call (no stale LSTM state).
pub fn vad_gate_check(samples: &[f32]) -> bool {
    let mut vad = match VoiceActivityDetector::builder()
        .sample_rate(16000u32)
        .chunk_size(CHUNK_SIZE)
        .build()
    {
        Ok(v) => v,
        Err(e) => {
            log::error!("VAD: failed to initialize VoiceActivityDetector: {}", e);
            // Fail open — allow transcription rather than silently dropping valid audio
            return true;
        }
    };

    let mut speech_frames: u32 = 0;
    let total_chunks = samples.len() / CHUNK_SIZE;

    for chunk in samples.chunks(CHUNK_SIZE) {
        if chunk.len() < CHUNK_SIZE {
            // Partial final chunk — skip (insufficient data for V5 model)
            break;
        }
        let prob = vad.predict(chunk.to_vec());
        if prob >= SPEECH_PROBABILITY_THRESHOLD {
            speech_frames += 1;
        }
    }

    log::info!(
        "VAD gate: {}/{} chunks classified as speech (threshold: {} chunks)",
        speech_frames,
        total_chunks,
        MIN_SPEECH_FRAMES
    );

    speech_frames >= MIN_SPEECH_FRAMES
}

// --- Streaming VAD worker (toggle mode) ---

/// Cancel handle for an active VAD worker.
///
/// Stored as `Arc<Mutex<Option<VadWorkerHandle>>>` in managed state so it can
/// be taken (replaced with None) on second tap or early stop.
/// Plan 02 wires this into the toggle mode hotkey handler.
pub struct VadWorkerHandle {
    pub cancel_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl VadWorkerHandle {
    /// Send a cancellation signal to the VAD worker task.
    /// No-op if already cancelled or worker completed.
    pub fn cancel(&mut self) {
        if let Some(tx) = self.cancel_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// Spawn a streaming VAD worker for toggle mode.
///
/// Reads 512-sample chunks from `buffer` in an async loop, running Silero VAD
/// on each chunk. Tracks silence duration and speech frame count.
///
/// **Auto-stop conditions:**
/// - `silence_frames >= SILENCE_FRAMES_THRESHOLD` after speech detected:
///   - If `speech_frames >= MIN_SPEECH_FRAMES`: triggers pipeline execution
///   - Otherwise: discards (cough/click/breath)
/// - `total_frames >= MAX_RECORDING_FRAMES`: 60s safety cap — triggers pipeline
///   if any speech detected, discards otherwise
/// - `cancel_rx` receives a message: external cancellation (second tap, etc.)
///
/// **CRITICAL — No circular module coupling:** This function references pipeline
/// types via inline `crate::pipeline::` paths only. Do NOT add
/// `use crate::pipeline;` at the top of this file — that would create a circular
/// import since pipeline.rs does `use crate::vad;`.
pub fn spawn_vad_worker(
    app: tauri::AppHandle,
    buffer: Arc<Mutex<Vec<f32>>>,
) -> VadWorkerHandle {
    let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel::<()>();

    tauri::async_runtime::spawn(async move {
        // Create a fresh VoiceActivityDetector per recording session.
        // The Silero model has internal LSTM state — reusing across sessions
        // causes stale activations and misclassification at session start.
        let mut vad = match VoiceActivityDetector::builder()
            .sample_rate(16000u32)
            .chunk_size(CHUNK_SIZE)
            .build()
        {
            Ok(v) => v,
            Err(e) => {
                log::error!("VAD worker: failed to initialize VoiceActivityDetector: {}", e);
                return;
            }
        };

        let mut cursor: usize = 0;
        let mut silence_frames: u32 = 0;
        let mut speech_frames: u32 = 0;
        let mut ever_spoke = false;
        let mut total_frames: u32 = 0;

        loop {
            // Check for external cancellation (second tap or hold-to-talk release)
            if cancel_rx.try_recv().is_ok() {
                log::info!("VAD worker: cancelled externally at {} total frames", total_frames);
                break;
            }

            // Read the next 512-sample chunk from the shared buffer.
            // Use blocking lock (not try_lock) — VAD worker is not a real-time thread
            // and blocking briefly here is fine. Only the cpal callback uses try_lock.
            let chunk: Option<Vec<f32>> = {
                let buf = buffer.lock().unwrap_or_else(|e| e.into_inner());
                if buf.len() >= cursor + CHUNK_SIZE {
                    Some(buf[cursor..cursor + CHUNK_SIZE].to_vec())
                } else {
                    None
                }
            };

            if let Some(samples) = chunk {
                cursor += CHUNK_SIZE;
                total_frames += 1;

                let prob = vad.predict(samples);

                if prob >= SPEECH_PROBABILITY_THRESHOLD {
                    speech_frames += 1;
                    silence_frames = 0;
                    ever_spoke = true;
                } else if ever_spoke {
                    silence_frames += 1;
                    if silence_frames >= SILENCE_FRAMES_THRESHOLD {
                        // 3.0s of silence after speech — auto-stop
                        log::info!(
                            "VAD worker: silence threshold reached after {} speech frames \
                             (total: {} frames, cursor: {} samples)",
                            speech_frames,
                            total_frames,
                            cursor
                        );
                        trigger_auto_stop(&app, speech_frames).await;
                        break;
                    }
                }

                // 60-second safety cap — prevent runaway recordings
                if total_frames >= MAX_RECORDING_FRAMES {
                    log::warn!(
                        "VAD worker: 60s safety cap reached ({} speech frames detected)",
                        speech_frames
                    );
                    trigger_auto_stop(&app, speech_frames).await;
                    break;
                }
            } else {
                // No new chunk available yet — yield to avoid spinning CPU
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }

        log::info!(
            "VAD worker: exiting (speech={}, silence={}, total={}, cursor={})",
            speech_frames,
            silence_frames,
            total_frames,
            cursor
        );
    });

    VadWorkerHandle {
        cancel_tx: Some(cancel_tx),
    }
}

/// Execute auto-stop logic after VAD detects end-of-speech (or safety cap).
///
/// If enough speech was detected (`speech_frames >= MIN_SPEECH_FRAMES`):
///   - CAS RECORDING -> PROCESSING (only fires if still in recording state)
///   - Stop level stream, emit pill-state processing, update tray
///   - Spawn `run_pipeline()` via full crate path to avoid circular imports
///     (`run_pipeline()` itself calls `flush_and_stop()` on the audio capture)
///
/// If insufficient speech:
///   - CAS RECORDING -> IDLE, emit pill-result error, reset tray and pill to idle
///
/// If CAS fails (pipeline already transitioned, e.g., user tapped again):
///   - Exit silently — the other tap handler owns the pipeline
async fn trigger_auto_stop(app: &tauri::AppHandle, speech_frames: u32) {
    let pipeline = app.state::<crate::pipeline::PipelineState>();

    if speech_frames < MIN_SPEECH_FRAMES {
        // Insufficient speech — discard without running whisper.
        // CAS: RECORDING -> IDLE to release the state machine.
        // If this fails, another handler already transitioned — nothing to do.
        if pipeline.transition(crate::pipeline::RECORDING, crate::pipeline::IDLE) {
            log::info!(
                "VAD worker: insufficient speech ({} frames < {} required), discarding",
                speech_frames,
                MIN_SPEECH_FRAMES
            );

            // Stop the level stream
            let stream_active = app.state::<crate::LevelStreamActive>();
            stream_active.0.store(false, std::sync::atomic::Ordering::Relaxed);

            // Emit error result and reset pill/tray to idle
            app.emit_to("pill", "pill-result", "error").ok();
            crate::tray::set_tray_state(app, crate::tray::TrayState::Idle);
            if let Some(tray) = app.tray_by_id("tray") {
                let _ = tray.set_tooltip(Some("VoiceType — idle"));
            }
            app.emit_to("pill", "pill-state", "idle").ok();
            app.emit_to("pill", "pill-hide", ()).ok();
        }
        return;
    }

    // Enough speech detected — attempt to run the pipeline.
    // CAS: RECORDING -> PROCESSING
    if !pipeline.transition(crate::pipeline::RECORDING, crate::pipeline::PROCESSING) {
        // Another handler beat us — likely a second tap. Exit silently.
        log::info!("VAD worker: CAS RECORDING->PROCESSING failed, another handler owns the pipeline");
        return;
    }

    log::info!(
        "VAD worker: {} speech frames detected — triggering pipeline",
        speech_frames
    );

    // Stop the level stream (matches hold-to-talk release path)
    let stream_active = app.state::<crate::LevelStreamActive>();
    stream_active.0.store(false, std::sync::atomic::Ordering::Relaxed);

    // Pill: switch to processing state
    app.emit_to("pill", "pill-state", "processing").ok();

    // Tray: processing state
    crate::tray::set_tray_state(app, crate::tray::TrayState::Processing);

    log::info!("VAD worker: RECORDING -> PROCESSING, spawning run_pipeline()");

    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        crate::pipeline::run_pipeline(app_clone).await;
    });
}
