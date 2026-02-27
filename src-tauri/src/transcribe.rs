use std::path::PathBuf;
use std::time::Instant;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Returns the path to the VoiceType models directory in APPDATA.
pub fn models_dir() -> PathBuf {
    let appdata = std::env::var("APPDATA").expect("APPDATA environment variable not set");
    PathBuf::from(appdata).join("VoiceType").join("models")
}

/// Resolves and validates the GPU model path.
///
/// Returns the path if the model file exists. Returns a detailed error with download
/// instructions if the file is missing.
pub fn resolve_model_path() -> Result<PathBuf, String> {
    let path = models_dir().join("ggml-large-v3-turbo-q5_0.bin");

    if !path.exists() {
        return Err(format!(
            "Whisper GPU model not found at: {}\n\
            \n\
            Download it with PowerShell:\n\
            Invoke-WebRequest -Uri 'https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin' \
            -OutFile \"$env:APPDATA\\VoiceType\\models\\ggml-large-v3-turbo-q5_0.bin\"\n\
            \n\
            Expected directory: {}",
            path.display(),
            models_dir().display()
        ));
    }

    let metadata = std::fs::metadata(&path).map_err(|e| e.to_string())?;
    log::info!(
        "Whisper model found: {} ({:.1} MB)",
        path.display(),
        metadata.len() as f64 / 1_048_576.0
    );

    Ok(path)
}

/// Loads a WhisperContext from the given model path with GPU acceleration enabled.
///
/// Logs model load duration and GPU status. Returns an error if the context cannot
/// be created (e.g. CUDA not available or model file corrupted).
pub fn load_whisper_context(model_path: &str) -> Result<WhisperContext, String> {
    let start = Instant::now();

    let mut ctx_params = WhisperContextParameters::default();
    ctx_params.use_gpu(true);

    let ctx = WhisperContext::new_with_params(model_path, ctx_params)
        .map_err(|e| format!("Failed to load whisper model from '{}': {}", model_path, e))?;

    log::info!(
        "Whisper model loaded from '{}' with GPU enabled — {}ms",
        model_path,
        start.elapsed().as_millis()
    );

    Ok(ctx)
}

/// Transcribes a slice of 16 kHz mono f32 audio samples using the provided WhisperContext.
///
/// Uses beam search (beam_size=5) with forced English and temperature 0.0.
/// A fresh WhisperState is created per call — this is thread-safe and the recommended
/// approach (see RESEARCH.md Pitfall 6).
///
/// Returns the trimmed transcription text, or an error string on failure.
pub fn transcribe_audio(ctx: &WhisperContext, audio: &[f32]) -> Result<String, String> {
    let start = Instant::now();

    let mut state = ctx.create_state().map_err(|e| e.to_string())?;

    let mut params = FullParams::new(SamplingStrategy::BeamSearch {
        beam_size: 5,
        patience: -1.0,
    });
    params.set_language(Some("en"));
    params.set_temperature(0.0);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    state.full(params, audio).map_err(|e| e.to_string())?;

    let n_segments = state.full_n_segments().map_err(|e| e.to_string())?;
    let mut text = String::new();
    for i in 0..n_segments {
        text.push_str(
            &state
                .full_get_segment_text(i)
                .map_err(|e| e.to_string())?,
        );
    }

    let result = text.trim().to_string();
    log::info!(
        "Transcription completed in {}ms: '{}'",
        start.elapsed().as_millis(),
        if result.len() > 80 {
            format!("{}...", &result[..80])
        } else {
            result.clone()
        }
    );

    Ok(result)
}
