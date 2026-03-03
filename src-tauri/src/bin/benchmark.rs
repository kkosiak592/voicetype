//! Standalone benchmark binary for whisper-rs and parakeet-rs models.
//!
//! Run from src-tauri/:
//!   cargo run --bin benchmark --features whisper,parakeet --release
//!
//! Models must be downloaded to %APPDATA%/VoiceType/models/ before benchmarking.
//! WAV fixtures: 9 files expected (3 durations x 3 variants: benchmark-{5s,30s,60s}{,-b,-c}.wav).
//! Generate them with: powershell -ExecutionPolicy Bypass -File test-fixtures/generate-benchmark-wavs.ps1

use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

#[cfg(feature = "whisper")]
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

#[cfg(feature = "parakeet")]
use parakeet_rs::{ExecutionConfig, ExecutionProvider, ParakeetTDT, TimestampMode};

#[cfg(feature = "bench_extra")]
use transcribe_rs::{
    TranscriptionEngine,
    engines::moonshine::{MoonshineEngine, MoonshineModelParams},
    engines::sense_voice::{SenseVoiceEngine, SenseVoiceModelParams},
};

#[cfg(feature = "bench_extra")]
use ort::execution_providers::{CUDAExecutionProvider, CPUExecutionProvider};

#[cfg(feature = "bench_extra")]
use voice_activity_detector::VoiceActivityDetector;

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
// VAD-based audio chunking (bench_extra only)
// ---------------------------------------------------------------------------

