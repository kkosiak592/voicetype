// Thread safety note: ParakeetTDT::transcribe_samples takes &mut self in parakeet-rs 0.1.x,
// so ParakeetTDT is not Sync. Plan 02 wraps it in Arc<Mutex<ParakeetTDT>> for Tauri managed
// state, which serialises inference calls (acceptable: dictation is single-user).
//
// API version: parakeet-rs 0.1.9 — uses ort "^2.0.0-rc.10" (matches voice_activity_detector).
// 0.3.x was not used because it requires ort "^2.0.0-rc.11", conflicting with the VAD crate.

use parakeet_rs::{ExecutionConfig, ExecutionProvider, ParakeetTDT, TimestampMode};
use std::time::Instant;

/// Loads a Parakeet TDT model from a directory of ONNX files.
///
/// `model_dir` must contain the encoder, decoder_joint, vocab, and config files
/// from the `istupakov/parakeet-tdt-0.6b-v2-onnx` HuggingFace repo (fp32 variant).
///
/// `provider` controls the ONNX execution provider:
/// - "cuda"    → CUDA EP (NVIDIA GPU); ort falls back to CPU if CUDA unavailable at runtime
/// - "directml"→ DirectML EP (any DirectX 12 GPU: Intel/AMD/NVIDIA on Windows)
/// - anything else → CPU EP (default, no GPU acceleration)
///
/// Logs model load duration at INFO level.
pub fn load_parakeet(model_dir: &str, provider: &str) -> Result<ParakeetTDT, String> {
    let start = Instant::now();

    log::info!("Loading Parakeet TDT model from: {} (provider={})", model_dir, provider);

    let config = match provider {
        "cuda" => {
            log::info!("Requesting CUDA ExecutionProvider for Parakeet TDT");
            Some(ExecutionConfig::new().with_execution_provider(ExecutionProvider::Cuda))
        }
        "directml" => {
            log::info!("Requesting DirectML ExecutionProvider for Parakeet TDT");
            Some(ExecutionConfig::new().with_execution_provider(ExecutionProvider::DirectML))
        }
        _ => {
            log::info!("Requesting CPU ExecutionProvider for Parakeet TDT");
            None // CPU ExecutionProvider (default)
        }
    };

    let parakeet = ParakeetTDT::from_pretrained(model_dir, config)
        .map_err(|e| format!("Failed to load Parakeet TDT model from '{}': {}", model_dir, e))?;

    let load_ms = start.elapsed().as_millis();
    log::info!("Parakeet TDT model loaded in {}ms (provider={})", load_ms, provider);

    match provider {
        "cuda" => log::info!(
            "Parakeet TDT EP: CUDA requested — if load took >3s, CUDA likely initialized; \
             confirm with first inference (<200ms = GPU, >800ms = CPU fallback)"
        ),
        "directml" => log::info!("Parakeet TDT EP: DirectML"),
        _ => log::info!("Parakeet TDT EP: CPU"),
    }

    Ok(parakeet)
}

/// Runs a dummy inference with ~0.5s of silent audio (8000 zero samples at 16kHz)
/// to trigger CUDA context initialization, cudaMalloc, and cuDNN algorithm selection.
/// The transcription result is discarded. Logs warm-up duration.
/// This should be called once after model loading, ideally in a background thread.
pub fn warm_up_parakeet(parakeet: &mut ParakeetTDT) {
    let start = Instant::now();
    // 0.5 seconds of silence at 16kHz = 8000 samples
    let silent_audio: Vec<f32> = vec![0.0f32; 8000];
    match parakeet.transcribe_samples(silent_audio, 16000, 1, Some(TimestampMode::Sentences)) {
        Ok(_) => {
            log::info!(
                "Parakeet warm-up completed in {}ms (CUDA context + cuDNN initialized)",
                start.elapsed().as_millis()
            );
        }
        Err(e) => {
            log::warn!("Parakeet warm-up inference failed (non-fatal): {}", e);
        }
    }
}

/// Transcribes a slice of 16 kHz mono f32 audio samples using Parakeet TDT.
///
/// Audio is cloned into a Vec<f32> because parakeet-rs takes ownership.
/// Sample rate is fixed at 16000 Hz, channels at 1 (mono).
///
/// Note: `parakeet` requires `&mut self` — callers must hold a mutable reference
/// or use Mutex<ParakeetTDT> for concurrent access.
///
/// Uses TimestampMode::Sentences to enable word-level deduplication (strips repeated tokens).
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
        .transcribe_samples(audio_vec, 16000, 1, Some(TimestampMode::Sentences))
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
