---
phase: 27-benchmark
plan: 01
subsystem: tooling
tags: [benchmark, whisper, parakeet, wav, tooling]
dependency_graph:
  requires: []
  provides: [benchmark-binary, wav-fixtures]
  affects: [developer-tooling]
tech_stack:
  added: []
  patterns: [standalone-binary, cargo-bin-target, wav-reading, gpu-detection]
key_files:
  created:
    - src-tauri/src/bin/benchmark.rs
    - test-fixtures/generate-benchmark-wavs.ps1
    - test-fixtures/benchmark-5s.wav
    - test-fixtures/benchmark-60s.wav
  modified:
    - src-tauri/Cargo.toml
decisions:
  - Use System.Speech.AudioFormat.SpeechAudioFormatInfo (not System.Speech.AudioFormatInfo or System.Speech.Synthesis.SpeechAudioFormatInfo) for TTS format specification on Windows
  - benchmark.rs directly uses whisper-rs and parakeet-rs APIs inline — does NOT import voice_to_text_lib to avoid pulling in Tauri
  - required-features = ["whisper", "parakeet"] on [[bin]] target ensures benchmark only builds with both features active
metrics:
  duration: "~2h (including full release build + benchmark run)"
  completed: "2026-03-03"
  tasks: 2
  files: 5
---

# Phase 27 Plan 01: Standalone Benchmark Binary Summary

Standalone `cargo run --bin benchmark` tool that loads all downloaded models and measures transcription latency across 5s and 60s WAV clips, printing an ASCII summary table with avg/min/max ms per model.

## What Was Built

### benchmark.rs (`src-tauri/src/bin/benchmark.rs`, 290 lines)

Standalone binary with no Tauri dependency. Key components:

- **WAV reader**: handles both Float32 and Int16 WAV formats, downmixes multi-channel to mono, linearly resamples to 16kHz if source differs
- **GPU detection**: uses `nvml_wrapper::Nvml` (same pattern as `detect_gpu()` in transcribe.rs) — returns `(use_gpu: bool, parakeet_provider: String)`
- **Model discovery**: checks `$APPDATA/VoiceType/models/` for all four known models, prints FOUND/MISSING for each
- **Whisper benchmarking**: `WhisperContextParameters` with `use_gpu()` + `flash_attn(true)`, fresh `WhisperState` per run, greedy sampling, 5 iterations per clip
- **Parakeet benchmarking**: CUDA or CPU ExecutionConfig, warm-up with 8000 silent samples, `transcribe_samples()` with `TimestampMode::Sentences`
- **Summary table**: ASCII table with 70-char width, avg/min/max columns, transcription sample per model/clip
- **Feature gating**: `#[cfg(feature = "whisper")]` and `#[cfg(feature = "parakeet")]` throughout

### generate-benchmark-wavs.ps1 (`test-fixtures/generate-benchmark-wavs.ps1`)

PowerShell TTS script using `System.Speech.AudioFormat.SpeechAudioFormatInfo` to generate 16kHz/16-bit/mono PCM WAV files. Generates a ~6.8s clip (short phrase) and ~83.6s clip (15-sentence technical passage).

### Cargo.toml addition

```toml
[[bin]]
name = "benchmark"
path = "src/bin/benchmark.rs"
required-features = ["whisper", "parakeet"]
```

## Benchmark Results (2026-03-03, Quadro P2000 CUDA)

| Model                    | Clip | Avg (ms) | Min (ms) | Max (ms) |
|--------------------------|------|----------|----------|----------|
| whisper-small-en         | 5s   | 502      | 437      | 704      |
| whisper-small-en         | 60s  | 2815     | 2775     | 2872     |
| whisper-large-v3-turbo   | 5s   | 1316     | 1304     | 1323     |
| whisper-large-v3-turbo   | 60s  | 6397     | 6300     | 6529     |
| whisper-distil-large-v3.5 | 5s  | 1324     | 1308     | 1332     |
| whisper-distil-large-v3.5 | 60s | 6197     | 6131     | 6276     |
| parakeet-tdt-v2          | 5s   | 501      | 457      | 548      |
| parakeet-tdt-v2          | 60s  | 7908     | 7066     | 10642    |

