use std::sync::atomic::{AtomicU8, Ordering};
use tauri::Emitter;
use tauri::Manager;
use crate::vad;

/// Type-safe pipeline phase. Stored as a `#[repr(u8)]` enum so it maps cleanly
/// to the underlying AtomicU8.
#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(u8)]
pub enum Phase {
    Idle = 0,
    Recording = 1,
    Processing = 2,
}

// Keep public constants for callers that use pipeline::IDLE etc.
pub const IDLE: Phase = Phase::Idle;
pub const RECORDING: Phase = Phase::Recording;
pub const PROCESSING: Phase = Phase::Processing;

/// AtomicU8-backed state machine for the hold-to-talk pipeline.
///
/// Transitions are guarded by compare_exchange to prevent concurrent recordings
/// or double-starts. Every exit path in run_pipeline must call reset_to_idle().
pub struct PipelineState(AtomicU8);

impl PipelineState {
    pub fn new() -> Self {
        PipelineState(AtomicU8::new(Phase::Idle as u8))
    }

    /// Attempt a CAS transition from `from` -> `to`. Returns true if successful.
    /// Returns false if the current state is not `from` (pipeline is busy).
    pub fn transition(&self, from: Phase, to: Phase) -> bool {
        self.0
            .compare_exchange(from as u8, to as u8, Ordering::SeqCst, Ordering::Relaxed)
            .is_ok()
    }

    /// Reset to Idle unconditionally. Used by every exit path in run_pipeline.
    pub fn reset_to_idle(&self) {
        self.0.store(Phase::Idle as u8, Ordering::SeqCst);
    }
}

