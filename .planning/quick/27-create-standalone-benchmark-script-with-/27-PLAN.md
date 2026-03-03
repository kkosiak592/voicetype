---
phase: 27-benchmark
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/Cargo.toml
  - src-tauri/src/bin/benchmark.rs
  - test-fixtures/generate-benchmark-wavs.ps1
autonomous: true
requirements: [BENCH-01]

must_haves:
  truths:
    - "Running `cargo run --bin benchmark` produces a summary table with avg/min/max latency per model"
    - "Each downloaded model is tested 5 times on both a 5s and 60s WAV clip"
    - "Models that are not downloaded are skipped with a message, not an error"
    - "WAV generation script creates valid 16kHz 16-bit mono WAV files of approximately 5s and 60s duration"
  artifacts:
    - path: "src-tauri/src/bin/benchmark.rs"
      provides: "Standalone benchmark binary"
      min_lines: 150
    - path: "src-tauri/Cargo.toml"
      provides: "[[bin]] target for benchmark"
      contains: "name = \"benchmark\""
    - path: "test-fixtures/generate-benchmark-wavs.ps1"
      provides: "PowerShell TTS WAV generation script"
      min_lines: 15
  key_links:
    - from: "src-tauri/src/bin/benchmark.rs"
      to: "whisper-rs"
      via: "direct crate dependency (load_whisper_context pattern)"
      pattern: "WhisperContext"
    - from: "src-tauri/src/bin/benchmark.rs"
      to: "parakeet-rs"
      via: "direct crate dependency (ParakeetTDT::from_pretrained)"
      pattern: "ParakeetTDT"
    - from: "src-tauri/src/bin/benchmark.rs"
      to: "test-fixtures/*.wav"
      via: "hound WAV reader"
      pattern: "WavReader::open"
---

<objective>
Create a standalone benchmark binary that loads each downloaded model and measures transcription latency across 5s and 60s test WAV clips, printing a summary table.

Purpose: Enables latency comparison across all available models (whisper small-en, large-v3-turbo, distil-large-v3.5, parakeet-tdt-v2) without running the full Tauri app.
Output: `cargo run --bin benchmark` from src-tauri/ prints a formatted table with avg/min/max ms per model per clip.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md

<interfaces>
<!-- From src-tauri/src/transcribe.rs — reuse patterns but NOT the functions directly (they live in the lib crate which pulls in Tauri). The benchmark binary uses whisper-rs and parakeet-rs directly. -->

Key patterns to replicate in benchmark.rs:
```rust
// Whisper loading (from transcribe.rs):
let mut ctx_params = WhisperContextParameters::default();
ctx_params.use_gpu(use_gpu);
ctx_params.flash_attn(true);
let ctx = WhisperContext::new_with_params(model_path, ctx_params)?;

// Whisper inference (from transcribe.rs):
let mut state = ctx.create_state()?;
let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
params.set_language(Some("en"));
params.set_temperature(0.0);
params.set_single_segment(false); // allow multi-segment for 60s clip
params.set_no_context(true);
state.full(params, &audio)?;

// Parakeet loading (from transcribe_parakeet.rs):
let parakeet = ParakeetTDT::from_pretrained(model_dir, config)?;

// Parakeet inference (from transcribe_parakeet.rs):
let result = parakeet.transcribe_samples(audio_vec, 16000, 1, Some(TimestampMode::Sentences))?;

// WAV reading (from lib.rs:1325 — only available in debug builds, so replicate in benchmark):
let reader = hound::WavReader::open(path)?;
// Handle both Float and Int sample formats, downmix to mono

// Models directory:
let appdata = std::env::var("APPDATA")?;
let models_dir = PathBuf::from(appdata).join("VoiceType").join("models");
```

Model files to check:
- `ggml-small.en-q5_1.bin` (whisper small-en, CPU)
- `ggml-large-v3-turbo-q5_0.bin` (whisper large-v3-turbo, GPU)
- `ggml-distil-large-v3.5.bin` (whisper distil-large-v3.5, GPU)
- `parakeet-tdt-v2-fp32/` (directory, parakeet ONNX model)
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Create WAV generation script and benchmark binary scaffolding</name>
  <files>test-fixtures/generate-benchmark-wavs.ps1, src-tauri/Cargo.toml, src-tauri/src/bin/benchmark.rs</files>
  <action>
**Step 1: PowerShell WAV generation script** (`test-fixtures/generate-benchmark-wavs.ps1`)

Create a PowerShell script that uses `System.Speech.Synthesis.SpeechSynthesizer` to generate two WAV files:
- `test-fixtures/benchmark-5s.wav` — A short phrase like "The quick brown fox jumps over the lazy dog." (aim for ~5 seconds of speech)
- `test-fixtures/benchmark-60s.wav` — A longer passage of ~60 seconds. Use a multi-paragraph passage repeated/extended to fill ~60s of TTS output. For example, repeat variations of common English dictation text (e.g., multiple sentences from different domains: tech, business, casual).

