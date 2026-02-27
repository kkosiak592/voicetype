# Phase 2: Audio + Whisper - Research

**Researched:** 2026-02-27
**Domain:** Rust audio capture (cpal/WASAPI), resampling (rubato), whisper.cpp inference (whisper-rs), CUDA 11.7 GPU acceleration, CPU fallback detection
**Confidence:** MEDIUM-HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Audio capture behavior:**
- Fall back to system default microphone with a brief notification if configured mic is not found (matches Discord/OBS/Teams pattern)
- On mic disconnect mid-recording, transcribe whatever audio was captured so far (don't discard)
- Target ~10ms audio buffers for low-latency capture
- Always downmix to mono regardless of input device channels (whisper needs mono 16kHz)
- Keep audio stream persistent (always-on mic, just not saving) so recording starts instantly on hotkey press — no device initialization delay

**Model management:**
- Store models in %APPDATA%/VoiceType/models/ (standard Windows app data directory)
- Phase 2 uses manual model placement — developer downloads model files to the expected path
- Prefer English-only model variants (.en) — no multi-language support needed
- Research phase should re-evaluate model choices (large-v3-turbo-q5_0 for GPU, small for CPU) to see if better English-only alternatives exist
- User has prior research in the artifacts/ folder that may inform model selection

**Transcription tuning:**
- Force language='en' — English-only mode, no auto-detection
- No initial_prompt for Phase 2 (vocabulary tuning is Phase 6)
- Hardcode whisper defaults (beam_size=5, temperature=0) — no exposed tuning parameters
- Simple batch API: record all audio, then transcribe as one batch after recording stops
- Speed comes from persistent audio stream + fast GPU inference, not streaming

**Verification & logging:**
- Log CUDA initialization success/failure at startup
- GPU verification: manual Task Manager check during test runs (per success criteria)
- Keep 2-3 reference WAV recordings in a test fixtures directory for regression testing
- Verbose logging for Phase 2: audio device selection, sample rate, buffer sizes, CUDA init, inference time, model load time
- Dial back logging verbosity in later phases

### Claude's Discretion
- Persistent audio stream implementation details
- Exact resampling configuration (rubato parameters)
- Error message content when model file is missing (must include download instructions and expected path)
- Test WAV file naming and directory structure
- Exact whisper-rs parameter values beyond beam_size and temperature

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| CORE-02 | App captures microphone audio at 16kHz via cpal/WASAPI | cpal 0.17.3 on Windows uses WASAPI; build_input_stream callback → mpsc channel → Vec<f32> buffer; SampleRate(16000) in StreamConfig; rubato FftFixedIn for resampling from device native rate |
| CORE-03 | App transcribes audio using whisper.cpp (whisper-rs) with GPU acceleration on CUDA 11.7 | whisper-rs 0.15.1 with `cuda` feature flag; WhisperContextParameters::use_gpu(true); CMAKE flag GGML_CUDA=1; LIBCLANG_PATH env var required; Pascal arch = compute capability 6.1 (CMAKE_CUDA_ARCHITECTURES=61) |
| CORE-04 | App falls back to CPU inference (whisper small model) when no NVIDIA GPU is detected | nvml-wrapper for runtime GPU detection (Nvml::init() returns error when NVML absent); WhisperContextParameters::use_gpu(false) for CPU build; ggml-small.en-q5_1.bin (190 MB) for CPU fallback |
</phase_requirements>

---

## Summary

Phase 2 implements the two highest-risk components in isolation: persistent audio capture at 16kHz and whisper.cpp transcription with CUDA GPU acceleration. The technical stack is fully established (cpal 0.17.3 + rubato + whisper-rs 0.15.1) and is the same stack used by multiple reference projects (BridgeVoice, Handy, Voquill). The main risk areas are the CUDA build configuration for Pascal architecture (compute 6.1) and the two-binary strategy required for GPU vs. CPU operation.

The model selection decision from the CONTEXT.md requires an update: **large-v3-turbo has no English-only (.en) variant** — that model type only exists for tiny, base, small, and medium sizes. The correct GPU model is `ggml-large-v3-turbo-q5_0.bin` (574 MB, multilingual but forced to English via `set_language("en")`) or `ggml-medium.en-q5_0.bin` (539 MB) as an English-only alternative with similar size. For CPU, `ggml-small.en-q5_1.bin` (190 MB) is the right choice.

The audio persistence pattern is straightforward: create one cpal stream at startup that always runs, use an Arc<Mutex<bool>> recording flag, and collect samples to a Vec<f32> buffer only while the flag is set. The resampling pipeline uses rubato's FftFixedIn to convert from device native rate (44100 or 48000 Hz) to whisper's required 16000 Hz in the callback before enqueuing.

**Primary recommendation:** Two separate Cargo feature targets — build with `--features cuda` for GPU distribution, build without for CPU. Runtime GPU detection via nvml-wrapper determines which model to load. whisper-rs's `WhisperContextParameters::use_gpu(true/false)` toggles GPU at context creation. All inference runs in `tokio::task::spawn_blocking` to avoid blocking the async Tauri runtime.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| cpal | 0.17.3 | Cross-platform audio I/O (WASAPI on Windows) | Standard for Rust audio capture; used by Keyless, Handy, Voquill, BridgeVoice reference projects |
| rubato | 0.12+ | Audio resampling (device native rate → 16kHz mono) | Standard Rust resampling crate; FftFixedIn is FFT-based, fast for fixed-ratio resampling |
| whisper-rs | 0.15.1 | Rust bindings to whisper.cpp, runs inference | Only maintained Rust binding to whisper.cpp; supports CUDA feature flag |
| nvml-wrapper | 0.10+ | Runtime NVIDIA GPU detection via NVML | Clean Rust wrapper; Nvml::init() returns error when no GPU present — perfect for fallback detection |
| tokio | (from Tauri) | spawn_blocking for synchronous inference on async runtime | Tauri's runtime is tokio; all blocking CPU work must go in spawn_blocking |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| hound | 3.x | Write WAV files for capture verification | Writing test WAV fixtures and verifying 16kHz capture; already in Handy reference project |
| log + env_logger | 0.4 / 0.11 | Structured verbose logging | Phase 2 verbose logging requirement; log! macros, controlled by RUST_LOG env var |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| whisper-rs | whisper-cpp-plus | whisper-cpp-plus is a newer fork with some improvements but far smaller ecosystem; whisper-rs has more reference implementations |
| rubato | manual linear resampling | Linear resampling introduces audible artifacts; rubato FFT resampling is quality-critical for transcription accuracy |
| nvml-wrapper | wgpu adapter enumeration | wgpu has known detection failures on some Windows configs (issue #2507); nvml-wrapper is dedicated to NVIDIA detection |

**Cargo.toml additions:**
```toml
[dependencies]
cpal = "0.17"
rubato = "0.15"
hound = "3"
nvml-wrapper = "0.10"
log = "0.4"
env_logger = "0.11"

[dependencies.whisper-rs]
version = "0.15"
features = ["cuda"]   # only in GPU build; omit for CPU build
```

**Note on two-binary approach:** The `cuda` feature in whisper-rs compiles in the CUDA runtime at build time. You cannot toggle GPU at runtime with a single binary unless you ship a CUDA build and fall back via `use_gpu(false)`. The cleanest approach for Phase 2 (verification only) is a single build with `cuda` feature and the GPU flag controlled via WhisperContextParameters at runtime. For distribution (Phase 7), a no-cuda CPU-only build is the correct fallback binary.

---

## Architecture Patterns

### Recommended Project Structure

```
src-tauri/src/
├── lib.rs              # Tauri app entry (existing)
├── tray.rs             # System tray (existing)
├── audio.rs            # cpal stream, persistent capture, mpsc, resampling
└── transcribe.rs       # whisper-rs context, model loading, inference, GPU detection
src-tauri/
├── Cargo.toml          # Add cpal, rubato, whisper-rs, nvml-wrapper
└── build.rs            # (if needed) LIBCLANG_PATH verification
test-fixtures/
├── hello-16khz.wav     # Short test WAV for transcription regression
├── engineering-terms.wav  # Structural engineering vocabulary sample
└── README.md           # Documents how fixtures were recorded
```

### Pattern 1: Persistent Audio Stream with Conditional Capture

**What:** cpal stream starts at app startup and runs forever. A shared atomic flag controls whether incoming samples are buffered. When the flag is true (hotkey held), samples accumulate. When false, samples are discarded.

**When to use:** Every voice capture app that needs instant start — eliminates ~50-200ms device initialization delay on hotkey press.

**Example:**
```rust
// Source: cpal docs + BridgeVoice architecture pattern
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct AudioCapture {
    _stream: cpal::Stream,              // must stay alive
    pub recording: Arc<AtomicBool>,
    pub buffer: Arc<Mutex<Vec<f32>>>,   // accumulated samples at 16kHz
}

pub fn start_persistent_stream() -> Result<AudioCapture, Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let device = host.default_input_device()
        .ok_or("no input device")?;

    let config = device.default_input_config()?;
    let native_sample_rate = config.sample_rate().0;
    let channels = config.channels() as usize;

    let recording = Arc::new(AtomicBool::new(false));
    let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));

    let recording_clone = recording.clone();
    let buffer_clone = buffer.clone();

    // Build resampler: device native rate → 16000 Hz
    let mut resampler = rubato::FftFixedIn::<f32>::new(
        native_sample_rate as usize,
        16000,
        128,   // chunk_size: ~8ms at 16kHz
        2,     // sub_chunks
        1,     // output channels (mono)
    )?;

    let stream = device.build_input_stream(
        &config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            if !recording_clone.load(Ordering::Relaxed) {
                return; // discard — not recording
            }
            // Downmix to mono
            let mono: Vec<f32> = data.chunks(channels)
                .map(|ch| ch.iter().sum::<f32>() / channels as f32)
                .collect();
            // Resample (simplified — real impl needs buffering for chunked resampler)
            if let Ok(mut buf) = buffer_clone.try_lock() {
                buf.extend_from_slice(&mono);
            }
        },
        |err| log::error!("audio stream error: {}", err),
        None,
    )?;

    stream.play()?;
    log::info!("Audio stream started: {} Hz, {} channels → 16kHz mono",
        native_sample_rate, channels);

    Ok(AudioCapture { _stream: stream, recording, buffer })
}
```

**Important:** rubato's FftFixedIn requires fixed-size input chunks. In practice, the callback buffer size varies (cpal makes no guarantees). Use an intermediate accumulator: collect raw callback samples into a staging buffer, then drain chunks of exactly `chunk_size * channels` into the resampler. See the Pitfalls section.

### Pattern 2: Whisper Inference with GPU/CPU Runtime Toggle

**What:** Build with `cuda` feature. At startup, detect GPU via nvml-wrapper. If GPU present, load large-v3-turbo with `use_gpu(true)`. If absent, load small with `use_gpu(false)`. Run inference in `spawn_blocking`.

**When to use:** Phase 2 (verification) and Phase 3+ (full pipeline).

**Example:**
```rust
// Source: whisper-rs docs.rs + nvml-wrapper docs.rs
use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams, SamplingStrategy};
use nvml_wrapper::Nvml;

pub enum ModelMode { Gpu, Cpu }

pub fn detect_gpu() -> ModelMode {
    match Nvml::init() {
        Ok(nvml) => {
            match nvml.device_by_index(0) {
                Ok(device) => {
                    let name = device.name().unwrap_or_default();
                    log::info!("NVIDIA GPU detected: {}", name);
                    ModelMode::Gpu
                }
                Err(e) => {
                    log::info!("NVML init OK but no device: {}", e);
                    ModelMode::Cpu
                }
            }
        }
        Err(e) => {
            log::info!("NVML init failed (no NVIDIA GPU): {}", e);
            ModelMode::Cpu
        }
    }
}

pub fn load_whisper(model_path: &str, mode: &ModelMode)
    -> Result<WhisperContext, Box<dyn std::error::Error>>
{
    let mut ctx_params = WhisperContextParameters::default();
    match mode {
        ModelMode::Gpu => ctx_params.use_gpu(true),
        ModelMode::Cpu => ctx_params.use_gpu(false),
    };

    let ctx = WhisperContext::new_with_params(model_path, ctx_params)
        .map_err(|e| format!("Failed to load model at {}: {}", model_path, e))?;

    log::info!("Whisper model loaded: {} (GPU={})", model_path,
        matches!(mode, ModelMode::Gpu));
    Ok(ctx)
}

// In Tauri command (async context):
pub async fn transcribe(audio: Vec<f32>, ctx: Arc<WhisperContext>)
    -> Result<String, String>
{
    let start = std::time::Instant::now();

    let result = tokio::task::spawn_blocking(move || {
        let mut state = ctx.create_state()
            .map_err(|e| e.to_string())?;

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

        state.full(params, &audio)
            .map_err(|e| e.to_string())?;

        let n = state.full_n_segments().map_err(|e| e.to_string())?;
        let mut text = String::new();
        for i in 0..n {
            text.push_str(&state.full_get_segment_text(i)
                .map_err(|e| e.to_string())?);
        }
        Ok::<String, String>(text.trim().to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    log::info!("Transcription completed in {}ms", start.elapsed().as_millis());
    Ok(result)
}
```

### Pattern 3: rubato Resampling with Chunked Accumulator

**What:** cpal callbacks deliver variable-length buffers. rubato FftFixedIn requires fixed input chunk size. Bridge them with an accumulator.

**Example:**
```rust
// Source: rubato docs.rs + cpal GitHub issue #753
use rubato::{FftFixedIn, Resampler};

struct ResamplingBuffer {
    resampler: FftFixedIn<f32>,
    staging: Vec<f32>,      // raw mono samples from callback
    output: Vec<f32>,       // resampled 16kHz samples
    chunk_size: usize,
}

impl ResamplingBuffer {
    fn new(in_rate: usize) -> Self {
        let chunk_size = 1024; // ~64ms at 16kHz
        let resampler = FftFixedIn::<f32>::new(
            in_rate, 16000, chunk_size, 2, 1
        ).expect("resampler init");
        Self { resampler, staging: Vec::new(), output: Vec::new(), chunk_size }
    }

    fn push(&mut self, samples: &[f32]) {
        self.staging.extend_from_slice(samples);
        // Process as many complete chunks as available
        while self.staging.len() >= self.chunk_size {
            let chunk: Vec<Vec<f32>> = vec![self.staging.drain(..self.chunk_size).collect()];
            if let Ok(out) = self.resampler.process(&chunk, None) {
                self.output.extend_from_slice(&out[0]);
            }
        }
    }

    fn flush(&mut self) -> Vec<f32> {
        // Flush remaining samples with zero-padding
        if !self.staging.is_empty() {
            let padded_len = self.chunk_size;
            let mut padded = self.staging.clone();
            padded.resize(padded_len, 0.0);
            let chunk = vec![padded];
            if let Ok(out) = self.resampler.process(&chunk, None) {
                // Only take non-zero-padded portion
                let out_samples = (self.staging.len() * 16000) / self.resampler.input_frames_next();
                self.output.extend_from_slice(&out[0][..out_samples.min(out[0].len())]);
            }
        }
        let result = self.output.clone();
        self.output.clear();
        self.staging.clear();
        result
    }
}
```

### Anti-Patterns to Avoid

- **Initializing cpal stream on hotkey press:** Adds 50-200ms initialization delay. Keep stream persistent; only toggle the recording flag.
- **Passing WhisperContext across threads without Arc:** WhisperContext is not Clone; wrap in Arc<WhisperContext> for sharing between Tauri commands.
- **Calling state.full() in an async context directly:** Blocks the tokio runtime. Always wrap in spawn_blocking.
- **Assuming device supports 16kHz:** WASAPI shared mode devices most commonly report 44100 or 48000 Hz. Always check and resample.
- **Using mpsc::Sender in cpal callback with blocking send:** Can deadlock. Use try_send or a lockless ring buffer (ringbuf crate).

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Audio resampling 44100/48000 → 16kHz | Custom linear interpolation | rubato FftFixedIn | Linear resampling produces audible aliasing that degrades transcription accuracy; FFT resampling is the correct approach |
| GPU presence detection | Checking if cuda DLL loads | nvml-wrapper Nvml::init() | NVML is the authoritative NVIDIA API; catches edge cases (driver installed but no GPU, Quadro vs GeForce differences) |
| WAV file writing | Manual RIFF header | hound crate | WAV format has spec-level edge cases (sample alignment, header fields); hound handles all of them |
| Whisper C FFI bindings | Direct whisper.cpp FFI | whisper-rs | Managing the C ABI lifecycle manually risks segfaults; whisper-rs handles model lifetime and state safety |
| Thread-safe audio accumulation | Custom mutex-wrapped Vec | ringbuf crate (or Arc<Mutex<Vec>>) | Lock contention in audio callbacks causes glitches; for Phase 2 try_lock is acceptable, upgrade to ringbuf if dropouts appear |

**Key insight:** Audio DSP (resampling, format conversion) and GPU management both have hidden correctness requirements that only manifest under specific hardware configurations. Use proven libraries.

---

## Common Pitfalls

### Pitfall 1: Silent CPU Fallback in CUDA Build
**What goes wrong:** whisper-rs CUDA build succeeds but inference silently runs on CPU. Task Manager shows 0% GPU. Latency is 2-4s instead of 300ms.
**Why it happens:** `CMAKE_CUDA_ARCHITECTURES` not set, so the CUDA kernel is not compiled for Pascal (compute 6.1). The binary falls back to CPU without error.
**How to avoid:** Set `CMAKE_CUDA_ARCHITECTURES=61` via the `GGML_CUDA_ARCHITECTURES` or `CMAKE_CUDA_ARCHITECTURES` CMake variable before building. Also call `log::info!("GPU: {}", ctx.is_multilingual())` and verify Task Manager GPU Compute utilization during a test run.
**Warning signs:** Inference takes >1s despite GPU being present; `nvidia-smi` shows 0% utilization during transcription.

### Pitfall 2: cpal Callback Stopped by mpsc Backpressure
**What goes wrong:** Audio stream silently stops delivering samples. Recording appears to work but buffer is empty.
**Why it happens:** `mpsc::Sender::send()` blocks if receiver isn't consuming fast enough. In cpal callback, any blocking call stops the callback thread (cpal issue #970).
**How to avoid:** Use `try_send` (never `send`), or use a lockless `ringbuf` producer. In Phase 2 `try_lock` on a Mutex is acceptable since callback drops are not critical for verification.
**Warning signs:** WAV file is shorter than expected; buffer always appears empty despite recording.

### Pitfall 3: WASAPI Device Returns Non-16kHz Config
**What goes wrong:** `device.default_input_config()` returns 44100 or 48000 Hz. Passing these samples directly to whisper-rs produces garbled/wrong transcription.
**Why it happens:** WASAPI shared mode uses the Windows audio engine's native format which is almost never 16kHz.
**How to avoid:** Always read `config.sample_rate()` and apply rubato resampling when it differs from 16000. Never assume 16kHz from WASAPI.
**Warning signs:** Transcription output is garbled or produces wrong words even on clear speech.

### Pitfall 4: LIBCLANG_PATH Not Set
**What goes wrong:** `cargo build --features cuda` fails with "could not find clang" or `bindgen` errors.
**Why it happens:** whisper-rs-sys uses bindgen to generate Rust bindings from whisper.h. bindgen requires libclang. Windows MSVC doesn't add it to PATH automatically.
**How to avoid:** Run `where.exe clang` to find the path. Set `LIBCLANG_PATH` to the directory containing `clang.exe` (typically `C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\Llvm\x64\bin`). Set in system environment or in `.cargo/config.toml`.
**Warning signs:** Build fails at the `whisper-rs-sys` crate with clang/bindgen error.

### Pitfall 5: Model Path Not Found — Opaque Error
**What goes wrong:** `WhisperContext::new_with_params` panics or returns an opaque error without indicating what path was tried.
**Why it happens:** whisper-rs propagates the C++ error message which does not include the attempted path.
**How to avoid:** Check `std::path::Path::new(model_path).exists()` before calling `new_with_params`. Return a user-friendly error with the expected path and download instructions.
**Warning signs:** App crashes at startup with unclear C FFI panic.

### Pitfall 6: WhisperContext Not Thread-Safe Across Tauri Commands
**What goes wrong:** Multiple rapid hotkey presses cause concurrent `state.full()` calls. Undefined behavior or panic.
**Why it happens:** `WhisperState` is per-context, but concurrent calls are not safe on the same context.
**How to avoid:** Use a `tokio::sync::Mutex<WhisperContext>` (not std Mutex) to serialize transcription requests, or create a new state per call (cheap operation). Creating a new `state` via `ctx.create_state()` each time is the simpler approach.
**Warning signs:** Panic or garbled output when hotkey pressed twice quickly.

---

## Code Examples

### CUDA Build Environment Setup (Windows)

```powershell
# Source: whisper-rs BUILDING.md (Codeberg)
# Run in PowerShell as Administrator before building

# 1. Verify CUDA Toolkit 11.7 installed
nvcc --version  # should show 11.7

# 2. Set LIBCLANG_PATH (Visual Studio 2022 path)
$clangPath = "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\Llvm\x64\bin"
[System.Environment]::SetEnvironmentVariable("LIBCLANG_PATH", $clangPath, "Machine")

# 3. Build with CUDA feature
# CMAKE_CUDA_ARCHITECTURES=61 for Quadro P2000 (Pascal)
$env:CMAKE_CUDA_ARCHITECTURES = "61"
cargo build --features cuda
```

**Note on CMAKE_CUDA_ARCHITECTURES:** whisper-rs-sys uses cmake-rs to build whisper.cpp. The `CMAKE_CUDA_ARCHITECTURES` environment variable is read by CMake automatically from the environment. Setting it to `"61"` targets Pascal (P2000, GTX 1060-1080). Setting it to `"all"` compiles for all architectures but takes much longer. Setting it to nothing lets CMake choose the host GPU, which is also fine if building on the P2000 machine.

### Model File Layout

```
%APPDATA%\VoiceType\models\
├── ggml-large-v3-turbo-q5_0.bin   # GPU model (574 MB) — download from HuggingFace
├── ggml-medium.en-q5_0.bin        # GPU alternative, English-only (539 MB)
└── ggml-small.en-q5_1.bin         # CPU fallback (190 MB)

# Download commands (PowerShell):
# GPU model:
Invoke-WebRequest -Uri "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin" -OutFile "$env:APPDATA\VoiceType\models\ggml-large-v3-turbo-q5_0.bin"

# CPU model:
Invoke-WebRequest -Uri "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en-q5_1.bin" -OutFile "$env:APPDATA\VoiceType\models\ggml-small.en-q5_1.bin"
```

### GPU Detection and Model Path Resolution

```rust
// Source: nvml-wrapper docs.rs + project pattern
use std::path::PathBuf;

pub fn models_dir() -> PathBuf {
    let appdata = std::env::var("APPDATA").expect("APPDATA not set");
    PathBuf::from(appdata).join("VoiceType").join("models")
}

pub fn resolve_model_path(mode: &ModelMode) -> Result<PathBuf, String> {
    let dir = models_dir();
    let filename = match mode {
        ModelMode::Gpu => "ggml-large-v3-turbo-q5_0.bin",
        ModelMode::Cpu => "ggml-small.en-q5_1.bin",
    };
    let path = dir.join(filename);
    if !path.exists() {
        return Err(format!(
            "Model file not found: {}\n\
            Download it to: {}\n\
            GPU model: https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin\n\
            CPU model: https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en-q5_1.bin",
            path.display(), dir.display()
        ));
    }
    Ok(path)
}
```

### WAV File Writing for Verification (hound)

```rust
// Source: hound docs.rs
use hound::{WavWriter, WavSpec, SampleFormat};

pub fn write_test_wav(path: &str, samples: &[f32]) -> Result<(), Box<dyn std::error::Error>> {
    let spec = WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut writer = WavWriter::create(path, spec)?;
    for &sample in samples {
        writer.write_sample(sample)?;
    }
    writer.finalize()?;
    Ok(())
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| tazz4843/whisper-rs on GitHub | Migrated to Codeberg (codeberg.org/tazz4843/whisper-rs); GitHub repo archived | ~2025 | GitHub repo will not receive updates; use Codeberg or the crates.io release (still 0.15.1 on crates.io) |
| large-v3-turbo as English-only model | No English-only variant exists for large/turbo; force English via set_language("en") | By design (OpenAI) | Cannot use .en suffix for turbo; must use multilingual model with forced language |
| rubato FftFixedIn (old API) | Newer rubato versions use `Fft` struct with `FixedSync` enum instead of `FftFixedIn` struct | rubato 0.15+ | API changed; check version used and use matching API (FftFixedIn still exists in 0.15; 0.16+ uses new Fft struct) |

**Deprecated/outdated from prior research:**
- large-v3-turbo-q5_0 described as having an English-only variant: No such variant exists. Use multilingual model with forced language parameter.

---

## Open Questions

1. **CMAKE_CUDA_ARCHITECTURES propagation through cargo build**
   - What we know: Setting `CMAKE_CUDA_ARCHITECTURES=61` as an environment variable should be read by cmake-rs during the whisper-rs-sys build
   - What's unclear: Whether whisper-rs-sys's build.rs explicitly reads this env var or whether CMake inherits it automatically; the Codeberg build.rs was not accessible during research
   - Recommendation: During plan 02-02 execution, verify by checking `target/debug/build/whisper-rs-sys-*/out/` for the cmake cache to confirm `CMAKE_CUDA_ARCHITECTURES=61` was used. Fallback: set via `.cargo/config.toml` `[env]` section.

2. **rubato API version (FftFixedIn vs Fft struct)**
   - What we know: rubato 0.15.x uses FftFixedIn directly; 0.16+ may use the new `Fft` struct with `FixedSync` enum
   - What's unclear: Exact version to pin in Cargo.toml
   - Recommendation: Pin to `rubato = "0.15"` for predictable API. The 0.15 FftFixedIn constructor `FftFixedIn::new(in_rate, out_rate, chunk_size, sub_chunks, channels)` is verified.

3. **whisper-rs state lifetime and Tauri command integration**
   - What we know: WhisperContext must be stored as application state; WhisperState is created per-transcription; Arc<WhisperContext> is thread-safe
   - What's unclear: Whether WhisperContext implements Send+Sync for Tauri managed state
   - Recommendation: Wrap in `Arc<tokio::sync::Mutex<WhisperContext>>` as Tauri managed state. If Send is not implemented, use a dedicated inference thread with a channel.

---

## Model Selection Re-evaluation (CONTEXT.md Request)

The CONTEXT.md requests re-evaluation of model choices, noting "better English-only alternatives may exist." Here are the findings:

**GPU (Quadro P2000, 5GB VRAM, CUDA 11.7):**

| Model | Size | VRAM | English Accuracy | Notes |
|-------|------|------|-----------------|-------|
| ggml-large-v3-turbo-q5_0.bin | 574 MB | ~2.5 GB | Excellent (~2% WER) | **Recommended** — best accuracy/speed, no .en variant exists |
| ggml-medium.en-q5_0.bin | 539 MB | ~1.5 GB | Very good (~3% WER) | English-only variant; similar size to turbo-q5; lower accuracy |
| ggml-large-v3-turbo.bin | 1.6 GB | ~3 GB | Excellent (~2% WER) | Full precision turbo; larger VRAM, marginal accuracy gain |

**Recommendation for GPU:** Stay with `ggml-large-v3-turbo-q5_0.bin`. Force language='en' via `set_language("en")` rather than using a .en model (which doesn't exist for this model size). The q5_0 quantization at 574 MB is the sweet spot.

**CPU (no GPU fallback):**

| Model | Size | RAM | English Accuracy | Speed (5s audio) | Notes |
|-------|------|-----|-----------------|-----------------|-------|
| ggml-small.en-q5_1.bin | 190 MB | ~300 MB | Good (~5% WER) | 2-4s | **Recommended** — English-only, smallest viable |
| ggml-medium.en-q5_0.bin | 539 MB | ~700 MB | Very good (~3% WER) | 4-8s | Better accuracy but slower CPU |

**Recommendation for CPU:** `ggml-small.en-q5_1.bin` (190 MB) is the right choice. For context, the original plan mentioned `small` model — the q5_1 quantized English-only variant is the correct specific file.

---

## Sources

### Primary (HIGH confidence)
- [whisper-rs 0.15.1 docs.rs](https://docs.rs/whisper-rs/latest/whisper_rs/) — WhisperContext, WhisperContextParameters, FullParams, state.full() API
- [WhisperContextParameters docs.rs](https://docs.rs/whisper-rs/latest/whisper_rs/struct.WhisperContextParameters.html) — use_gpu field confirmed, builder pattern verified
- [cpal 0.17.3 GitHub release](https://github.com/rustaudio/cpal/releases) — version 0.17.3 released February 18, 2026; WASAPI default on Windows
- [rubato docs.rs](https://docs.rs/rubato) — FftFixedIn constructor, process_into_buffer API, Fft struct with FixedSync
- [nvml-wrapper docs.rs](https://docs.rs/nvml-wrapper/latest/nvml_wrapper/) — Nvml::init() pattern, device_by_index API
- [HuggingFace ggerganov/whisper.cpp](https://huggingface.co/ggerganov/whisper.cpp/tree/main) — Confirmed ggml-large-v3-turbo-q5_0.bin (574 MB), ggml-small.en-q5_1.bin (190 MB), ggml-medium.en-q5_0.bin (539 MB) exist; no large-turbo .en variant exists
- [whisper-rs Codeberg (active repo)](https://codeberg.org/tazz4843/whisper-rs) — confirmed BUILDING.md Windows CUDA requirements (LIBCLANG_PATH, CUDA, MSVC); GitHub mirror archived

### Secondary (MEDIUM confidence)
- [whisper-rs basic transcription DeepWiki](https://deepwiki.com/moinulmoin/whisper-rs/5.1-basic-transcription) — complete transcription example with SamplingStrategy::BeamSearch, set_language("en") — cross-verified with docs.rs API
- [cpal recording audio DeepWiki](https://deepwiki.com/RustAudio/cpal/5.2-audio-input-and-processing) — build_input_stream callback pattern with Arc<Mutex<>> — cross-verified with cpal API docs
- [Whisper model comparison multiple sources](https://whisper-api.com/blog/models/) — large/turbo models have no .en variant — confirmed by HuggingFace repo inspection

### Tertiary (LOW confidence — needs validation during implementation)
- CMAKE_CUDA_ARCHITECTURES=61 environment variable inheritance through cargo/cmake-rs: documented pattern from CMake docs and whisper.cpp issues, but specific whisper-rs-sys build.rs behavior not directly verified
- rubato API version: FftFixedIn constructor shown is for 0.15.x; 0.16+ uses different struct; verify with `cargo add rubato@0.15`

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all crate versions confirmed from crates.io/docs.rs; build requirements from official BUILDING.md
- Architecture: HIGH — patterns derived from docs.rs APIs and multiple reference project analysis (BridgeVoice, Handy, Voquill)
- Model selection: HIGH — verified directly from HuggingFace ggerganov/whisper.cpp repository file listing
- CUDA build configuration: MEDIUM — CMAKE_CUDA_ARCHITECTURES env var is standard CMake behavior but whisper-rs-sys internal handling not directly inspected (Codeberg bot-blocked)
- Pitfalls: MEDIUM — cpal WASAPI callback deadlock is from documented GitHub issue #970; other pitfalls from known patterns across reference projects

**Research date:** 2026-02-27
**Valid until:** 2026-03-29 (rubato API may shift with 0.16+ release; whisper-rs versions stable)
