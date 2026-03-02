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

/// Extended GPU detection result for provider selection and UI display.
#[derive(Debug, Clone)]
pub struct GpuDetection {
    /// Human-readable GPU name (e.g., "NVIDIA Quadro P2000", "DirectML (auto-detected)")
    pub gpu_name: String,
    /// Which execution provider to use for Parakeet: "cuda", "directml", or "cpu"
    pub parakeet_provider: String,
    /// Whether this is an NVIDIA GPU (for Whisper CUDA and ModelMode::Gpu)
    pub is_nvidia: bool,
    /// Whether a discrete (dedicated) GPU with >512 MB VRAM was detected via DXGI.
    /// True for NVIDIA GPUs (always discrete) and discrete AMD/Intel Arc GPUs.
    /// False for integrated-only GPUs (Intel UHD, AMD APU) or when DXGI fails.
    pub has_discrete_gpu: bool,
}

/// Detects whether a discrete GPU with sufficient VRAM (>512 MB dedicated) exists via DXGI.
///
/// Enumerates DXGI adapters and returns true if any non-software adapter has
/// DedicatedVideoMemory > 512 MB. This distinguishes discrete GPUs (AMD RX, Intel Arc)
/// from integrated-only GPUs (Intel UHD, AMD APU) which share system RAM.
///
/// Returns false on any DXGI error (safe fallback — recommends small-en).
/// Non-Windows builds always return false.
#[cfg(target_os = "windows")]
pub fn has_discrete_gpu() -> bool {
    use windows::Win32::Graphics::Dxgi::{
        CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE,
    };

    let factory: IDXGIFactory1 = match unsafe { CreateDXGIFactory1() } {
        Ok(f) => f,
        Err(e) => {
            log::warn!("DXGI: CreateDXGIFactory1 failed: {} — assuming no discrete GPU", e);
            return false;
        }
    };

    let mut i = 0u32;
    loop {
        let adapter = match unsafe { factory.EnumAdapters1(i) } {
            Ok(a) => a,
            Err(_) => break, // No more adapters
        };

        let desc = match unsafe { adapter.GetDesc1() } {
            Ok(d) => d,
            Err(e) => {
                log::warn!("DXGI: GetDesc1 failed for adapter {}: {}", i, e);
                i += 1;
                continue;
            }
        };

        // Convert wide string description to a Rust String for logging
        let name_wide = &desc.Description;
        let name_len = name_wide.iter().position(|&c| c == 0).unwrap_or(name_wide.len());
        let name = String::from_utf16_lossy(&name_wide[..name_len]);

        let dedicated_vram_mb = desc.DedicatedVideoMemory / (1024 * 1024);
        let is_software = (desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32) != 0;

        log::info!(
            "DXGI adapter {}: '{}' — dedicated VRAM: {} MB — software: {}",
            i,
            name,
            dedicated_vram_mb,
            is_software
        );

        if !is_software && desc.DedicatedVideoMemory > 512 * 1024 * 1024 {
            log::info!("DXGI: Discrete GPU found: '{}' ({} MB VRAM)", name, dedicated_vram_mb);
            return true;
        }

        i += 1;
    }

    log::info!("DXGI: No discrete GPU with >512 MB VRAM found — integrated-only or no GPU");
    false
}

#[cfg(not(target_os = "windows"))]
pub fn has_discrete_gpu() -> bool {
    false
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

/// Full GPU detection returning a `GpuDetection` with GPU name, Parakeet provider recommendation,
/// NVIDIA flag, and discrete GPU flag. On NVIDIA: provider="cuda", has_discrete_gpu=true.
/// On non-NVIDIA: DXGI enumeration determines has_discrete_gpu; provider="directml" if discrete,
/// "cpu" if integrated-only.
///
/// Used at startup to populate `CachedGpuDetection` for provider selection and UI display.
pub fn detect_gpu_full() -> GpuDetection {
    use nvml_wrapper::Nvml;
    match Nvml::init() {
        Ok(nvml) => match nvml.device_by_index(0) {
            Ok(device) => {
                let name = device.name().unwrap_or_else(|_| "Unknown NVIDIA GPU".to_string());
                log::info!("GPU detection (full): NVIDIA GPU found: {}", name);
                GpuDetection {
                    gpu_name: name,
                    parakeet_provider: "cuda".to_string(),
                    is_nvidia: true,
                    has_discrete_gpu: true, // NVIDIA GPUs are always discrete
                }
            }
            Err(e) => {
                log::info!("GPU detection (full): NVML init OK but no device: {} — checking DXGI for discrete GPU", e);
                let discrete = has_discrete_gpu();
                GpuDetection {
                    gpu_name: if discrete {
                        "DirectML (auto-detected)".to_string()
                    } else {
                        "Integrated GPU".to_string()
                    },
                    parakeet_provider: if discrete { "directml".to_string() } else { "cpu".to_string() },
                    is_nvidia: false,
                    has_discrete_gpu: discrete,
                }
            }
        },
        Err(e) => {
            log::info!("GPU detection (full): NVML failed: {} — checking DXGI for discrete GPU", e);
            let discrete = has_discrete_gpu();
            GpuDetection {
                gpu_name: if discrete {
                    "DirectML (auto-detected)".to_string()
                } else {
                    "Integrated GPU".to_string()
                },
                parakeet_provider: if discrete { "directml".to_string() } else { "cpu".to_string() },
                is_nvidia: false,
                has_discrete_gpu: discrete,
            }
        }
    }
}

/// Resolves and validates the model path for the given inference mode.
///
/// - Gpu: ggml-large-v3-turbo-q5_0.bin (large, fast GPU model)
/// - Cpu: ggml-small.en-q5_1.bin (small, runs acceptably on CPU)
///
/// Returns Err with a user-friendly message if the file is missing (app handles download).
pub fn resolve_model_path(mode: &ModelMode) -> Result<PathBuf, String> {
    let filename = match mode {
        ModelMode::Gpu => "ggml-large-v3-turbo-q5_0.bin",
        ModelMode::Cpu => "ggml-small.en-q5_1.bin",
    };

    let path = models_dir().join(filename);

    log::info!(
        "Model selection: {:?} mode — looking for '{}'",
        mode,
        filename
    );

    if !path.exists() {
        return Err(format!(
            "Model file not found: {}. Use the app's Model settings to download.",
            path.display()
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
/// `initial_prompt`: Domain-specific vocabulary prompt for whisper. When non-empty, it is
/// injected via `set_initial_prompt` and `set_no_context(false)` — whisper uses the prompt
/// as a prior to bias transcription toward domain terminology. When empty, `set_no_context(true)`
/// is preserved to avoid context carryover between recordings.
///
/// Returns the trimmed transcription text, or an error string on failure.
pub fn transcribe_audio(ctx: &WhisperContext, audio: &[f32], initial_prompt: &str) -> Result<String, String> {
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
    params.set_temperature_inc(0.0);   // disable temperature fallback retries

    // When initial_prompt is set, whisper uses it as a prior for domain vocabulary.
    // CRITICAL: set_no_context(true) suppresses initial_prompt — must be false when prompt is used.
    // When prompt is empty, enable no_context to prevent context carryover between recordings.
    if !initial_prompt.is_empty() {
        params.set_initial_prompt(initial_prompt);
        params.set_no_context(false);
    } else {
        params.set_no_context(true);   // no prior context carryover
    }

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
