---
phase: quick-35
plan: 01
subsystem: benchmark
tags: [benchmark, whisper, moonshine-streaming, vad, fairness]
dependency_graph:
  requires: []
  provides: [fair-whisper-benchmark, incremental-streaming-benchmark]
  affects: [src-tauri/src/bin/benchmark.rs]
tech_stack:
  added: []
  patterns: [vad-chunking, incremental-frame-feeding, greedy-decode-argmax]
key_files:
  modified:
    - src-tauri/src/bin/benchmark.rs
decisions:
  - "decode_step (pub) + inline argmax used instead of private decode_step_greedy for streaming inference"
  - "STREAMING_FRAME_SAMPLES=5120 (320ms at 16kHz) chosen to approximate real-world audio capture cadence"
  - "model_opt pattern (Option<StreamingModel>) used instead of continue/break since streaming sections are in if-let blocks not for loops"
metrics:
  duration: ~15 minutes
  completed: 2026-03-04T01:51:35Z
  tasks: 2
  files_modified: 1
---

# Phase quick-35 Plan 01: Fix Benchmark Fairness — Whisper VAD Chunking and Streaming Incremental Feed Summary

**One-liner:** Whisper benchmark now VAD-chunks >30s clips matching all other models; streaming Moonshine sections feed audio in 320ms frames via low-level StreamingModel API with time-to-first-partial reporting.

## What Was Built

### Task 1: Add VAD chunking to Whisper benchmark section (commit: 85b53e8)

The Whisper benchmark was the only model section that fed raw audio directly without VAD chunking for long clips. This made Whisper appear faster than reality on 60s clips (one big buffer) while all other models (Moonshine, SenseVoice, Parakeet) chunked via `vad_chunk_audio`.

Changes:
- `#[cfg(any(feature = "bench_extra", feature = "parakeet"))]` gates updated to add `feature = "whisper"` for both the `VoiceActivityDetector` import and the `vad_chunk_audio` function definition
- Whisper benchmark inner loop now computes VAD chunks if `audio.len() > 30 * 16000`, then iterates over segments
- Each chunk gets its own `FullParams` and `WhisperState` (required — `state.full()` consumes params)
- VAD overhead printed alongside inference time; `(total incl. VAD: avg=Xms)` printed after stats

### Task 2: Switch streaming Moonshine to incremental frame feeding (commit: bdb3c14)

All three streaming sections (tiny/small/medium) previously called `engine.transcribe_samples()` which internally calls `model.generate()` — passing the full audio buffer at once, identical to batch mode. This defeated the purpose of benchmarking a streaming model.

Changes:
- Import `transcribe_rs::engines::moonshine::streaming_model::StreamingModel` directly
- Add `STREAMING_FRAME_SAMPLES: usize = 5120` constant (320ms frames at 16kHz)
- Remove unused `MoonshineStreamingEngine` and `StreamingModelParams` from imports
- Replace all three streaming sections with direct `StreamingModel` usage:
  1. Load via `StreamingModel::new(mpath, 0, providers)`
  2. For each segment: `create_state()`, feed in 5120-sample frames via `process_audio_chunk()`, `encode(is_final=true)`, `compute_cross_kv()`, autoregressive decode via `decode_step()` + inline argmax
  3. Track `first_partial_ms` (time to first decoded token) and print alongside total
- VAD chunking for >30s clips preserved; applies per segment before incremental feeding
- `decode_step_greedy` is private in `StreamingModel` — used public `decode_step` returning full logits + inline argmax via `.max_by()`

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Used `decode_step` + inline argmax instead of `decode_step_greedy`**
- **Found during:** Task 2 implementation
- **Issue:** Plan mentioned `decode_step_greedy` as the method to use, but it's declared `fn` (private) in `streaming_model.rs`. Only `decode_step` (returning full logits `Vec<f32>`) is `pub`.
- **Fix:** Used `decode_step()` + `.iter().enumerate().max_by(...)` argmax inline — equivalent result, slightly more allocation but functionally identical to the greedy path.
- **Files modified:** src-tauri/src/bin/benchmark.rs
- **Commit:** bdb3c14

**2. [Rule 2 - Pattern] Used `Option<StreamingModel>` pattern instead of `continue`**
- **Found during:** Task 2 implementation
- **Issue:** Plan noted that streaming sections are in `if let Some(ref mpath)` blocks (not `for` loops), so `continue` cannot skip to the next model on load failure. Plan suggested a flag pattern.
- **Fix:** Used `Option<StreamingModel>` with an inner `if let Some(ref mut model)` guard — clean Rust pattern that handles the load-fail case without needing `continue` or artificial flags.
- **Files modified:** src-tauri/src/bin/benchmark.rs
- **Commit:** bdb3c14

## Verification

```
cargo check --bin benchmark --features whisper,parakeet        -> Finished (0 errors)
cargo check --bin benchmark --features whisper,parakeet,bench_extra -> Finished (0 errors)
```

## Self-Check: PASSED

- src-tauri/src/bin/benchmark.rs: modified and verified compiling
- Commits 85b53e8 and bdb3c14 exist in git log
- Both feature flag combinations compile cleanly
