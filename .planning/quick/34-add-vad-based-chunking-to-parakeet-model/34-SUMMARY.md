---
phase: quick-34
plan: 34
subsystem: benchmark
tags: [benchmark, parakeet, vad, chunking, performance]
dependency_graph:
  requires: []
  provides: [parakeet-vad-chunking]
  affects: [src-tauri/src/bin/benchmark.rs]
tech_stack:
  added: []
  patterns: [vad-chunking-before-inference, chunk-text-concat-with-spaces]
key_files:
  created: []
  modified:
    - src-tauri/src/bin/benchmark.rs
decisions:
  - Widened vad_chunk_audio cfg gate to any(bench_extra, parakeet) rather than adding a new function — avoids code duplication and voice_activity_detector is already an unconditional dep
metrics:
  duration: "~5 minutes"
  completed: "2026-03-04T00:26:12Z"
  tasks: 1
  files: 1
---

# Quick Task 34: Add VAD-based chunking to parakeet benchmark section

**One-liner:** Parakeet benchmark now VAD-chunks audio >30s into silence-split segments before transcription, matching the moonshine/sensevoice pattern, by widening the existing vad_chunk_audio cfg gate to include the parakeet feature.

## What Was Done

Single task: widened two cfg gates (`VoiceActivityDetector` import and `vad_chunk_audio` fn) from `#[cfg(feature = "bench_extra")]` to `#[cfg(any(feature = "bench_extra", feature = "parakeet"))]`, then replaced the parakeet per-clip loop body with the standard chunking pattern already used by moonshine-tiny/base, sensevoice, and moonshine-streaming-*.

## Changes

### src-tauri/src/bin/benchmark.rs

**cfg gate widening (lines ~31, ~128):**
- `VoiceActivityDetector` import: `bench_extra` → `any(bench_extra, parakeet)`
- `vad_chunk_audio` fn: `bench_extra` → `any(bench_extra, parakeet)`

**Parakeet inner loop replacement:**
Before reading the iteration loop, added:
```rust
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
```

The iteration loop now iterates over chunks per run, concatenating segment texts with spaces, with per-segment error reporting.

## Verification

- `cargo check --bin benchmark --features whisper,parakeet --release`: PASSED (no errors)
- `cargo check --bin benchmark --features whisper,parakeet,bench_extra --release`: PASSED (no duplicate symbol from widened gate)

## Commits

| Hash | Message |
|------|---------|
| 21d4441 | feat(quick-34): add VAD chunking to parakeet benchmark section |

## Deviations from Plan

None - plan executed exactly as written.

## Self-Check: PASSED

- File modified: `src-tauri/src/bin/benchmark.rs` — confirmed
- Commit 21d4441 — confirmed
- Both feature combinations compile cleanly — verified
