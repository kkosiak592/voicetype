use std::sync::atomic::{AtomicU8, Ordering};
use tauri::Emitter;
use tauri::Manager;

pub const IDLE: u8 = 0;
pub const RECORDING: u8 = 1;
pub const PROCESSING: u8 = 2;

/// AtomicU8-backed state machine for the hold-to-talk pipeline.
///
/// Transitions are guarded by compare_exchange to prevent concurrent recordings
/// or double-starts. Every exit path in run_pipeline must call reset_to_idle().
pub struct PipelineState(pub AtomicU8);

impl PipelineState {
    pub fn new() -> Self {
        PipelineState(AtomicU8::new(IDLE))
    }

    /// Attempt a CAS transition from `from` -> `to`. Returns true if successful.
    /// Returns false if the current state is not `from` (pipeline is busy).
    pub fn transition(&self, from: u8, to: u8) -> bool {
        self.0
            .compare_exchange(from, to, Ordering::SeqCst, Ordering::Relaxed)
            .is_ok()
    }

    pub fn set(&self, val: u8) {
        self.0.store(val, Ordering::SeqCst);
    }

    pub fn get(&self) -> u8 {
        self.0.load(Ordering::SeqCst)
    }
}

/// Core pipeline orchestration — called from the Released hotkey handler.
///
/// Steps:
///   1. Stop recording, get audio buffer
///   2. Minimum audio gate (< 100ms = 1600 samples at 16kHz → discard)
///   3. Whisper inference in spawn_blocking
///   4. Text formatting (trim_start + trailing space)
///   5. Inject text via clipboard paste
///   6. Reset to idle
///
/// Every early-return path calls reset_to_idle() — no stuck states.
pub async fn run_pipeline(app: tauri::AppHandle) {
    // 1. Stop recording and get audio buffer
    let audio_state = app.state::<crate::audio::AudioCapture>();
    let sample_count = audio_state.flush_and_stop();
    let samples = audio_state.get_buffer();

    // 2. Minimum audio gate: < 100ms at 16kHz = 1600 samples — discard silently
    if samples.len() < 1600 {
        log::info!(
            "Pipeline: audio too short ({} samples, {:.0}ms), discarding",
            samples.len(),
            samples.len() as f32 / 16.0
        );
        // Pill: error flash — user held hotkey but got no usable audio
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

    // 3. Run whisper inference (blocking — whisper-rs is sync)
    #[cfg(feature = "whisper")]
    let transcription: String = {
        let whisper_state = app.state::<crate::WhisperState>();
        let ctx = match &whisper_state.0 {
            Some(ctx) => ctx.clone(),
            None => {
                log::error!("Pipeline: whisper model not loaded");
                // Pill: error flash — model not available
                app.emit_to("pill", "pill-result", "error").ok();
                reset_to_idle(&app);
                return;
            }
        };
        match tauri::async_runtime::spawn_blocking(move || {
            crate::transcribe::transcribe_audio(&ctx, &samples)
        })
        .await
        {
            Ok(Ok(text)) => text,
            Ok(Err(e)) => {
                log::error!("Pipeline: whisper inference error: {}", e);
                // Pill: error flash — inference failed
                app.emit_to("pill", "pill-result", "error").ok();
                reset_to_idle(&app);
                return;
            }
            Err(e) => {
                log::error!("Pipeline: spawn_blocking panicked: {}", e);
                // Pill: error flash — spawn_blocking panicked
                app.emit_to("pill", "pill-result", "error").ok();
                reset_to_idle(&app);
                return;
            }
        }
    };

    // Without whisper feature, we can't transcribe — log and reset
    #[cfg(not(feature = "whisper"))]
    {
        log::warn!("Pipeline: whisper feature not enabled, cannot transcribe");
        // Pill: error flash — no transcription possible
        app.emit_to("pill", "pill-result", "error").ok();
        reset_to_idle(&app);
        return;
    }

    #[cfg(feature = "whisper")]
    {
        // 4. Text formatting per CONTEXT.md locked decisions:
        //    - Trim leading whitespace
        //    - Append trailing space for consecutive dictation bridging
        //    - Discard empty/whitespace-only results silently
        let trimmed = transcription.trim_start();
        if trimmed.is_empty() || trimmed.chars().all(|c| c.is_whitespace()) {
            log::info!("Pipeline: empty transcription, discarding");
            // Pill: error flash — no speech detected
            app.emit_to("pill", "pill-result", "error").ok();
            reset_to_idle(&app);
            return;
        }
        let to_inject = format!("{} ", trimmed); // trailing space per CONTEXT.md

        log::info!(
            "Pipeline: injecting '{}' ({} chars)",
            if to_inject.len() > 60 {
                &to_inject[..60]
            } else {
                &to_inject
            },
            to_inject.len()
        );

        // 5. Inject text (blocking — arboard + enigo are sync)
        match tauri::async_runtime::spawn_blocking(move || crate::inject::inject_text(&to_inject)).await {
            Ok(Ok(())) => {
                log::info!("Pipeline: injection complete");
                // Tray tooltip for development debugging: show last transcription
                if let Some(tray) = app.tray_by_id("tray") {
                    let _ = tray.set_tooltip(Some(&format!("VoiceType — last: {}", trimmed)));
                }
                // Pill: success flash before hide
                app.emit_to("pill", "pill-result", "success").ok();
            }
            Ok(Err(e)) => log::error!("Pipeline: injection failed: {}", e),
            Err(e) => log::error!("Pipeline: injection panicked: {}", e),
        }

        // 6. Reset to idle
        reset_to_idle(&app);
    }
}

/// Reset pipeline state to IDLE and update tray icon.
///
/// Called from every exit path in run_pipeline — ensures no stuck states
/// regardless of error type (Pitfall 3 from RESEARCH.md).
fn reset_to_idle(app: &tauri::AppHandle) {
    app.state::<PipelineState>().set(IDLE);
    crate::tray::set_tray_state(app, crate::tray::TrayState::Idle);
    if let Some(tray) = app.tray_by_id("tray") {
        let _ = tray.set_tooltip(Some("VoiceType — idle"));
    }
    // Pill: transition to idle and hide
    app.emit_to("pill", "pill-state", "idle").ok();
    app.emit_to("pill", "pill-hide", ()).ok();
}
