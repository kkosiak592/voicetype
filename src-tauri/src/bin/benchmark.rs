//! Standalone benchmark binary for whisper-rs and parakeet-rs models.
//!
//! Run from src-tauri/:
//!   cargo run --bin benchmark --features whisper,parakeet --release
//!
//! Models must be downloaded to %APPDATA%/VoiceType/models/ before benchmarking.
//! WAV fixtures must exist at test-fixtures/benchmark-5s.wav and benchmark-60s.wav.
//! Generate them with: powershell -ExecutionPolicy Bypass -File test-fixtures/generate-benchmark-wavs.ps1

use std::path::PathBuf;
use std::time::Instant;

#[cfg(feature = "whisper")]
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

#[cfg(feature = "parakeet")]
use parakeet_rs::{ExecutionConfig, ExecutionProvider, ParakeetTDT, TimestampMode};

// ---------------------------------------------------------------------------
// WAV reading
// ---------------------------------------------------------------------------

/// Read a WAV file to a f32 mono 16kHz sample vector.
///
/// Handles both Float and Int sample formats, downmixes multi-channel audio
/// to mono, and linearly resamples to 16000 Hz if the source differs.
fn read_wav_to_f32(path: &str) -> Result<Vec<f32>, String> {
    let mut reader = hound::WavReader::open(path)
        .map_err(|e| format!("Failed to open WAV file '{}': {}", path, e))?;

    let spec = reader.spec();
    println!(
        "  WAV: {}Hz, {} ch, {}bit {:?}",
        spec.sample_rate, spec.channels, spec.bits_per_sample, spec.sample_format
    );

    let channels = spec.channels as usize;

    // Decode samples → f32 mono
    let mono_f32: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => {
            let samples: Vec<f32> = reader
                .samples::<f32>()
                .collect::<Result<_, _>>()
                .map_err(|e| format!("Failed to read WAV samples: {}", e))?;
            samples
                .chunks(channels)
                .map(|ch| ch.iter().sum::<f32>() / channels as f32)
                .collect()
        }
        hound::SampleFormat::Int => {
            let max_val = (1i64 << (spec.bits_per_sample - 1)) as f32;
            let samples: Vec<i32> = reader
                .samples::<i32>()
                .collect::<Result<_, _>>()
                .map_err(|e| format!("Failed to read WAV samples: {}", e))?;
            samples
                .chunks(channels)
                .map(|ch| {
                    let mono_int: f32 = ch.iter().map(|&s| s as f32).sum::<f32>() / channels as f32;
                    mono_int / max_val
                })
                .collect()
        }
    };

    // Resample to 16kHz if needed (linear interpolation)
    let audio = if spec.sample_rate == 16000 {
        mono_f32
    } else {
        let src_rate = spec.sample_rate as f64;
        let dst_rate = 16000.0f64;
        let ratio = src_rate / dst_rate;
        let dst_len = (mono_f32.len() as f64 / ratio).ceil() as usize;
        let mut resampled = Vec::with_capacity(dst_len);
        for i in 0..dst_len {
            let src_pos = i as f64 * ratio;
            let src_idx = src_pos as usize;
            let frac = src_pos - src_idx as f64;
            let s0 = mono_f32.get(src_idx).copied().unwrap_or(0.0);
            let s1 = mono_f32.get(src_idx + 1).copied().unwrap_or(0.0);
            resampled.push(s0 + (s1 - s0) * frac as f32);
        }
        println!(
            "  Resampled {}Hz -> 16kHz: {} -> {} samples",
            spec.sample_rate,
            mono_f32.len(),
            resampled.len()
        );
        resampled
    };

    println!(
        "  Decoded: {} samples at 16kHz ({:.1}s)",
        audio.len(),
        audio.len() as f32 / 16000.0
    );
    Ok(audio)
}

// ---------------------------------------------------------------------------
// GPU detection
// ---------------------------------------------------------------------------