/// Split audio into VAD-based chunks for models with short context windows.
/// Only called for clips > 30s. Returns Vec of audio segments split at silence boundaries.
///
/// Algorithm:
/// 1. Run Silero VAD over entire audio in 512-sample chunks
/// 2. Track speech/silence state -- silence starts when prob < 0.5
/// 3. When silence exceeds SILENCE_SPLIT_CHUNKS (~320ms), end current segment
/// 4. Cap segments at MAX_SEGMENT_SAMPLES (30s = 480000 samples)
/// 5. Discard segments shorter than MIN_SEGMENT_SAMPLES (0.5s = 8000 samples)
#[cfg(feature = "bench_extra")]
fn vad_chunk_audio(samples: &[f32]) -> Vec<Vec<f32>> {
    const CHUNK_SIZE: usize = 512;
    const SPEECH_THRESHOLD: f32 = 0.5;
    // 300ms at 16kHz = 4800 samples. At 512 samples/chunk = ~9.4 chunks. Round to 10.
    const SILENCE_SPLIT_CHUNKS: usize = 10; // ~320ms of silence triggers a split
    const MAX_SEGMENT_SAMPLES: usize = 30 * 16000; // 30 seconds
    const MIN_SEGMENT_SAMPLES: usize = 8000; // 0.5 seconds

    let mut vad = match VoiceActivityDetector::builder()
        .sample_rate(16000u32)
        .chunk_size(CHUNK_SIZE)
        .build()
    {
        Ok(v) => v,
        Err(e) => {
            eprintln!("  VAD chunking failed to init: {} -- returning single segment", e);
            return vec![samples.to_vec()];
        }
    };

    // Classify each chunk as speech or silence
    let num_chunks = samples.len() / CHUNK_SIZE;
    let mut is_speech: Vec<bool> = Vec::with_capacity(num_chunks);
    for i in 0..num_chunks {
        let start = i * CHUNK_SIZE;
        let chunk = &samples[start..start + CHUNK_SIZE];
        let prob = vad.predict(chunk.to_vec());
        is_speech.push(prob >= SPEECH_THRESHOLD);
    }

    // Find split points: runs of >= SILENCE_SPLIT_CHUNKS consecutive silence chunks
    let mut segments: Vec<Vec<f32>> = Vec::new();
    let mut seg_start_chunk: usize = 0;
    let mut silence_run: usize = 0;

    for (i, &speech) in is_speech.iter().enumerate() {
        if !speech {
            silence_run += 1;
        } else {
            silence_run = 0;
        }

        let seg_len_samples = (i + 1 - seg_start_chunk) * CHUNK_SIZE;

        // Split if: silence gap reached OR segment exceeds max duration
        let should_split = (silence_run >= SILENCE_SPLIT_CHUNKS && seg_len_samples > MIN_SEGMENT_SAMPLES)
            || seg_len_samples >= MAX_SEGMENT_SAMPLES;

        if should_split && i + 1 < num_chunks {
            // End segment at the start of the silence run (keep speech, trim trailing silence)
            let split_chunk = if silence_run >= SILENCE_SPLIT_CHUNKS {
                i + 1 - silence_run // start of silence run
            } else {
                i + 1 // max length reached -- split here
            };

            let start_sample = seg_start_chunk * CHUNK_SIZE;
            let end_sample = std::cmp::min(split_chunk * CHUNK_SIZE, samples.len());

            if end_sample > start_sample && (end_sample - start_sample) >= MIN_SEGMENT_SAMPLES {
                segments.push(samples[start_sample..end_sample].to_vec());
            }

            seg_start_chunk = i + 1; // next chunk starts new segment
            silence_run = 0;
        }
    }

    // Final segment
    let start_sample = seg_start_chunk * CHUNK_SIZE;
    if start_sample < samples.len() {
        let remaining = &samples[start_sample..];
        if remaining.len() >= MIN_SEGMENT_SAMPLES {
            segments.push(remaining.to_vec());
        }
    }

    // Fallback: if chunking produced nothing, return the whole audio
    if segments.is_empty() {
        segments.push(samples.to_vec());
    }

    println!("  VAD chunking: {} segments from {:.1}s audio",
        segments.len(),
        samples.len() as f32 / 16000.0);
    for (i, seg) in segments.iter().enumerate() {
        println!("    segment {}: {:.1}s ({} samples)", i, seg.len() as f32 / 16000.0, seg.len());
    }

    segments
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
// Reference transcripts (must match generate-benchmark-wavs.ps1 exactly)
// ---------------------------------------------------------------------------

const REF_5S: &str = "The quick brown fox jumps over the lazy dog. Pack my box with five dozen liquor jugs.";

const REF_30S: &str = "Speech recognition technology has advanced significantly in recent years. \
Modern deep learning models can transcribe audio with remarkable accuracy. \
The key factors that affect performance include microphone quality and background noise. \
Models trained on large datasets tend to generalize better across different speakers. \
Quantized models offer a good balance between speed and accuracy for real time use. \
This thirty second clip tests how models handle medium length audio segments.";

const REF_60S: &str = "Voice dictation software converts spoken words into written text in real time. \
Modern systems use deep learning models trained on thousands of hours of audio data. \
Accuracy depends on microphone quality, background noise, and speaking pace. \
The whisper model was released by OpenAI and is widely used for offline transcription. \
Parakeet is an NVIDIA model optimised for real-time inference on CUDA hardware. \
To benchmark these models fairly, we measure wall-clock latency across multiple runs. \
We test both a short five-second clip and a longer sixty-second passage. \
Results include the average, minimum, and maximum inference time in milliseconds. \
Lower latency means faster transcription and a better user experience. \
Sub five hundred millisecond latency is generally imperceptible to the user. \
English language models tend to be smaller and faster than multilingual alternatives. \
Quantised models use reduced precision weights to run faster with minimal accuracy loss. \
The Q5 underscore 1 format stores each weight in approximately five bits. \
GPU acceleration can reduce inference time by ten times compared to CPU-only execution. \
This benchmark helps select the best model for a given hardware configuration.";

const REF_5S_B: &str = "A copper wire carries electrical current through the circuit board with minimal resistance.";

const REF_5S_C: &str = "The satellite orbits Earth every ninety minutes, capturing high resolution photographs.";

const REF_30S_B: &str = "The process of steel manufacturing begins with iron ore extraction from open pit mines. \
Workers transport the raw material to blast furnaces where temperatures exceed fifteen hundred degrees. \
Carbon is introduced to create an alloy stronger than pure iron alone. \
Rolling mills then shape the molten steel into beams, sheets, and coiled wire. \
Quality control inspectors test samples for tensile strength and corrosion resistance. \
Modern foundries produce over two billion tonnes of steel worldwide each year.";

const REF_30S_C: &str = "Mediterranean cooking relies heavily on olive oil, fresh herbs, and seasonal vegetables. \
Tomatoes were introduced to European cuisine after Spanish explorers returned from the Americas. \
A traditional risotto requires constant stirring to release starch from Arborio rice grains. \
Fermentation transforms grape juice into wine through the action of natural yeasts on sugar. \
Sourdough bread uses a live culture of bacteria and wild yeast instead of commercial packets. \
The Maillard reaction between amino acids and sugars creates the brown crust on grilled meat.";

const REF_60S_B: &str = "The Panama Canal connects the Atlantic and Pacific oceans through a series of concrete locks. \
Construction began in nineteen oh four and took ten years to complete at enormous human cost. \
Ships entering from the Atlantic side are raised twenty six metres above sea level by three lock chambers. \
Gatun Lake was created by damming the Chagres River and flooding an entire valley. \
Each lock chamber uses gravity fed water from the lake rather than mechanical pumps. \
A single transit moves approximately two hundred million litres of fresh water into the ocean. \
The canal was expanded in twenty sixteen with larger locks to accommodate modern container ships. \
These new Neopanamax locks use water saving basins that recycle sixty percent of each fill. \
Over fourteen thousand vessels pass through the canal annually carrying five percent of world trade. \
Drought conditions in recent years have forced authorities to limit daily transits and vessel draft. \
Tolls range from a few hundred dollars for small sailboats to nearly a million for the largest tankers. \
The canal remains one of the most significant engineering achievements of the twentieth century. \
Ongoing maintenance requires continuous dredging of the navigational channel to prevent silting. \
Tropical rainfall patterns directly influence water levels in Gatun and Alajuela lakes. \
The Panama Canal Authority employs over nine thousand workers to operate and maintain the waterway.";

const REF_60S_C: &str = "The human immune system consists of two complementary defence mechanisms working in coordination. \
Innate immunity provides immediate but non specific protection through physical barriers and white blood cells. \
Neutrophils are the first responders arriving at infection sites within minutes of tissue damage. \
The adaptive immune system develops targeted responses through B cells and T cells over several days. \
B cells produce antibodies that bind to specific molecular patterns on the surface of pathogens. \
Helper T cells coordinate the overall immune response by releasing chemical signalling molecules called cytokines. \
Memory cells persist for decades allowing the body to mount rapid responses to previously encountered threats. \
Vaccination works by introducing harmless fragments of a pathogen to train the adaptive immune system. \
Autoimmune disorders occur when the immune system mistakenly attacks the body's own healthy tissue. \
Allergic reactions represent an exaggerated immune response to normally harmless environmental substances. \
Immunosuppressive drugs are prescribed after organ transplants to prevent rejection of donor tissue. \
The thymus gland plays a critical role in T cell maturation during childhood and adolescence. \
Researchers continue developing immunotherapy treatments that harness the immune system to fight cancer cells. \
The gut microbiome influences immune function through constant interaction with intestinal immune tissue. \
Regular moderate exercise has been shown to enhance immune surveillance and reduce inflammation markers.";

/// Normalise text for WER comparison: lowercase, strip punctuation, collapse whitespace.
fn normalise_for_wer(s: &str) -> Vec<String> {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c.is_whitespace() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .map(|w| w.to_string())
        .collect()
}

/// Word Error Rate via word-level Levenshtein distance.
/// Returns (wer_percent, substitutions, insertions, deletions, ref_word_count).
fn compute_wer(reference: &str, hypothesis: &str) -> (f64, usize, usize, usize, usize) {
    let ref_words = normalise_for_wer(reference);
    let hyp_words = normalise_for_wer(hypothesis);
    let n = ref_words.len();
    let m = hyp_words.len();

    if n == 0 {
        return (if m == 0 { 0.0 } else { 100.0 }, 0, m, 0, 0);
    }

    // DP matrix: d[i][j] = edit distance between ref[..i] and hyp[..j]
    let mut d = vec![vec![0usize; m + 1]; n + 1];
    for i in 0..=n { d[i][0] = i; }
    for j in 0..=m { d[0][j] = j; }

    for i in 1..=n {
        for j in 1..=m {
            let cost = if ref_words[i - 1] == hyp_words[j - 1] { 0 } else { 1 };
            d[i][j] = (d[i - 1][j] + 1)          // deletion
                .min(d[i][j - 1] + 1)             // insertion
                .min(d[i - 1][j - 1] + cost);     // substitution
        }
    }

    // Backtrace to count S, I, D
    let (mut i, mut j) = (n, m);
    let (mut subs, mut ins, mut dels) = (0, 0, 0);
    while i > 0 || j > 0 {
        if i > 0 && j > 0 {
            let cost = if ref_words[i - 1] == hyp_words[j - 1] { 0 } else { 1 };
            if d[i][j] == d[i - 1][j - 1] + cost {
                if cost == 1 { subs += 1; }
                i -= 1;
                j -= 1;
                continue;
            }
        }
        if i > 0 && d[i][j] == d[i - 1][j] + 1 {
            dels += 1;
            i -= 1;
        } else {
            ins += 1;
            j -= 1;
        }
    }

    let wer = (subs + ins + dels) as f64 / n as f64 * 100.0;
    (wer, subs, ins, dels, n)
}

fn reference_for_clip(clip_label: &str) -> &'static str {
    match clip_label {
        "5s" => REF_5S,
        "5s-b" => REF_5S_B,
        "5s-c" => REF_5S_C,
        "30s" => REF_30S,
        "30s-b" => REF_30S_B,
        "30s-c" => REF_30S_C,
        "60s" => REF_60S,
        "60s-b" => REF_60S_B,
        "60s-c" => REF_60S_C,
        _ => "",
    }
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
    wer: f64,
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
        ("ggml-medium.en-q5_0.bin",       "whisper-medium-en"),
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

    #[cfg(feature = "bench_extra")]
    let moonshine_tiny_path: Option<PathBuf> = {
        let p = models_dir.join("moonshine-tiny-ONNX");
        if p.exists() && p.is_dir() {
            println!("  FOUND    moonshine-tiny");
            Some(p)
        } else {
            println!("  MISSING  moonshine-tiny ({})", p.display());
            None
        }
    };
    #[cfg(feature = "bench_extra")]
    let moonshine_base_path: Option<PathBuf> = {
        let p = models_dir.join("moonshine-base-ONNX");
        if p.exists() && p.is_dir() {
            println!("  FOUND    moonshine-base");
            Some(p)
        } else {
            println!("  MISSING  moonshine-base ({})", p.display());
            None
        }
    };
    #[cfg(feature = "bench_extra")]
    let sensevoice_path: Option<PathBuf> = {
        let p = models_dir.join("sensevoice-small");
        if p.exists() && p.is_dir() {
            println!("  FOUND    sensevoice-small");
            Some(p)
        } else {
            println!("  MISSING  sensevoice-small ({})", p.display());
            None
        }
    };
    #[cfg(not(feature = "bench_extra"))]
    println!("  (bench_extra feature disabled — skipping moonshine/sensevoice models)");

    // WAV files
    println!("\n-- WAV fixtures --");
    let clips: Vec<(&str, &str)> = vec![
        ("benchmark-5s.wav",    "5s"),
        ("benchmark-5s-b.wav",  "5s-b"),
        ("benchmark-5s-c.wav",  "5s-c"),
        ("benchmark-30s.wav",   "30s"),
        ("benchmark-30s-b.wav", "30s-b"),
        ("benchmark-30s-c.wav", "30s-c"),
        ("benchmark-60s.wav",   "60s"),
        ("benchmark-60s-b.wav", "60s-b"),
        ("benchmark-60s-c.wav", "60s-c"),
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

            let reference = reference_for_clip(clip_label);
            let (wer, subs, ins, dels, ref_n) = compute_wer(reference, &first_text);
            println!("  -> avg={}ms  min={}ms  max={}ms  WER={:.1}% (S={} I={} D={} / {} words)", avg, min, max, wer, subs, ins, dels, ref_n);

            results.push(BenchResult {
                model: model_label.to_string(),
                clip: clip_label.to_string(),
                avg_ms: avg,
                min_ms: min,
                max_ms: max,
                wer,
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

            let reference = reference_for_clip(clip_label);
            let (wer, subs, ins, dels, ref_n) = compute_wer(reference, &first_text);
            println!("  -> avg={}ms  min={}ms  max={}ms  WER={:.1}% (S={} I={} D={} / {} words)", avg, min, max, wer, subs, ins, dels, ref_n);

            results.push(BenchResult {
                model: "parakeet-tdt-v2".to_string(),
                clip: clip_label.to_string(),
                avg_ms: avg,
                min_ms: min,
                max_ms: max,
                wer,
                first_text,
            });
        }
    }

    // -----------------------------------------------------------------------
    // Moonshine + SenseVoice models (bench_extra feature)
    // -----------------------------------------------------------------------
    #[cfg(feature = "bench_extra")]
    {
        let bench_extra_providers: Option<Vec<ort::execution_providers::ExecutionProviderDispatch>> =
            if parakeet_provider == "cuda" {
                println!("  [bench_extra] Using CUDA ExecutionProvider for Moonshine/SenseVoice");
                Some(vec![
                    CUDAExecutionProvider::default().with_tf32(true).build(),
                    CPUExecutionProvider::default().build(),
                ])
            } else {
                None // Use default CPU
            };

        // --- Moonshine tiny ---
        if let Some(ref mpath) = moonshine_tiny_path {
            println!("\n=== moonshine-tiny (provider={}) ===", if bench_extra_providers.is_some() { "cuda" } else { "cpu" });
            let load_start = Instant::now();
            let mut engine = MoonshineEngine::new();
            let mut params = MoonshineModelParams::tiny();
            params.execution_providers = bench_extra_providers.clone();
            match engine.load_model_with_params(mpath.as_path(), params) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("  ERROR loading moonshine-tiny: {}", e);
                }
            }
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

                let needs_chunking = audio.len() > 30 * 16000; // > 30 seconds
                let vad_start = Instant::now();
                let chunks: Vec<Vec<f32>> = if needs_chunking {
                    vad_chunk_audio(&audio)
                } else {
                    vec![audio]
                };
                let vad_ms = if needs_chunking { vad_start.elapsed().as_millis() as u64 } else { 0 };
                if needs_chunking {
                    println!("  VAD chunking: {}ms -> {} segments", vad_ms, chunks.len());
                }

                let mut latencies: Vec<u64> = Vec::with_capacity(ITERATIONS);
                let mut first_text = String::new();

                for i in 0..ITERATIONS {
                    let t = Instant::now();
                    let mut combined_text = String::new();
                    let mut had_error = false;

                    for (seg_idx, seg) in chunks.iter().enumerate() {
                        match engine.transcribe_samples(seg.clone(), None) {
                            Ok(result) => {
                                if !combined_text.is_empty() && !result.text.trim().is_empty() {
                                    combined_text.push(' ');
                                }
                                combined_text.push_str(result.text.trim());
                            }
                            Err(e) => {
                                eprintln!("  ERROR during inference run {} seg {}: {}", i + 1, seg_idx, e);
                                had_error = true;
                                break;
                            }
                        }
                    }

                    if had_error { break; }

                    let elapsed = t.elapsed().as_millis() as u64;
                    latencies.push(elapsed);
                    if i == 0 {
                        first_text = combined_text.trim().to_string();
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

                let reference = reference_for_clip(clip_label);
                let (wer, subs, ins, dels, ref_n) = compute_wer(reference, &first_text);
                println!("  -> avg={}ms  min={}ms  max={}ms  WER={:.1}% (S={} I={} D={} / {} words)", avg, min, max, wer, subs, ins, dels, ref_n);
                if needs_chunking {
                    println!("  (total incl. VAD: avg={}ms)", avg + vad_ms);
                }

                results.push(BenchResult {
                    model: "moonshine-tiny".to_string(),
                    clip: clip_label.to_string(),
                    avg_ms: avg,
                    min_ms: min,
                    max_ms: max,
                    wer,
                    first_text,
                });
            }
        }

        // --- Moonshine base ---
        if let Some(ref mpath) = moonshine_base_path {
            println!("\n=== moonshine-base (provider={}) ===", if bench_extra_providers.is_some() { "cuda" } else { "cpu" });
            let load_start = Instant::now();
            let mut engine = MoonshineEngine::new();
            let mut params = MoonshineModelParams::base();
            params.execution_providers = bench_extra_providers.clone();
            match engine.load_model_with_params(mpath.as_path(), params) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("  ERROR loading moonshine-base: {}", e);
                }
            }
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

                let needs_chunking = audio.len() > 30 * 16000; // > 30 seconds
                let vad_start = Instant::now();
                let chunks: Vec<Vec<f32>> = if needs_chunking {
                    vad_chunk_audio(&audio)
                } else {
                    vec![audio]
                };
                let vad_ms = if needs_chunking { vad_start.elapsed().as_millis() as u64 } else { 0 };
                if needs_chunking {
                    println!("  VAD chunking: {}ms -> {} segments", vad_ms, chunks.len());
                }

                let mut latencies: Vec<u64> = Vec::with_capacity(ITERATIONS);
                let mut first_text = String::new();

                for i in 0..ITERATIONS {
                    let t = Instant::now();
                    let mut combined_text = String::new();
                    let mut had_error = false;

                    for (seg_idx, seg) in chunks.iter().enumerate() {
                        match engine.transcribe_samples(seg.clone(), None) {
                            Ok(result) => {
                                if !combined_text.is_empty() && !result.text.trim().is_empty() {
                                    combined_text.push(' ');
                                }
                                combined_text.push_str(result.text.trim());
                            }
                            Err(e) => {
                                eprintln!("  ERROR during inference run {} seg {}: {}", i + 1, seg_idx, e);
                                had_error = true;
                                break;
                            }
                        }
                    }

                    if had_error { break; }

                    let elapsed = t.elapsed().as_millis() as u64;
                    latencies.push(elapsed);
                    if i == 0 {
                        first_text = combined_text.trim().to_string();
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

                let reference = reference_for_clip(clip_label);
                let (wer, subs, ins, dels, ref_n) = compute_wer(reference, &first_text);
                println!("  -> avg={}ms  min={}ms  max={}ms  WER={:.1}% (S={} I={} D={} / {} words)", avg, min, max, wer, subs, ins, dels, ref_n);
                if needs_chunking {
                    println!("  (total incl. VAD: avg={}ms)", avg + vad_ms);
                }

                results.push(BenchResult {
                    model: "moonshine-base".to_string(),
                    clip: clip_label.to_string(),
                    avg_ms: avg,
                    min_ms: min,
                    max_ms: max,
                    wer,
                    first_text,
                });
            }
        }

        // --- SenseVoice small ---
        if let Some(ref spath) = sensevoice_path {
            println!("\n=== sensevoice-small (provider={}) ===", if bench_extra_providers.is_some() { "cuda" } else { "cpu" });
            let load_start = Instant::now();
            let mut engine = SenseVoiceEngine::new();
            let mut params = SenseVoiceModelParams::default();
            params.execution_providers = bench_extra_providers.clone();
            match engine.load_model_with_params(spath.as_path(), params) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("  ERROR loading sensevoice-small: {}", e);
                }
            }
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

                let needs_chunking = audio.len() > 30 * 16000; // > 30 seconds
                let vad_start = Instant::now();
                let chunks: Vec<Vec<f32>> = if needs_chunking {
                    vad_chunk_audio(&audio)
                } else {
                    vec![audio]
                };
                let vad_ms = if needs_chunking { vad_start.elapsed().as_millis() as u64 } else { 0 };
                if needs_chunking {
                    println!("  VAD chunking: {}ms -> {} segments", vad_ms, chunks.len());
                }

                let mut latencies: Vec<u64> = Vec::with_capacity(ITERATIONS);
                let mut first_text = String::new();

                for i in 0..ITERATIONS {
                    let t = Instant::now();
                    let mut combined_text = String::new();
                    let mut had_error = false;

                    for (seg_idx, seg) in chunks.iter().enumerate() {
                        match engine.transcribe_samples(seg.clone(), None) {
                            Ok(result) => {
                                if !combined_text.is_empty() && !result.text.trim().is_empty() {
                                    combined_text.push(' ');
                                }
                                combined_text.push_str(result.text.trim());
                            }
                            Err(e) => {
                                eprintln!("  ERROR during inference run {} seg {}: {}", i + 1, seg_idx, e);
                                had_error = true;
                                break;
                            }
                        }
                    }

                    if had_error { break; }

                    let elapsed = t.elapsed().as_millis() as u64;
                    latencies.push(elapsed);
                    if i == 0 {
                        first_text = combined_text.trim().to_string();
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

                let reference = reference_for_clip(clip_label);
                let (wer, subs, ins, dels, ref_n) = compute_wer(reference, &first_text);
                println!("  -> avg={}ms  min={}ms  max={}ms  WER={:.1}% (S={} I={} D={} / {} words)", avg, min, max, wer, subs, ins, dels, ref_n);
                if needs_chunking {
                    println!("  (total incl. VAD: avg={}ms)", avg + vad_ms);
                }

                results.push(BenchResult {
                    model: "sensevoice-small".to_string(),
                    clip: clip_label.to_string(),
                    avg_ms: avg,
                    min_ms: min,
                    max_ms: max,
                    wer,
                    first_text,
                });
            }
        }
    }

    print_summary(&results);
    write_markdown_report(&results);
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
        "{:<30} | {:<4} | {:>8} | {:>8} | {:>8} | {:>7}",
        "Model", "Clip", "Avg (ms)", "Min (ms)", "Max (ms)", "WER %"
    );
    println!("{}", "-".repeat(80));
    for r in results {
        println!(
            "{:<30} | {:<4} | {:>8} | {:>8} | {:>8} | {:>6.1}%",
            r.model, r.clip, r.avg_ms, r.min_ms, r.max_ms, r.wer
        );
    }
    println!("{}", "=".repeat(80));
    println!("Transcription samples (first run of each model/clip):");
    for r in results {
        if !r.first_text.is_empty() {
            println!("  [{} / {}] {}", r.model, r.clip, truncate(&r.first_text, 100));
        }
    }

    // -----------------------------------------------------------------------
    // Model summary — averages across all clips + speed/accuracy score
    // -----------------------------------------------------------------------
    // Collect unique model names in order
    let mut model_names: Vec<String> = Vec::new();
    for r in results {
        if !model_names.contains(&r.model) {
            model_names.push(r.model.clone());
        }
    }

    struct ModelSummary {
        name: String,
        avg_latency_ms: f64,
        avg_wer: f64,
        accuracy: f64,      // 100 - WER (clamped to 0)
    }

    let mut summaries: Vec<ModelSummary> = Vec::new();
    for name in &model_names {
        let model_results: Vec<&BenchResult> = results.iter().filter(|r| &r.model == name).collect();
        if model_results.is_empty() { continue; }
        let count = model_results.len();
        let avg_latency = model_results.iter().map(|r| r.avg_ms as f64).sum::<f64>() / count as f64;
        let avg_wer = model_results.iter().map(|r| r.wer).sum::<f64>() / count as f64;
        let accuracy = (100.0 - avg_wer).max(0.0);
        summaries.push(ModelSummary {
            name: name.clone(),
            avg_latency_ms: avg_latency,
            avg_wer,
            accuracy,
        });
    }

    if summaries.is_empty() { return; }

    // Find best (lowest) latency and best (highest) accuracy for normalization
    let best_latency = summaries.iter().map(|s| s.avg_latency_ms).fold(f64::MAX, f64::min);
    let best_accuracy = summaries.iter().map(|s| s.accuracy).fold(0.0f64, f64::max);

    println!("\n");
    println!("================================================================================");
    println!("MODEL RANKINGS");
    println!("================================================================================");
    println!(
        "{:<30} | {:>10} | {:>7} | {:>8} | {:>8} | {:>6}",
        "Model", "Avg Lat.", "Avg WER", "Accuracy", "Speed", "Score"
    );
    println!("{}", "-".repeat(88));

    // Compute scores and collect for sorting
    struct ScoredModel {
        name: String,
        avg_latency_ms: f64,
        avg_wer: f64,
        accuracy: f64,
        speed_score: f64,
        overall_score: f64,
    }

    let mut scored: Vec<ScoredModel> = summaries.iter().map(|s| {
        // Speed score: best_latency / this_latency * 100 (best model = 100)
        let speed_score = if s.avg_latency_ms > 0.0 {
            (best_latency / s.avg_latency_ms) * 100.0
        } else {
            100.0
        };
        // Accuracy score: this_accuracy / best_accuracy * 100 (best model = 100)
        let accuracy_score = if best_accuracy > 0.0 {
            (s.accuracy / best_accuracy) * 100.0
        } else {
            100.0
        };
        // Overall score: geometric mean of speed and accuracy (balances both)
        let overall_score = (speed_score * accuracy_score).sqrt();
        ScoredModel {
            name: s.name.clone(),
            avg_latency_ms: s.avg_latency_ms,
            avg_wer: s.avg_wer,
            accuracy: s.accuracy,
            speed_score,
            overall_score,
        }
    }).collect();

    // Sort by overall score descending
    scored.sort_by(|a, b| b.overall_score.partial_cmp(&a.overall_score).unwrap_or(std::cmp::Ordering::Equal));

    for s in &scored {
        println!(
            "{:<30} | {:>8.0}ms | {:>5.1}%  | {:>6.1}%  | {:>6.1}/100 | {:>4.1}/100",
            s.name, s.avg_latency_ms, s.avg_wer, s.accuracy, s.speed_score, s.overall_score
        );
    }
    println!("{}", "=".repeat(88));
    println!("Speed:  100 = fastest model, scaled relative to best avg latency ({:.0}ms)", best_latency);
    println!("Score:  geometric mean of speed and accuracy (balanced ranking)");
}

