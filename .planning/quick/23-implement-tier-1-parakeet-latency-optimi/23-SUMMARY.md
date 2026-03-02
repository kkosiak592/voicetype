---
phase: quick-23
plan: "01"
subsystem: backend/parakeet
tags: [latency, cuda, parakeet, warmup, tf32]
dependency_graph:
  requires: []
  provides: [LATENCY-CUDA-EP, LATENCY-WARMUP]
  affects: [src-tauri/patches/parakeet-rs/src/execution.rs, src-tauri/src/transcribe_parakeet.rs, src-tauri/src/lib.rs]
tech_stack:
  added: []
  patterns: [background-warmup-thread, cuda-tf32]
key_files:
  modified:
    - src-tauri/patches/parakeet-rs/src/execution.rs
    - src-tauri/src/transcribe_parakeet.rs
    - src-tauri/src/lib.rs
decisions:
  - CUDA graph omitted: variable encoder input shape across audio chunks prevents safe capture; TF32 only
  - warm_up_parakeet placed before transcribe_with_parakeet in file for logical ordering
metrics:
  duration_seconds: 143
  completed_date: "2026-03-02T18:04:16Z"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 3
---

# Quick Task 23: Tier 1 Parakeet Latency Optimizations Summary

**One-liner:** TF32 matmul/conv enabled on Ampere+ GPUs via CUDA EP builder, plus background silent-audio warm-up inference after every Parakeet model load to pre-initialize CUDA context and cuDNN.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Enable TF32 in CUDA EP builder | 61415d5 | execution.rs |
| 2 | Add warm-up function and invoke after model load | 2c50f6b | transcribe_parakeet.rs, lib.rs |

## Changes Made

### Task 1 — execution.rs

Added `.with_tf32(true)` to the `CUDAExecutionProvider::default()` builder chain in the `ExecutionProvider::Cuda` match arm. Updated the eprintln log message to say "TF32 enabled". CUDA graph was explicitly not added because Parakeet's encoder receives variable-length audio — input shapes change every call, making CUDA graph capture invalid for this session.

### Task 2 — transcribe_parakeet.rs + lib.rs

Added `pub fn warm_up_parakeet(parakeet: &mut ParakeetTDT)` which runs 8000 zero-samples (0.5s of silence at 16kHz) through the model and discards the result, logging completion time at INFO level and errors at WARN level (non-fatal).

Inserted warm-up calls in both Parakeet load paths in lib.rs:
- **Startup path** (~line 1408): after `*guard = Some(Arc::new(Mutex::new(p)))`, clones the Arc and spawns a thread calling `warm_up_parakeet`.
- **Engine-switch path** (~line 314): same pattern after the engine-switch model load succeeds.

Both spawn `std::thread::spawn` so the UI is never blocked. If a real transcription arrives during warm-up it waits on the inner Mutex (sub-2s, acceptable).

## Verification

- `cargo check --features parakeet` passes with no errors
- `execution.rs` contains `.with_tf32(true)` at line 117
- `transcribe_parakeet.rs` exports `warm_up_parakeet` at line 64
- `lib.rs` calls `warm_up_parakeet` at lines 324 (engine-switch) and 1420 (startup)

## Deviations from Plan

### Auto-fixed Issues

None — plan executed exactly as written (CUDA graph omission was explicitly specified in the plan).

## Self-Check: PASSED

- `src-tauri/patches/parakeet-rs/src/execution.rs` — FOUND, contains `.with_tf32(true)`
- `src-tauri/src/transcribe_parakeet.rs` — FOUND, exports `warm_up_parakeet`
- `src-tauri/src/lib.rs` — FOUND, calls `warm_up_parakeet` in both load paths
- Commit `61415d5` — FOUND
- Commit `2c50f6b` — FOUND