/// Returns (use_gpu_for_whisper, parakeet_provider).
/// NVIDIA → (true, "cuda"), otherwise → (false, "cpu").
#[cfg(feature = "whisper")]
fn detect_gpu() -> (bool, String) {
    use nvml_wrapper::Nvml;
    match Nvml::init() {
        Ok(nvml) => match nvml.device_by_index(0) {
            Ok(device) => {
                let name = device.name().unwrap_or_else(|_| "Unknown NVIDIA GPU".to_string());
                println!("GPU detected: {} (NVIDIA — CUDA mode)", name);
                (true, "cuda".to_string())
            }
            Err(e) => {
                println!("NVML init OK but no device at index 0: {} — CPU mode", e);
                (false, "cpu".to_string())
            }
        },
        Err(e) => {
            println!("NVML init failed (no NVIDIA GPU or drivers): {} — CPU mode", e);
            (false, "cpu".to_string())
        }
    }
}

// When only the parakeet feature is enabled, we still need GPU detection.
#[cfg(all(not(feature = "whisper"), feature = "parakeet"))]
fn detect_gpu() -> (bool, String) {
    // Without nvml-wrapper (tied to the whisper feature), default to cpu.
    println!("GPU detection skipped (whisper feature disabled) — using CPU for parakeet");
    (false, "cpu".to_string())
}

// ---------------------------------------------------------------------------
// WAV path discovery
// ---------------------------------------------------------------------------

