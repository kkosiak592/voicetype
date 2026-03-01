use std::path::PathBuf;
use std::time::Instant;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Inference mode: GPU (CUDA) or CPU fallback.
///
/// Determined once at startup via `detect_gpu()`. Controls model selection and
/// whisper context parameters (use_gpu true/false).
#[derive(Debug, Clone, Copy)]
pub enum ModelMode {
    Gpu,
    Cpu,
}

/// Returns the path to the VoiceType models directory in APPDATA.
pub fn models_dir() -> PathBuf {
    let appdata = std::env::var("APPDATA").expect("APPDATA environment variable not set");
    PathBuf::from(appdata).join("VoiceType").join("models")
}

/// Detects whether an NVIDIA GPU is available at runtime using NVML.
///
/// Returns ModelMode::Gpu if an NVIDIA GPU is found, ModelMode::Cpu otherwise.
/// Logs the detection result in both cases.
///
/// NVML is the NVIDIA Management Library — it's available whenever NVIDIA drivers
/// are installed, independent of CUDA. This allows runtime detection even when
/// shipping a single CUDA-enabled binary.
pub fn detect_gpu() -> ModelMode {
    use nvml_wrapper::Nvml;

    match Nvml::init() {
        Ok(nvml) => match nvml.device_by_index(0) {
            Ok(device) => {
                let name = device.name().unwrap_or_else(|_| "Unknown NVIDIA GPU".to_string());
                log::info!("NVIDIA GPU detected: {}", name);
                ModelMode::Gpu
            }
            Err(e) => {
                log::info!("NVML init OK but no device at index 0: {} — using CPU mode", e);
                ModelMode::Cpu
            }
        },
        Err(e) => {
            log::info!("NVML init failed (no NVIDIA GPU or drivers not installed): {} — using CPU mode", e);
            ModelMode::Cpu
        }
    }
}

/// Resolves and validates the model path for the given inference mode.
///
/// - Gpu: ggml-large-v3-turbo-q5_0.bin (large, fast GPU model)
/// - Cpu: ggml-small.en-q5_1.bin (small, runs acceptably on CPU)
///
/// Returns a detailed error with download instructions if the file is missing.
pub fn resolve_model_path(mode: &ModelMode) -> Result<PathBuf, String> {
    let (filename, download_url) = match mode {
        ModelMode::Gpu => (
            "ggml-large-v3-turbo-q5_0.bin",
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin",
        ),
        ModelMode::Cpu => (
            "ggml-small.en-q5_1.bin",
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en-q5_1.bin",
        ),
    };

    let path = models_dir().join(filename);

    log::info!(
        "Model selection: {:?} mode — looking for '{}'",
        mode,
        filename
    );

    if !path.exists() {
        return Err(format!(
            "Whisper {:?} model not found at: {}\n\
            \n\
            Download it with PowerShell:\n\
            Invoke-WebRequest -Uri '{}' \
            -OutFile \"$env:APPDATA\\VoiceType\\models\\{}\"\n\
            \n\
            Expected directory: {}",
            mode,
            path.display(),
            download_url,
            filename,
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

/// Loads a WhisperContext from the given model path.
///
/// GPU mode enables CUDA acceleration; CPU mode forces software inference.
/// Flash attention is enabled for faster self-attention computation.
/// Logs model load duration and GPU status.
pub fn load_whisper_context(model_path: &str, mode: &ModelMode) -> Result<WhisperContext, String> {
    let start = Instant::now();
    let use_gpu = matches!(mode, ModelMode::Gpu);

    log::info!(
        "Loading whisper model: {} (GPU={})",
        model_path,
        use_gpu
    );

    let mut ctx_params = WhisperContextParameters::default();
    ctx_params.use_gpu(use_gpu);
    ctx_params.flash_attn(true);

    let ctx = WhisperContext::new_with_params(model_path, ctx_params)
        .map_err(|e| format!("Failed to load whisper model from '{}': {}", model_path, e))?;

    log::info!(
        "Whisper model loaded in {}ms (GPU={})",
        start.elapsed().as_millis(),
        use_gpu
    );

    Ok(ctx)
}

/// Transcribes a slice of 16 kHz mono f32 audio samples using the provided WhisperContext.
///
/// Uses greedy decoding (best_of=1) with forced English and temperature 0.0. Greedy is
/// 30-50% faster than beam search (beam_size=5) with negligible accuracy loss for short
/// dictation phrases. Single-segment mode is forced since hold-to-talk clips are always
/// one utterance. A fresh WhisperState is created per call — this is thread-safe and the
/// recommended approach (see RESEARCH.md Pitfall 6).
///
/// Returns the trimmed transcription text, or an error string on failure.
pub fn transcribe_audio(ctx: &WhisperContext, audio: &[f32]) -> Result<String, String> {
    let start = Instant::now();

    let mut state = ctx.create_state().map_err(|e| e.to_string())?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some("en"));
    params.set_temperature(0.0);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_single_segment(true);   // short dictation = one segment
    params.set_no_context(true);       // no prior context carryover
    params.set_temperature_inc(0.0);   // disable temperature fallback retries

    state.full(params, audio).map_err(|e| e.to_string())?;

    let n_segments = state.full_n_segments();
    let mut text = String::new();
    for i in 0..n_segments {
        if let Some(segment) = state.get_segment(i) {
            text.push_str(segment.to_str().map_err(|e| e.to_string())?);
        }
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