// ---------------------------------------------------------------------------
// Markdown report writer
// ---------------------------------------------------------------------------

fn write_markdown_report(results: &[BenchResult]) {
    if results.is_empty() {
        return;
    }

    let path = "benchmark-results.md";
    let mut file = match std::fs::File::create(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("ERROR: could not create {}: {}", path, e);
            return;
        }
    };

    // Timestamp (Unix epoch seconds — no chrono dependency)
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let _ = writeln!(file, "# VoiceType Benchmark Results\n");
    let _ = writeln!(file, "Generated: epoch {} (run `date -d @{}` to convert)\n", timestamp, timestamp);

    // --- Results table ---
    let _ = writeln!(file, "## Results\n");
    let _ = writeln!(file, "| Model | Clip | Avg (ms) | Min (ms) | Max (ms) | WER % |");
    let _ = writeln!(file, "|-------|------|----------|----------|----------|-------|");
    for r in results {
        let _ = writeln!(file, "| {} | {} | {} | {} | {} | {:.1}% |",
            r.model, r.clip, r.avg_ms, r.min_ms, r.max_ms, r.wer);
    }

    // --- Model Rankings ---
    // Collect unique model names in order
    let mut model_names: Vec<String> = Vec::new();
    for r in results {
        if !model_names.contains(&r.model) {
            model_names.push(r.model.clone());
        }
    }

    struct MdModelSummary {
        name: String,
        avg_latency_ms: f64,
        avg_wer: f64,
        accuracy: f64,
    }

    let mut summaries: Vec<MdModelSummary> = Vec::new();
    for name in &model_names {
        let model_results: Vec<&BenchResult> = results.iter().filter(|r| &r.model == name).collect();
        if model_results.is_empty() { continue; }
        let count = model_results.len();
        let avg_latency = model_results.iter().map(|r| r.avg_ms as f64).sum::<f64>() / count as f64;
        let avg_wer = model_results.iter().map(|r| r.wer).sum::<f64>() / count as f64;
        let accuracy = (100.0 - avg_wer).max(0.0);
        summaries.push(MdModelSummary {
            name: name.clone(),
            avg_latency_ms: avg_latency,
            avg_wer,
            accuracy,
        });
    }

    if !summaries.is_empty() {
        let best_latency = summaries.iter().map(|s| s.avg_latency_ms).fold(f64::MAX, f64::min);
        let best_accuracy = summaries.iter().map(|s| s.accuracy).fold(0.0f64, f64::max);

        struct ScoredMd {
            name: String,
            avg_latency_ms: f64,
            avg_wer: f64,
            accuracy: f64,
            speed_score: f64,
            overall_score: f64,
        }

        let mut scored: Vec<ScoredMd> = summaries.iter().map(|s| {
            let speed_score = if s.avg_latency_ms > 0.0 {
                (best_latency / s.avg_latency_ms) * 100.0
            } else {
                100.0
            };
            let accuracy_score = if best_accuracy > 0.0 {
                (s.accuracy / best_accuracy) * 100.0
            } else {
                100.0
            };
            let overall_score = (speed_score * accuracy_score).sqrt();
            ScoredMd {
                name: s.name.clone(),
                avg_latency_ms: s.avg_latency_ms,
                avg_wer: s.avg_wer,
                accuracy: s.accuracy,
                speed_score,
                overall_score,
            }
        }).collect();

        scored.sort_by(|a, b| b.overall_score.partial_cmp(&a.overall_score).unwrap_or(std::cmp::Ordering::Equal));

        let _ = writeln!(file, "\n## Model Rankings\n");
        let _ = writeln!(file, "| Model | Avg Lat. | Avg WER | Accuracy | Speed | Score |");
        let _ = writeln!(file, "|-------|----------|---------|----------|-------|-------|");
        for s in &scored {
            let _ = writeln!(file, "| {} | {:.0}ms | {:.1}% | {:.1}% | {:.1}/100 | {:.1}/100 |",
                s.name, s.avg_latency_ms, s.avg_wer, s.accuracy, s.speed_score, s.overall_score);
        }
        let _ = writeln!(file, "\nSpeed: 100 = fastest model, scaled relative to best avg latency ({:.0}ms)", best_latency);
        let _ = writeln!(file, "Score: geometric mean of speed and accuracy (balanced ranking)");
    }

    // --- Reference Transcriptions ---
    let _ = writeln!(file, "\n## Reference Transcriptions\n");
    let clip_labels = ["5s", "5s-b", "5s-c", "30s", "30s-b", "30s-c", "60s", "60s-b", "60s-c"];
    for label in &clip_labels {
        let ref_text = reference_for_clip(label);
        if !ref_text.is_empty() {
            let _ = writeln!(file, "### {}\n", label);
            let _ = writeln!(file, "> {}\n", ref_text);
        }
    }

    // --- Transcription Samples ---
    let _ = writeln!(file, "## Transcription Samples\n");
    for name in &model_names {
        let _ = writeln!(file, "### {}\n", name);
        for r in results {
            if r.model == *name && !r.first_text.is_empty() {
                let _ = writeln!(file, "- **{}**: \"{}\"", r.clip, r.first_text);
            }
        }
        let _ = writeln!(file);
    }

    println!("Wrote benchmark-results.md");
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}
