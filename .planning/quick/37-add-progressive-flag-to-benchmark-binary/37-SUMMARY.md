---
phase: quick-37
plan: 37
subsystem: benchmark
tags: [benchmark, progressive, latency, vad]
dependency_graph:
  requires: []
  provides: [progressive-benchmark-mode]
  affects: [src-tauri/src/bin/benchmark.rs]
tech_stack:
  added: []
  patterns: [run_progressive generic closure helper, simulated real-time chunk dispatch]
key_files:
  modified:
    - src-tauri/src/bin/benchmark.rs
decisions:
  - "Progressive mode uses std::thread::sleep for real-time simulation (not tokio) — sync binary"
  - "run_progressive generic over FnMut closure avoids duplicating simulation logic across 7 engine sections"
  - "audio_for_progressive cloned before VAD chunking to preserve original for progressive use"
  - "Progressive runs 1 iteration only (simulates real-time, multiple iterations waste time)"
  - "vad_chunk_audio called unconditionally in run_progressive — even 5s clips always chunked"
metrics:
  duration_minutes: 20
  completed: "2026-03-04"
  tasks_completed: 1
  files_modified: 1
---

# Quick 37: Add --progressive Flag to Benchmark Binary

One-liner: --progressive flag simulates VAD-driven real-time chunk dispatch with post-release latency metric across all 7 benchmark engine sections.

## What Was Built

Added `--progressive` flag to `src-tauri/src/bin/benchmark.rs` that enables a new progressive transcription simulation mode alongside the existing batch benchmark. When active, each WAV is VAD-chunked unconditionally (even 5s clips), chunks are dispatched with simulated real-time audio availability (sleeping until each chunk's end-of-audio timestamp), and transcription runs sequentially as chunks become available. The key new metric is "post-release latency" — time from the last chunk becoming available to final transcription completion — directly measuring user-perceived delay in a progressive dictation pipeline.

## Key Changes

**New struct `ProgressiveResult`** (alongside `BenchResult`):
- `total_ms`: wall-clock time for all chunks
- `post_release_ms`: time from last chunk available to transcription done (can be negative)
- `num_chunks`: VAD chunk count
- `wer`: WER of concatenated output
- `first_text`: combined transcription text

**`run_progressive<F>()` generic function**:
- Calls `vad_chunk_audio()` unconditionally (no >30s gate)
- Computes `chunk_available_at_ms[i] = cumulative_samples / 16.0` for each chunk
- Sleeps until each chunk's availability time before transcribing
- Tracks total wall time and post-release latency
- Engine-specific transcription passed as `FnMut(&[f32]) -> Result<String, String>`

**CLI flag**: `--progressive` parsed in arg loop, prints "Mode: progressive (simulated real-time dispatch)" in header.

**Engine wiring** — progressive block added after batch results.push() for all 7 engines:
- Whisper: creates state per chunk, runs `state.full()`, collects segment text
- Parakeet: calls `parakeet.transcribe_samples()`
- Moonshine tiny/base: calls `engine.transcribe_samples()`
- Moonshine streaming tiny/small/medium: full streaming pipeline (process_audio_chunk, encode, compute_cross_kv, decode_step loop, decode_tokens)
- SenseVoice: calls `engine.transcribe_samples()`

**Output tables** — added when `progressive_results` is non-empty:
- "PROGRESSIVE vs BATCH COMPARISON" flat table (model, clip, batch_avg, prog_total, post_release_ms, chunks, dWER)
- "POST-RELEASE LATENCY BY DURATION" pivot table (rows=models, cols=5s/30s/60s/90s)
- Equivalent sections in `write_markdown_report()`

**Signature change**: `print_summary(results, progressive)` and `write_markdown_report(results, progressive)` — both updated to accept `&[ProgressiveResult]`.

## Deviations from Plan

**[Rule 1 - Bug] Audio ownership: clone before VAD chunk**
- Found during: Task 1 (implementing all engine sections)
- Issue: Original code used `vec![audio]` which moves `audio` into the chunks vec, leaving no original for `run_progressive`. All engine sections used this pattern.
- Fix: Renamed `audio` to `audio_for_progressive` and used `vec![audio_for_progressive.clone()]` when not chunking, preserving the original for progressive use.
- Files modified: src-tauri/src/bin/benchmark.rs
- Commit: 0bf1333

## Verification

cargo check result: `Finished 'release' profile [optimized] target(s) in 28.31s` — clean compile with whisper,parakeet,bench_extra features. One dead_code warning for `first_text` field (non-blocking; field is stored for completeness per plan spec).

## Self-Check: PASSED

- `src-tauri/src/bin/benchmark.rs` modified: confirmed
- Commit 0bf1333 exists: confirmed
- `--progressive` flag parsed: confirmed (line ~609 in updated file)
- `ProgressiveResult` struct defined: confirmed
- `run_progressive` function defined: confirmed
- Progressive blocks wired into all 7 engine sections: confirmed
- `print_summary` and `write_markdown_report` updated with progressive parameter: confirmed
- Comparison and pivot tables added: confirmed
