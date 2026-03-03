---
phase: quick-31
plan: 01
subsystem: benchmark
tags: [vad, chunking, benchmark, moonshine, sensevoice]
dependency_graph:
  requires: [voice_activity_detector crate, bench_extra feature, transcribe-rs]
  provides: [vad_chunk_audio function, chunked benchmark for short-context models]
  affects: [benchmark binary bench_extra loops]
tech_stack:
  added: []
  patterns: [VAD-based audio segmentation before inference for short-context models]
key_files:
  created: []
  modified:
    - src-tauri/src/bin/benchmark.rs
decisions:
  - VAD chunking runs once per clip outside iteration loop; chunks reused across all 5 iterations
  - VAD time printed separately from per-iteration inference latency for fair cross-model comparison
  - 320ms silence gap (10 chunks at 512 samples/chunk) balances splitting accuracy vs over-segmentation
  - 30s max segment cap matches Moonshine training window and SenseVoice inference limit
  - 0.5s minimum segment discards sub-word fragments from noise
metrics:
  duration: 2m
  completed: "2026-03-03"
---

# Quick Task 31: Add VAD-Based Chunking to Benchmark Summary

Silero VAD chunking for Moonshine/SenseVoice benchmark loops -- clips >30s split at silence boundaries before per-segment transcription with concatenated WER.

## What Was Done

### Task 1: Add vad_chunk_audio function and wire into bench_extra loops

Added `vad_chunk_audio()` function gated behind `#[cfg(feature = "bench_extra")]` that:
- Runs Silero VAD over entire audio in 512-sample chunks
- Tracks speech/silence state transitions
- Splits at silence gaps >= 320ms (10 consecutive below-threshold chunks)
- Caps segments at 30s (480,000 samples) to stay within model context windows
- Discards segments < 0.5s (8,000 samples) as noise fragments
- Falls back to single-segment if VAD init fails or chunking produces no valid segments

Modified all three bench_extra loops (moonshine-tiny, moonshine-base, sensevoice-small):
- Added `needs_chunking = audio.len() > 30 * 16000` threshold guard
- Clips <= 30s pass through unchanged (single-element vec, no VAD overhead)
- Clips > 30s: VAD runs once, segments reused across all 5 iterations
- Per-iteration timing wraps transcription of all segments (not VAD)
- VAD time printed separately; total (VAD + avg) printed for chunked clips
- Segment transcription text concatenated with space separator for WER

**Commit:** `2dfaeef`

## Deviations from Plan

None -- plan executed exactly as written.

## Verification

- `cargo check --bin benchmark --features whisper,parakeet,bench_extra` compiles without errors
- `vad_chunk_audio` function definition: 1 match (line 128)
- `vad_chunk_audio` call sites: 3 matches (moonshine-tiny, moonshine-base, sensevoice-small)
- `needs_chunking` threshold guard: present in all 3 loops
- `VoiceActivityDetector` import under `bench_extra` cfg gate confirmed

## Self-Check: PASSED
