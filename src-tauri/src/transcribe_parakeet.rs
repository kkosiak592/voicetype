// Thread safety note: ParakeetTDT::transcribe_samples takes &mut self in parakeet-rs 0.1.x,
// so ParakeetTDT is not Sync. Plan 02 wraps it in Arc<Mutex<ParakeetTDT>> for Tauri managed
// state, which serialises inference calls (acceptable: dictation is single-user).
//
// API version: parakeet-rs 0.1.9 — uses ort "^2.0.0-rc.10" (matches voice_activity_detector).
// 0.3.x was not used because it requires ort "^2.0.0-rc.11", conflicting with the VAD crate.

use parakeet_rs::{ExecutionConfig, ExecutionProvider, ParakeetTDT};
use std::time::Instant;

/// Loads a Parakeet TDT model from a directory of ONNX files.
///
/// `model_dir` must contain the encoder, decoder_joint, vocab, and config files
/// from the `istupakov/parakeet-tdt-0.6b-v2-onnx` HuggingFace repo (int8 variant).
///
/// `use_cuda`: true → CUDA execution provider (GPU); false → CPU execution provider.
///
/// Logs model load duration at INFO level.
pub fn load_parakeet(model_dir: &str, use_cuda: bool) -> Result<ParakeetTDT, String> {
    let start = Instant::now();

    log::info!(
        "Loading Parakeet TDT model from: {} (CUDA={})",
        model_dir,
        use_cuda
    );

    let config = if use_cuda {
        Some(
            ExecutionConfig::new()
                .with_execution_provider(ExecutionProvider::Cuda),
        )
    } else {
        // None → parakeet-rs defaults to CPU ExecutionProvider
        None
    };

    let parakeet = ParakeetTDT::from_pretrained(model_dir, config)
        .map_err(|e| format!("Failed to load Parakeet TDT model from '{}': {}", model_dir, e))?;

    log::info!(
        "Parakeet TDT model loaded in {}ms (CUDA={})",
        start.elapsed().as_millis(),
        use_cuda
    );

    Ok(parakeet)
}

/// Transcribes a slice of 16 kHz mono f32 audio samples using Parakeet TDT.
///
/// Audio is cloned into a Vec<f32> because parakeet-rs takes ownership.
/// Sample rate is fixed at 16000 Hz, channels at 1 (mono).
///
/// Note: `parakeet` requires `&mut self` — callers must hold a mutable reference
/// or use Mutex<ParakeetTDT> for concurrent access.
///
/// Returns the trimmed transcription text, or an error string on failure.
/// Logs transcription duration and the first 80 characters of the result.
pub fn transcribe_with_parakeet(
    parakeet: &mut ParakeetTDT,
    audio: &[f32],
) -> Result<String, String> {
    let start = Instant::now();

    // parakeet-rs 0.1.x takes an owned Vec<f32>
    let audio_vec: Vec<f32> = audio.to_vec();

    let result = parakeet
        .transcribe_samples(audio_vec, 16000, 1, None)
        .map_err(|e| format!("Parakeet transcription error: {}", e))?;

    let text = result.text.trim().to_string();

    log::info!(
        "Parakeet transcription completed in {}ms: '{}'",
        start.elapsed().as_millis(),
        if text.len() > 80 {
            format!("{}...", &text[..80])
        } else {
            text.clone()
        }
    );

    Ok(text)
}