Script requirements:
- Set output format: 16kHz, 16-bit, mono PCM WAV (use `[System.Speech.AudioFormatInfo]::new([System.Speech.Synthesis.SpeakOutputFormat]::Riff16Khz16BitMono)` or equivalent — set `SpeechSynthesizer.SetOutputToWaveFile(path, format)` with AudioFormatInfo specifying 16kHz, 16 bits, 1 channel)
- Actually the simplest approach: use `$synth.SetOutputToWaveFile($path)` which outputs WAV at the default rate, then we handle resampling in the Rust reader. BUT the benchmark binary needs 16kHz input. Better approach: use `[System.Speech.AudioFormatInfo]` constructor with `EncodingFormat=Pcm, SamplesPerSecond=16000, BitsPerSample=16, ChannelCount=1`. The class is `System.Speech.AudioFormatInfo(System.Speech.Synthesis.EncodingFormat, int samplesPerSec, int bitsPerSample, int channelCount)`. Call `$synth.SetOutputToWaveFile($path, $format)`.
- Add `Add-Type -AssemblyName System.Speech` at the top
- Print file sizes after generation
- Overwrite existing files (the current ones are 22050Hz and wrong duration)

**Step 2: Add `[[bin]]` target** to `src-tauri/Cargo.toml`

Add after the `[lib]` section:

```toml
[[bin]]
name = "benchmark"
path = "src/bin/benchmark.rs"
# Standalone benchmark binary — does NOT depend on Tauri.
# Run: cargo run --bin benchmark --features whisper,parakeet --release
```

**Step 3: Create `src-tauri/src/bin/benchmark.rs`**

Create the directory `src-tauri/src/bin/` first.

The benchmark binary should:

