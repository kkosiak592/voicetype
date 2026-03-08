// Moonshine Tiny ONNX batch transcription engine.
//
// Uses transcribe-rs MoonshineEngine (v1 batch, not streaming).
// The v1 batch engine (encoder_model.onnx + decoder_model_merged.onnx) is decisively
// faster than the streaming engine for hold-to-talk use cases (191ms vs 597ms on 5s clips).
//
// MoonshineEngine::transcribe_samples takes &mut self — not Sync.
// Callers wrap it in Arc<Mutex<MoonshineEngine>> (inner) + Mutex<Option<...>> (outer)
// following the exact ParakeetStateMutex pattern.

use std::time::Instant;
use transcribe_rs::engines::moonshine::{MoonshineEngine, MoonshineModelParams};
// TranscriptionEngine trait required to call load_model_with_params and transcribe_samples
use transcribe_rs::TranscriptionEngine as TranscribeRsEngine;

/// Loads a Moonshine Tiny model from a directory of ONNX files.
///
/// `model_dir` must be the path to the moonshine-tiny-ONNX/ directory, which contains:
///   - encoder_model.onnx      (~4 MB)
///   - decoder_model_merged.onnx (~104 MB)
///   - tokenizer.json
///
/// `provider` controls the ONNX execution provider:
/// - "cuda"     → CUDA EP (NVIDIA GPU); ort falls back to CPU if CUDA unavailable at runtime
/// - "directml" → CPU fallback for now (DirectML EP for Moonshine is an open question)
/// - anything else → CPU EP (default, no GPU acceleration)
///
/// Logs model load duration at INFO level.
pub fn load_moonshine(
    model_dir: &std::path::Path,
    provider: &str,
) -> Result<MoonshineEngine, String> {
    let start = Instant::now();

    log::info!(
        "Loading Moonshine Tiny model from: {} (provider={})",
        model_dir.display(),
        provider
    );

    let mut params = MoonshineModelParams::tiny();

    // Configure execution providers based on the detected GPU provider.
    // Access ort types through the ort crate (direct dependency in bench_extra)
    // or through dep:ort added to moonshine feature.
    params.execution_providers = match provider {
        "cuda" => {
            log::info!("Requesting CUDA ExecutionProvider for Moonshine");
            Some(vec![
                ort::execution_providers::CUDAExecutionProvider::default().build(),
                ort::execution_providers::CPUExecutionProvider::default().build(),
            ])
        }
        "directml" => {
            // DirectML EP for ONNX Moonshine — for now, fall back to CPU.
            // The DirectML EP path via ort is an open question for this model.
            log::info!("DirectML requested for Moonshine — falling back to CPU (DirectML EP not yet wired)");
            None
        }
        _ => {
            log::info!("Requesting CPU ExecutionProvider for Moonshine");
            None
        }
    };

    let mut engine = MoonshineEngine::new();
    TranscribeRsEngine::load_model_with_params(&mut engine, model_dir, params)
        .map_err(|e| format!("Failed to load Moonshine Tiny model from '{}': {}", model_dir.display(), e))?;

    let load_ms = start.elapsed().as_millis();
    log::info!("Moonshine Tiny model loaded in {}ms (provider={})", load_ms, provider);

    Ok(engine)
}

/// Runs a dummy inference with ~0.5s of silent audio (8000 zero samples at 16kHz)
/// to trigger CUDA context initialization and cuDNN algorithm selection.
/// The transcription result is discarded. Logs warm-up duration.
/// This should be called once after model loading, ideally in a background thread.
pub fn warm_up_moonshine(engine: &mut MoonshineEngine) {
    let start = Instant::now();
    // 0.5 seconds of silence at 16kHz = 8000 samples
    let silent_audio: Vec<f32> = vec![0.0f32; 8000];
    match TranscribeRsEngine::transcribe_samples(engine, silent_audio, None) {
        Ok(_) => {
            log::info!(
                "Moonshine warm-up completed in {}ms",
                start.elapsed().as_millis()
            );
        }
        Err(e) => {
            log::warn!("Moonshine warm-up inference failed (non-fatal): {}", e);
        }
    }
}

/// Transcribes a slice of 16 kHz mono f32 audio samples using Moonshine Tiny.
///
/// Moonshine Tiny is trained on 4-30 second audio segments. Audio longer than 30 seconds
/// MUST be split with VAD before inference — passing longer audio produces garbled output.
///
/// For audio > 30s: splits via `vad_chunk_audio` then concatenates chunk results.
/// For audio <= 30s: passes directly to the engine.
///
/// Returns the trimmed transcription text, or an error string on failure.
/// Logs transcription duration and the first 80 characters of the result.
pub fn transcribe_with_moonshine(
    engine: &mut MoonshineEngine,
    audio: &[f32],
) -> Result<String, String> {
    const MOONSHINE_MAX_SAMPLES: usize = 30 * 16000; // 30 seconds at 16kHz

    let start = Instant::now();

    let chunks: Vec<Vec<f32>> = if audio.len() > MOONSHINE_MAX_SAMPLES {
        log::info!(
            "Moonshine: audio {:.1}s exceeds 30s limit — splitting with VAD",
            audio.len() as f32 / 16000.0
        );
        crate::vad::vad_chunk_audio(audio, 30)
    } else {
        vec![audio.to_vec()]
    };

    let mut combined_text = String::new();
    for chunk in &chunks {
        match TranscribeRsEngine::transcribe_samples(engine, chunk.clone(), None) {
            Ok(result) => {
                let trimmed = result.text.trim();
                if !trimmed.is_empty() {
                    if !combined_text.is_empty() {
                        combined_text.push(' ');
                    }
                    combined_text.push_str(trimmed);
                }
            }
            Err(e) => return Err(format!("Moonshine inference error: {}", e)),
        }
    }

    let text = combined_text.trim().to_string();

    log::info!(
        "Moonshine transcription completed in {}ms ({} chunks): '{}'",
        start.elapsed().as_millis(),
        chunks.len(),
        if text.len() > 80 {
            format!("{}...", &text[..80])
        } else {
            text.clone()
        }
    );

    Ok(text)
}