**Key observations:**
- whisper-small-en is fastest for short clips (437ms min) and scales well (2.8s for 83.6s audio = 30x real-time)
- whisper-large-v3-turbo and distil-large-v3.5 have nearly identical 5s latency (~1310-1332ms) despite different decoder layer counts
- distil-large-v3.5 is ~3% faster than large-v3-turbo on the 60s clip (6197ms vs 6397ms)
- parakeet matches small-en on the 5s clip (501ms vs 502ms avg) but is slower on 60s (7.9s vs 2.8s)
- parakeet 60s run 1 was 10642ms (CUDA warmup effect on first long clip); runs 2-5 settled at 7.1-7.4s

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] whisper-rs API mismatch: full_n_segments() returns i32, not Result**
- **Found during:** Task 1 (cargo check)
- **Issue:** Plan template used `.unwrap_or(0)` on `full_n_segments()` result, but it returns `i32` directly
- **Fix:** Removed `.unwrap_or(0)` — use the i32 directly in the range
- **Files modified:** src-tauri/src/bin/benchmark.rs

**2. [Rule 1 - Bug] whisper-rs API mismatch: full_get_segment_text does not exist**
- **Found during:** Task 1 (cargo check)
- **Issue:** Plan referenced `state.full_get_segment_text(s)` but the actual API is `state.get_segment(s)` returning `Option<WhisperSegment>` with `.to_str()`
- **Fix:** Replaced with `state.get_segment(s)` and `.to_str()` pattern matching the implementation in transcribe.rs
- **Files modified:** src-tauri/src/bin/benchmark.rs

**3. [Rule 1 - Bug] parakeet-rs from_pretrained requires AsRef<Path>, not &Cow<str>**
- **Found during:** Task 1 (cargo check)
- **Issue:** `&parakeet_path.to_string_lossy()` produces `&Cow<str>` which doesn't implement `AsRef<Path>`
- **Fix:** Changed to `&*parakeet_path.to_string_lossy()` to deref to `&str` which implements `AsRef<Path>`
- **Files modified:** src-tauri/src/bin/benchmark.rs

**4. [Rule 1 - Bug] TranscriptionResult is a struct with .text field, not a Vec**
- **Found during:** Task 1 (cargo check)
- **Issue:** Plan code called `.iter().map(|s| s.text...)` on the result as if it were `Vec<Segment>`, but `TranscriptionResult` has `text: String` and `tokens: Vec<TimedToken>` fields
- **Fix:** Changed to `result.text.trim().to_string()` directly
- **Files modified:** src-tauri/src/bin/benchmark.rs

**5. [Rule 1 - Bug] PowerShell System.Speech type name incorrect**
- **Found during:** Task 2 (WAV generation)
- **Issue:** Plan referenced `[System.Speech.AudioFormatInfo]::new(EncodingFormat, ...)` — this type does not exist. Plan's fallback suggestion `[System.Speech.Synthesis.SpeechAudioFormatInfo]` also doesn't exist.
- **Fix:** Enumerated actual types from the System.Speech assembly; correct type is `[System.Speech.AudioFormat.SpeechAudioFormatInfo]::new(samplesPerSecond, AudioBitsPerSample, AudioChannel)`
- **Files modified:** test-fixtures/generate-benchmark-wavs.ps1

## Self-Check: PASSED

Files verified:
- src-tauri/src/bin/benchmark.rs: EXISTS (290 lines, > 150 minimum)
- src-tauri/Cargo.toml: contains `name = "benchmark"` — VERIFIED
- test-fixtures/generate-benchmark-wavs.ps1: EXISTS (46 lines, > 15 minimum)
- test-fixtures/benchmark-5s.wav: EXISTS (16kHz, 16-bit, mono PCM — VERIFIED)
- test-fixtures/benchmark-60s.wav: EXISTS (16kHz, 16-bit, mono PCM — VERIFIED)

Commits verified:
- e320bed: feat(27-01): add standalone benchmark binary and WAV generation script
- 7ee1651: feat(27-02): generate 16kHz benchmark WAV fixtures and validate end-to-end
