---
phase: quick-12
plan: 01
subsystem: audio-pipeline
tags: [vad, silence-trimming, parakeet, whisper, audio-preprocessing]
dependency_graph:
  requires: []
  provides: [vad_trim_silence]
  affects: [pipeline.rs, vad.rs]
tech_stack:
  added: []
  patterns: [fresh-vad-instance-per-call, fail-open-fallback, buffer-shadowing]
key_files:
  created: []
  modified:
    - src-tauri/src/vad.rs
    - src-tauri/src/pipeline.rs
decisions:
  - "Fresh VoiceActivityDetector per vad_trim_silence call — no LSTM state contamination between calls"
  - "1-chunk (512 samples = 32ms) padding preserved on each side of speech boundaries — prevents onset/offset clipping"
  - "Fail-open on no-speech detection — returns full buffer rather than empty, prevents silent failures"
  - "Single trim point in pipeline.rs before engine dispatch — applies to both Whisper and Parakeet without per-engine changes"
metrics:
  duration: "208s"
  completed: "2026-03-02"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 2
---

# Quick Task 12: VAD Silence Trimming Summary

**One-liner:** Silero VAD post-hoc silence trim added to pipeline — leading/trailing silence stripped from audio before engine dispatch, with 1-chunk padding and fail-open fallback.

## What Was Built

Added `vad_trim_silence()` to `vad.rs` and wired it into `pipeline.rs` between the speech gate and engine dispatch. Every dictation recording now has leading and trailing silence removed before being sent to Whisper or Parakeet, improving transcription accuracy for short utterances.

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Add vad_trim_silence() to vad.rs | 70c66c4 | src-tauri/src/vad.rs |
| 2 | Integrate vad_trim_silence into pipeline.rs | 05266f5 | src-tauri/src/pipeline.rs |

## Implementation Details

### vad_trim_silence() (vad.rs)

- Creates fresh `VoiceActivityDetector` per call (same pattern as `vad_gate_check`)
- Iterates buffer in 512-sample chunks, records first and last speech chunks
- Applies 1-chunk (512 samples = 32ms) padding on each side clamped to buffer bounds
- Fail-open: returns full buffer if VAD init fails or no speech is detected
- Logs: `"VAD trim: speech chunks X-Y of Z (padded: A-B), trimmed N.N% (M -> K samples)"`

### pipeline.rs integration

- Inserted at line 139 (after `let _ = sample_count;`, before active engine read)
- `let samples = vad::vad_trim_silence(&samples);` shadows the owned `Vec<f32>`
- No imports needed — `vad::` already in scope via `use crate::vad;` at line 4
- Both Whisper and Parakeet engine dispatch paths receive the trimmed buffer

## Verification

- `cargo check --features "whisper,parakeet"` — passes, no warnings
- `cargo build --features "whisper,parakeet" --release` — passes, 1m 20s
- `vad_trim_silence` is `pub`, takes `&[f32]`, returns `Vec<f32>`
- `vad::vad_trim_silence(&samples)` appears in pipeline.rs between speech gate and engine dispatch

## Deviations from Plan

None — plan executed exactly as written.

## Self-Check: PASSED

- FOUND: src-tauri/src/vad.rs
- FOUND: src-tauri/src/pipeline.rs
- FOUND commit: 70c66c4 (vad_trim_silence function)
- FOUND commit: 05266f5 (pipeline integration)