fn find_wav(filename: &str) -> Option<String> {
    // Try relative to CWD first, then ../test-fixtures/ (when running from src-tauri/)
    let candidates = [
        format!("test-fixtures/{}", filename),
        format!("../test-fixtures/{}", filename),
    ];
    for path in &candidates {
        if std::path::Path::new(path).exists() {
            return Some(path.clone());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Benchmark result
// ---------------------------------------------------------------------------

struct BenchResult {
    model: String,
    clip: String,
    avg_ms: u64,
    min_ms: u64,
    max_ms: u64,
    first_text: String,
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    println!("==========================================================");
    println!("VoiceType Model Benchmark");
    println!("==========================================================");

    // GPU detection
    let (use_gpu, parakeet_provider) = {
        #[cfg(any(feature = "whisper", feature = "parakeet"))]
        { detect_gpu() }
        #[cfg(not(any(feature = "whisper", feature = "parakeet")))]
        { (false, "cpu".to_string()) }
    };

    // Models directory
    let appdata = match std::env::var("APPDATA") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("ERROR: APPDATA environment variable not set");
            std::process::exit(1);
        }
    };
    let models_dir = PathBuf::from(&appdata).join("VoiceType").join("models");
    println!("Models dir: {}", models_dir.display());

    // Model discovery
    let whisper_models: Vec<(&str, &str)> = vec![
        ("ggml-small.en-q5_1.bin",        "whisper-small-en"),
        ("ggml-large-v3-turbo-q5_0.bin",  "whisper-large-v3-turbo"),
        ("ggml-distil-large-v3.5.bin",    "whisper-distil-large-v3.5"),
    ];
    let parakeet_dir_name = "parakeet-tdt-v2-fp32";

    println!("\n-- Model availability --");
    let mut found_whisper: Vec<(PathBuf, &str)> = Vec::new();
    #[cfg(feature = "whisper")]
    for (filename, label) in &whisper_models {
        let path = models_dir.join(filename);
        if path.exists() {
            println!("  FOUND    {}", label);
            found_whisper.push((path, label));
        } else {
            println!("  MISSING  {} ({})", label, path.display());
        }
    }
    #[cfg(not(feature = "whisper"))]
    {
        let _ = whisper_models;
        println!("  (whisper feature disabled — skipping whisper models)");
    }

    let parakeet_path = models_dir.join(parakeet_dir_name);
    let parakeet_found = parakeet_path.exists() && parakeet_path.is_dir();
    #[cfg(feature = "parakeet")]
    if parakeet_found {
        println!("  FOUND    parakeet-tdt-v2");
    } else {
        println!("  MISSING  parakeet-tdt-v2 ({})", parakeet_path.display());
    }
    #[cfg(not(feature = "parakeet"))]
    println!("  (parakeet feature disabled — skipping parakeet model)");

    // WAV files
    println!("\n-- WAV fixtures --");
    let clips: Vec<(&str, &str)> = vec![
        ("benchmark-5s.wav",  "5s"),
        ("benchmark-60s.wav", "60s"),
    ];

    let mut clip_paths: Vec<(String, &str)> = Vec::new();
    for (filename, label) in &clips {
        match find_wav(filename) {
            Some(p) => {
                println!("  FOUND    {} -> {}", label, p);
                clip_paths.push((p, label));
            }
            None => {
                println!(
                    "  MISSING  {} (run: powershell -ExecutionPolicy Bypass -File test-fixtures/generate-benchmark-wavs.ps1)",
                    filename
                );
            }
        }
    }

    if clip_paths.is_empty() {
        eprintln!("\nERROR: No WAV clips found. Generate them first:");
        eprintln!("  powershell -ExecutionPolicy Bypass -File test-fixtures/generate-benchmark-wavs.ps1");
        std::process::exit(1);
    }

    // Run benchmarks
    let mut results: Vec<BenchResult> = Vec::new();
    const ITERATIONS: usize = 5;

    // -----------------------------------------------------------------------
    // Whisper models
    // -----------------------------------------------------------------------
    #[cfg(feature = "whisper")]
    for (model_path, model_label) in &found_whisper {
        println!("\n=== {} ===", model_label);

        // Load model once
        let load_start = Instant::now();
        let mut ctx_params = WhisperContextParameters::default();
        ctx_params.use_gpu(use_gpu);
        ctx_params.flash_attn(true);
        let ctx = match WhisperContext::new_with_params(
            &model_path.to_string_lossy(),
            ctx_params,
        ) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("  ERROR loading {}: {}", model_label, e);
                continue;
            }
        };
        println!("  Load time: {}ms", load_start.elapsed().as_millis());

        for (wav_path, clip_label) in &clip_paths {
            println!("  Clip: {}", clip_label);
            let audio = match read_wav_to_f32(wav_path) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("  ERROR reading WAV: {}", e);
                    continue;
                }
            };

            let mut latencies: Vec<u64> = Vec::with_capacity(ITERATIONS);
            let mut first_text = String::new();

            for i in 0..ITERATIONS {
                let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
                params.set_language(Some("en"));
                params.set_temperature(0.0);
                params.set_temperature_inc(0.0);
                params.set_single_segment(false);
                params.set_no_context(true);
                params.set_print_special(false);
                params.set_print_progress(false);
                params.set_print_realtime(false);
                params.set_print_timestamps(false);

                let mut state = match ctx.create_state() {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("  ERROR creating whisper state: {}", e);
                        break;
                    }
                };

                let t = Instant::now();
                match state.full(params, &audio) {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("  ERROR during inference run {}: {}", i + 1, e);
                        break;
                    }
                }
                let elapsed = t.elapsed().as_millis() as u64;
                latencies.push(elapsed);

                if i == 0 {
                    // Collect transcription text from first run
                    let n_seg = state.full_n_segments();
                    let mut text = String::new();
                    for s in 0..n_seg {
                        if let Some(segment) = state.get_segment(s) {
                            if let Ok(s_str) = segment.to_str() {
                                text.push_str(s_str.trim());
                                text.push(' ');
                            }
                        }
                    }
                    first_text = text.trim().to_string();
                    println!("  [run 1] {}ms — \"{}\"", elapsed, truncate(&first_text, 80));
                } else {
                    println!("  [run {}] {}ms", i + 1, elapsed);
                }
            }

            if latencies.is_empty() {
                continue;
            }
            let avg = latencies.iter().sum::<u64>() / latencies.len() as u64;
            let min = *latencies.iter().min().unwrap();
            let max = *latencies.iter().max().unwrap();
            println!("  -> avg={}ms  min={}ms  max={}ms", avg, min, max);

            results.push(BenchResult {
                model: model_label.to_string(),
                clip: clip_label.to_string(),
                avg_ms: avg,
                min_ms: min,
                max_ms: max,
                first_text,
            });
        }
    }

    // -----------------------------------------------------------------------
    // Parakeet model
    // -----------------------------------------------------------------------
    #[cfg(feature = "parakeet")]
    if parakeet_found {
        println!("\n=== parakeet-tdt-v2 (provider={}) ===", parakeet_provider);

        let load_start = Instant::now();
        let config = if parakeet_provider == "cuda" {
            println!("  Requesting CUDA ExecutionProvider");
            Some(ExecutionConfig::new().with_execution_provider(ExecutionProvider::Cuda))
        } else {
            println!("  Using CPU ExecutionProvider");
            None
        };

        let mut parakeet = match ParakeetTDT::from_pretrained(
            &*parakeet_path.to_string_lossy(),
            config,
        ) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("  ERROR loading parakeet: {}", e);
                print_summary(&results);
                return;
            }
        };
        println!("  Load time: {}ms", load_start.elapsed().as_millis());

        // Warm-up inference
        println!("  Warming up (8000 silent samples)...");
        let warmup_start = Instant::now();
        let silent: Vec<f32> = vec![0.0f32; 8000];
        let _ = parakeet.transcribe_samples(silent, 16000, 1, Some(TimestampMode::Sentences));
        println!("  Warm-up: {}ms", warmup_start.elapsed().as_millis());

        for (wav_path, clip_label) in &clip_paths {
            println!("  Clip: {}", clip_label);
            let audio = match read_wav_to_f32(wav_path) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("  ERROR reading WAV: {}", e);
                    continue;
                }
            };

            let mut latencies: Vec<u64> = Vec::with_capacity(ITERATIONS);
            let mut first_text = String::new();

            for i in 0..ITERATIONS {
                let t = Instant::now();
                match parakeet.transcribe_samples(
                    audio.clone(),
                    16000,
                    1,
                    Some(TimestampMode::Sentences),
                ) {
                    Ok(result) => {
                        let elapsed = t.elapsed().as_millis() as u64;
                        latencies.push(elapsed);
                        if i == 0 {
                            first_text = result.text.trim().to_string();
                            println!(
                                "  [run 1] {}ms — \"{}\"",
                                elapsed,
                                truncate(&first_text, 80)
                            );
                        } else {
                            println!("  [run {}] {}ms", i + 1, elapsed);
                        }
                    }
                    Err(e) => {
                        eprintln!("  ERROR during inference run {}: {}", i + 1, e);
                        break;
                    }
                }
            }

            if latencies.is_empty() {
                continue;
            }
            let avg = latencies.iter().sum::<u64>() / latencies.len() as u64;
            let min = *latencies.iter().min().unwrap();
            let max = *latencies.iter().max().unwrap();
            println!("  -> avg={}ms  min={}ms  max={}ms", avg, min, max);

            results.push(BenchResult {
                model: "parakeet-tdt-v2".to_string(),
                clip: clip_label.to_string(),
                avg_ms: avg,
                min_ms: min,
                max_ms: max,
                first_text,
            });
        }
    }

    print_summary(&results);
}

// ---------------------------------------------------------------------------
// Summary table
// ---------------------------------------------------------------------------

fn print_summary(results: &[BenchResult]) {
    println!("\n");
    println!("============================================================");
    println!("BENCHMARK RESULTS");
    println!("============================================================");
    if results.is_empty() {
        println!("No results collected (no models found or all failed).");
        return;
    }
    println!(
        "{:<30} | {:<4} | {:>8} | {:>8} | {:>8}",
        "Model", "Clip", "Avg (ms)", "Min (ms)", "Max (ms)"
    );
    println!("{}", "-".repeat(70));
    for r in results {
        println!(
            "{:<30} | {:<4} | {:>8} | {:>8} | {:>8}",
            r.model, r.clip, r.avg_ms, r.min_ms, r.max_ms
        );
    }
    println!("{}", "=".repeat(70));
    println!("Transcription samples (first run of each model/clip):");
    for r in results {
        if !r.first_text.is_empty() {
            println!("  [{} / {}] {}", r.model, r.clip, truncate(&r.first_text, 100));
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}