1. **WAV reader function** — `read_wav_to_f32(path: &str) -> Result<(Vec<f32>, u32), String>` that handles both Float and Int WAV formats, downmixes to mono. Replicate the pattern from lib.rs:1325 (it's gated behind `#[cfg(debug_assertions)]` so we cannot import it). If the WAV sample rate is not 16000, resample using simple linear interpolation (good enough for benchmark purposes) or just require 16kHz input and error if not.

2. **GPU detection** — Use `nvml_wrapper::Nvml` to detect NVIDIA GPU (same pattern as `detect_gpu()` in transcribe.rs). Set `use_gpu = true` if NVIDIA found, false otherwise. Print detection result.

3. **Model discovery** — Check `$APPDATA/VoiceType/models/` for each known model file:
   - `ggml-small.en-q5_1.bin` (label: "whisper-small-en")
   - `ggml-large-v3-turbo-q5_0.bin` (label: "whisper-large-v3-turbo")
   - `ggml-distil-large-v3.5.bin` (label: "whisper-distil-large-v3.5")
   - `parakeet-tdt-v2-fp32/` directory (label: "parakeet-tdt-v2")

   Print which models are found and which are missing. Only benchmark found models.

4. **Benchmark loop** — For each found model, for each WAV clip (5s, 60s):
   - Load model once (print load time)
   - For Parakeet: run warm-up inference first (silent audio, discard result)
   - Run 5 transcription iterations, recording wall-clock time for each
   - Collect transcription text from first run (for sanity checking)
   - Compute avg/min/max latency across 5 runs

5. **Summary table** — Print a formatted ASCII table:
   ```
   ============================================================
   BENCHMARK RESULTS
   ============================================================
   Model                     | Clip | Avg (ms) | Min (ms) | Max (ms)
   --------------------------|------|----------|----------|--------
   whisper-small-en          | 5s   |     342  |     330  |     360
   whisper-small-en          | 60s  |    2150  |    2100  |    2200
   whisper-large-v3-turbo    | 5s   |     180  |     170  |     195
   ...
   ```

6. **Whisper inference** — For each whisper model:
   - Create `WhisperContextParameters` with `use_gpu(use_gpu)` and `flash_attn(true)`
   - Load with `WhisperContext::new_with_params(path, params)`
   - For each run: create fresh `WhisperState` via `ctx.create_state()`, set `FullParams` with `Greedy { best_of: 1 }`, language "en", temperature 0.0, `set_single_segment(false)` (60s clip has multiple segments), `set_no_context(true)`, `set_print_special/progress/realtime/timestamps(false)`, `set_temperature_inc(0.0)`
   - Call `state.full(params, &audio)`, collect segments

7. **Parakeet inference** — For parakeet model:
   - Detect provider: "cuda" if NVIDIA, else check for discrete GPU via DXGI (or just use "cpu" for simplicity in the benchmark — avoid pulling in the windows crate DXGI code). Actually, for simplicity: if NVIDIA detected use "cuda", otherwise use "cpu". DirectML detection is complex and not critical for benchmark.
   - Create `ExecutionConfig::new().with_execution_provider(ExecutionProvider::Cuda)` or None for CPU
   - Load with `ParakeetTDT::from_pretrained(dir, config)`
   - Warm up with 8000 zero samples
   - For each run: `parakeet.transcribe_samples(audio.clone(), 16000, 1, Some(TimestampMode::Sentences))`
   - Note: `transcribe_samples` takes `&mut self` so no concurrency issue in single-threaded benchmark

8. **CLI** — No external CLI library needed. Just parse args manually or use none (hardcoded paths). WAV file paths: look for `test-fixtures/benchmark-5s.wav` and `test-fixtures/benchmark-60s.wav` relative to the working directory. If not found, try `../test-fixtures/` (when running from src-tauri/). Print an error with instructions to run the PowerShell script if WAVs not found.

9. **Imports** — The binary needs:
   ```rust
   use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};
   use parakeet_rs::{ExecutionConfig, ExecutionProvider, ParakeetTDT, TimestampMode};
   use hound;
   use std::path::PathBuf;
   use std::time::Instant;
   ```

   The binary does NOT import from `voice_to_text_lib` — it is fully self-contained. This avoids pulling in Tauri, cpal, and other heavy dependencies.

10. **Feature gating** — Wrap whisper code in `#[cfg(feature = "whisper")]` and parakeet code in `#[cfg(feature = "parakeet")]`. This matches the existing Cargo.toml feature flags. Both are enabled by default.

11. **Error handling** — Use simple `fn main()` with manual error printing (no anyhow/thiserror). Print errors to stderr and continue to next model. The benchmark should not abort on a single model failure.
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text/src-tauri && cargo check --bin benchmark --features whisper,parakeet 2>&1 | tail -5</automated>
  </verify>
  <done>
  - `cargo check --bin benchmark` compiles without errors
  - PowerShell script exists and is runnable
  - benchmark.rs contains WAV reading, model discovery, whisper+parakeet benchmarking, and table output
  </done>
</task>

<task type="auto">
  <name>Task 2: Generate WAV fixtures and run the benchmark end-to-end</name>
  <files>test-fixtures/benchmark-5s.wav, test-fixtures/benchmark-60s.wav</files>
  <action>
**Step 1: Run the PowerShell WAV generation script**

```powershell
powershell -ExecutionPolicy Bypass -File test-fixtures/generate-benchmark-wavs.ps1
```

Verify the output files:
- `test-fixtures/benchmark-5s.wav` should be ~160KB (5s * 16000Hz * 2 bytes = 160,000 bytes + 44 byte header)
- `test-fixtures/benchmark-60s.wav` should be ~1.9MB (60s * 16000Hz * 2 bytes = 1,920,000 bytes + 44 byte header)

If the script fails (e.g., System.Speech not available), generate the WAVs programmatically in Rust instead: add a `--generate` flag to the benchmark binary that creates synthetic WAV files using the `hound` crate with a sine wave tone (440Hz) at 16kHz mono. This is a fallback — TTS speech is preferred for realistic benchmarking.

**Step 2: Run the benchmark in release mode**

```bash
cd src-tauri && cargo run --bin benchmark --features whisper,parakeet --release
```

This will take several minutes depending on how many models are downloaded. The benchmark should:
- Print GPU detection result
- List found/missing models
- For each found model: print load time, then 5x transcription times per clip
- Print the summary table at the end

**Step 3: Verify output**

Confirm the summary table is printed with avg/min/max columns. If any model fails, verify the error is reported gracefully and other models still run.

If compilation or runtime errors occur, fix them in `benchmark.rs`. Common issues:
- whisper-rs API differences: check that `state.get_segment(i)` returns the right type (it returns `Option<&WhisperSegment>` — call `.to_str()` or `.text()` depending on version)
- parakeet-rs `transcribe_samples` signature: takes `(Vec<f32>, u32, u16, Option<TimestampMode>)` — the audio must be owned (clone it)
- hound WAV reading: `reader.spec()` returns by value, not reference
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && powershell -ExecutionPolicy Bypass -File test-fixtures/generate-benchmark-wavs.ps1 && cd src-tauri && cargo run --bin benchmark --features whisper,parakeet --release 2>&1 | tail -20</automated>
  </verify>
  <done>
  - WAV files exist at correct sizes (~160KB for 5s, ~1.9MB for 60s) at 16kHz 16-bit mono
  - `cargo run --bin benchmark --release` completes without crashes
  - Summary table is printed with avg/min/max latency for each downloaded model
  - Models not present on disk are skipped with an informational message
  </done>
</task>

</tasks>

<verification>
1. `cargo check --bin benchmark --features whisper,parakeet` compiles cleanly
2. WAV files in test-fixtures/ are 16kHz, 16-bit, mono
3. `cargo run --bin benchmark --release` prints a summary table
4. Skipped models show an informational message, not a crash
</verification>

<success_criteria>
Running `cargo run --bin benchmark --features whisper,parakeet --release` from src-tauri/ produces a formatted table showing avg/min/max transcription latency (in ms) for each downloaded model across both 5s and 60s clips, with 5 iterations per measurement.
</success_criteria>

<output>
After completion, create `.planning/quick/27-create-standalone-benchmark-script-with-/27-SUMMARY.md`
</output>