/// Core pipeline orchestration — called from the Released hotkey handler.
///
/// Steps:
///   1. Stop recording, get audio buffer
///   2. Speech gate — mode-conditional:
///      - Hold-to-talk: sample count check (4800 samples = 300ms @ 16kHz; VAD adds 20-30ms latency without benefit)
///      - Toggle mode: full Silero VAD post-hoc scan (needed for auto-stop timing accuracy)
///   3. Engine dispatch: Whisper or Parakeet depending on ActiveEngine managed state
///   4. Text formatting (trim_start + trailing space)
///   5. Corrections engine + ALL CAPS (applied regardless of engine)
///   6. Inject text via clipboard paste
///   7. Reset to idle
///
/// Every early-return path calls reset_to_idle() — no stuck states.
pub async fn run_pipeline(app: tauri::AppHandle) {
    // 1. Stop recording and get audio buffer
    // Lock AudioCaptureMutex, flush+get buffer, then drop guard before any async work.
    let (sample_count, samples) = {
        let audio_mutex = app.state::<crate::audio::AudioCaptureMutex>();
        let guard = audio_mutex.0.lock().unwrap_or_else(|e| e.into_inner());
        match guard.as_ref() {
            Some(audio) => {
                let count = audio.flush_and_stop();
                let buf = audio.get_buffer();
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

    // Cancel any active VAD worker (prevents double-trigger if run_pipeline
    // is called from second tap while VAD worker is still polling)
    cancel_stale_vad_worker(&app);

    // 2. Speech gate: mode-conditional
    //    Hold-to-talk: simple sample-count check (300ms minimum = 4800 samples at 16kHz).
    //                  User intent is explicit — VAD adds 20-30ms latency without benefit.
    //    Toggle mode: full Silero VAD post-hoc scan (used for auto-stop timing accuracy).
    let recording_mode = app.state::<crate::RecordingMode>().get();
    let should_transcribe = match recording_mode {
        crate::Mode::HoldToTalk => {
            if samples.len() < 4800 {
                log::info!(
                    "Pipeline: hold-to-talk audio too short ({} samples < 4800), discarding",
                    samples.len()
                );
                false
            } else {
                true
            }
        }
        crate::Mode::Toggle => {
            if samples.len() < 512 {
                log::info!(
                    "Pipeline: audio too short for VAD ({} samples), discarding",
                    samples.len()
                );
                false
            } else {
                //    Full VAD speech gate:
                //    Requires >= 9 chunks (~300ms) classified as speech by the neural model.
                //    Prevents whisper hallucination on silence, coughs, clicks, breathing.
                vad::vad_gate_check(&samples)
            }
        }
    };
    if !should_transcribe {
        log::info!(
            "Pipeline: speech gate rejected — {} samples ({:.1}s audio), mode={:?}",
            samples.len(),
            samples.len() as f32 / 16000.0,
            recording_mode
        );
        app.emit_to("pill", "pill-result", "error").ok();
        reset_to_idle(&app);
        return;
    }

    log::info!(
        "Pipeline: processing {} samples ({:.1}s audio)",
        samples.len(),
        samples.len() as f32 / 16000.0
    );
    let _ = sample_count; // used for logging above; suppress unused warning

    // VAD silence trim: remove leading/trailing silence before engine dispatch.
    // Applies to both Whisper and Parakeet — engine-agnostic improvement.
    // Cost: ~2-5ms (one Silero VAD pass over the buffer).
    let samples = vad::vad_trim_silence(&samples);

    // Read active engine before any async work (AppHandle not Send into spawn_blocking).
    let active_engine = {
        let state = app.state::<crate::ActiveEngine>();
        let guard = state.0.lock().unwrap_or_else(|e| e.into_inner());
        *guard
    };

    // Read initial_prompt for Whisper (Parakeet doesn't support it).
    // Done outside spawn_blocking because AppHandle is not Send.
    #[cfg(feature = "whisper")]
    let initial_prompt: String = {
        let profile = app.state::<crate::profiles::ActiveProfile>();
        let guard = profile.0.lock().unwrap_or_else(|e| e.into_inner());
        guard.initial_prompt.clone()
    };

    // 3. Engine dispatch: Whisper or Parakeet
    //
    // `samples` is moved into whichever spawn_blocking closure executes.
    // The match arms are mutually exclusive at runtime; Rust requires the variable
    // to be available in all arms — we clone before the match to satisfy the borrow
    // checker without runtime cost in the hot path (only one arm runs).
    let transcription: String = match active_engine {
        crate::TranscriptionEngine::Whisper => {
            #[cfg(feature = "whisper")]
            {
                // Lock WhisperStateMutex, clone Arc, drop guard before spawn_blocking
                let ctx = {
                    let whisper_mutex = app.state::<crate::WhisperStateMutex>();
                    let guard = whisper_mutex.0.lock().unwrap_or_else(|e| e.into_inner());
                    match guard.as_ref() {
                        Some(ctx) => ctx.clone(),
                        None => {
                            log::error!("Pipeline: whisper model not loaded");
                            app.emit_to("pill", "pill-result", "error").ok();
                            reset_to_idle(&app);
                            return;
                        }
                    }
                };
                match tauri::async_runtime::spawn_blocking(move || {
                    crate::transcribe::transcribe_audio(&ctx, &samples, &initial_prompt)
                })
                .await
                {
                    Ok(Ok(text)) => text,
                    Ok(Err(e)) => {
                        log::error!("Pipeline: whisper inference error: {}", e);
                        app.emit_to("pill", "pill-result", "error").ok();
                        reset_to_idle(&app);
                        return;
                    }
                    Err(e) => {
                        log::error!("Pipeline: spawn_blocking panicked: {}", e);
                        app.emit_to("pill", "pill-result", "error").ok();
                        reset_to_idle(&app);
                        return;
                    }
                }
            }
            #[cfg(not(feature = "whisper"))]
            {
                log::warn!("Pipeline: whisper feature not enabled, cannot transcribe");
                app.emit_to("pill", "pill-result", "error").ok();
                reset_to_idle(&app);
                return;
            }
        }
        crate::TranscriptionEngine::Parakeet => {
            #[cfg(feature = "parakeet")]
            {
                // Clone Arc<Mutex<ParakeetTDT>> out of managed state.
                // The inner Mutex is locked inside spawn_blocking for &mut access.
                let parakeet_arc = {
                    let state = app.state::<crate::ParakeetStateMutex>();
                    let guard = state.0.lock().unwrap_or_else(|e| e.into_inner());
                    match guard.as_ref() {
                        Some(arc) => arc.clone(),
                        None => {
                            log::error!("Pipeline: Parakeet model not loaded");
                            app.emit_to("pill", "pill-result", "error").ok();
                            reset_to_idle(&app);
                            return;
                        }
                    }
                };
                match tauri::async_runtime::spawn_blocking(move || {
                    let mut guard = parakeet_arc.lock().unwrap_or_else(|e| e.into_inner());
                    crate::transcribe_parakeet::transcribe_with_parakeet(&mut guard, &samples)
                })
                .await
                {
                    Ok(Ok(text)) => text,
                    Ok(Err(e)) => {
                        log::error!("Pipeline: parakeet inference error: {}", e);
                        app.emit_to("pill", "pill-result", "error").ok();
                        reset_to_idle(&app);
                        return;
                    }
                    Err(e) => {
                        log::error!("Pipeline: parakeet spawn_blocking panicked: {}", e);
                        app.emit_to("pill", "pill-result", "error").ok();
                        reset_to_idle(&app);
                        return;
                    }
                }
            }
            #[cfg(not(feature = "parakeet"))]
            {
                log::warn!(
                    "Pipeline: parakeet feature not enabled, engine set to parakeet — falling back"
                );
                app.emit_to("pill", "pill-result", "error").ok();
                reset_to_idle(&app);
                return;
            }
        }
    };

    // 4. Text formatting per CONTEXT.md locked decisions:
    //    - Trim leading whitespace
    //    - Append trailing space for consecutive dictation bridging
    //    - Discard empty/whitespace-only results silently
    //    Applied regardless of engine (Whisper or Parakeet).
    let trimmed = transcription.trim_start();
    if trimmed.is_empty() || trimmed.chars().all(|c| c.is_whitespace()) {
        log::info!("Pipeline: empty transcription, discarding");
        app.emit_to("pill", "pill-result", "error").ok();
        reset_to_idle(&app);
        return;
    }

    // 5. Apply corrections (word-level find-and-replace per active profile dictionary)
    //    Applied regardless of engine — corrections engine is engine-agnostic.
    let corrected = {
        let engine = app.state::<crate::corrections::CorrectionsState>();
        let guard = engine.0.lock().unwrap_or_else(|e| e.into_inner());
        guard.apply(trimmed)
    };

    // Apply ALL CAPS if active profile flag is set (engine-agnostic).
    let formatted = {
        let profile = app.state::<crate::profiles::ActiveProfile>();
        let guard = profile.0.lock().unwrap_or_else(|e| e.into_inner());
        if guard.all_caps {
            corrected.to_uppercase()
        } else {
            corrected
        }
    };

    let to_inject = format!("{} ", formatted); // trailing space per CONTEXT.md

    log::info!(
        "Pipeline: injecting '{}' ({} chars)",
        if to_inject.len() > 60 {
            &to_inject[..60]
        } else {
            &to_inject
        },
        to_inject.len()
    );

    // 6. Inject text (blocking — arboard + enigo are sync)
    match tauri::async_runtime::spawn_blocking(move || crate::inject::inject_text(&to_inject))
        .await
    {
        Ok(Ok(())) => {
            log::info!("Pipeline: injection complete");
            // Tray tooltip for development debugging: show last transcription
            if let Some(tray) = app.tray_by_id("tray") {
                let _ = tray.set_tooltip(Some(&format!("VoiceType — last: {}", formatted)));
            }
            // Pill: success flash before hide
            app.emit_to("pill", "pill-result", "success").ok();
        }
        Ok(Err(e)) => {
            log::error!("Pipeline: injection failed: {}", e);
            app.emit_to("pill", "pill-result", "error").ok();
        }
        Err(e) => {
            log::error!("Pipeline: injection panicked: {}", e);
            app.emit_to("pill", "pill-result", "error").ok();
        }
    }

    // 7. Reset to idle
    reset_to_idle(&app);
}

/// Cancel any active VAD worker in managed state.
///
/// Called at the top of run_pipeline() to ensure no stale VAD worker fires
/// a second pipeline trigger after the pipeline has already been entered
/// (e.g., second tap in toggle mode races with a silence-timeout).
fn cancel_stale_vad_worker(app: &tauri::AppHandle) {
    // Take the handle out under lock, then cancel it after releasing the guard.
    // The `let result = ...; result` pattern forces the MutexGuard temporary to
    // drop before `vad_state` goes out of scope (compiler E0597 fix).
    let maybe_handle: Option<crate::vad::VadWorkerHandle> = {
        let vad_state = app.state::<crate::VadWorkerState>();
        let result = match vad_state.0.lock() {
            Ok(mut guard) => guard.take(),
            Err(_) => None,
        };
        result
    };
    if let Some(mut handle) = maybe_handle {
        handle.cancel();
        log::info!("Pipeline: cancelled stale VAD worker");
    }
}

/// Reset pipeline state to IDLE and update tray icon.
///
/// Called from every exit path in run_pipeline — ensures no stuck states
/// regardless of error type (Pitfall 3 from RESEARCH.md).
fn reset_to_idle(app: &tauri::AppHandle) {
    app.state::<PipelineState>().reset_to_idle();
    crate::tray::set_tray_state(app, crate::tray::TrayState::Idle);
    if let Some(tray) = app.tray_by_id("tray") {
        let _ = tray.set_tooltip(Some("VoiceType — idle"));
    }
    // Pill: transition to idle and hide
    app.emit_to("pill", "pill-state", "idle").ok();
    app.emit_to("pill", "pill-hide", ()).ok();
}
