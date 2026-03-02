---
phase: quick-10
plan: "01"
subsystem: backend/inference
tags: [cuda, gpu, parakeet, onnx, performance]
dependency_graph:
  requires: [parakeet-rs 0.1.9 with cuda feature, CUDA Toolkit (already present for whisper-rs)]
  provides: [CUDA ExecutionProvider for Parakeet TDT inference]
  affects: [transcribe_parakeet.rs, Cargo.toml, lib.rs]
tech_stack:
  added: []
  patterns: [ort CUDA EP with automatic CPU fallback]
key_files:
  modified:
    - src-tauri/Cargo.toml
    - src-tauri/src/transcribe_parakeet.rs
    - src-tauri/src/lib.rs
decisions:
  - "Use parakeet_rs::{ExecutionConfig, ExecutionProvider} (top-level re-exports) not parakeet_rs::execution::* (private module)"
  - "Both load_parakeet call sites pass true for use_cuda — CUDA EP is always requested; ort falls back to CPU if CUDA unavailable"
metrics:
  duration: "~5 minutes"
  completed_date: "2026-03-01"
  tasks_completed: 1
  tasks_total: 1
---

# Quick Task 10: Enable CUDA GPU Acceleration for Parakeet TDT Summary

**One-liner:** CUDA ExecutionProvider wired into Parakeet TDT via parakeet-rs cuda feature, routing ONNX inference through GPU with automatic CPU fallback.

## Tasks Completed

| # | Task | Commit | Status |
|---|------|--------|--------|
| 1 | Enable cuda feature on parakeet-rs and wire CUDA ExecutionProvider | 1f2a9c5 | Done |

## What Was Built

Three coordinated changes to enable GPU inference for Parakeet TDT:

1. **src-tauri/Cargo.toml** — Added `features = ["cuda"]` to the `parakeet-rs` dependency. Updated the NOTE comment to reflect that CUDA is now enabled and ort downloads pre-compiled CUDA binaries automatically.

2. **src-tauri/src/transcribe_parakeet.rs** — Imported `ExecutionConfig` and `ExecutionProvider` from `parakeet_rs` (top-level re-exports, since `parakeet_rs::execution` is a private module). Renamed `_use_cuda` to `use_cuda`. When `use_cuda=true`, constructs `ExecutionConfig::new().with_execution_provider(ExecutionProvider::Cuda)` and passes it to `ParakeetTDT::from_pretrained`. Updated doc comment to reflect CUDA EP usage.

3. **src-tauri/src/lib.rs** — Changed both `load_parakeet(&dir_str, false)` call sites (line 258: startup load, line 1246: engine switch load) to `load_parakeet(&dir_str, true)`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed private module import path**
- **Found during:** Task 1 (cargo check)
- **Issue:** Plan specified `use parakeet_rs::execution::{ModelConfig as ExecutionConfig, ExecutionProvider}` but `execution` is a private module (`mod execution` without `pub`). Compiler error E0603.
- **Fix:** Used top-level re-exports instead: `use parakeet_rs::{ExecutionConfig, ExecutionProvider, ParakeetTDT}`. The lib.rs re-exports them via `pub use execution::{ExecutionProvider, ModelConfig as ExecutionConfig}`.
- **Files modified:** src-tauri/src/transcribe_parakeet.rs
- **Commit:** 1f2a9c5 (same commit, caught before final commit)

## Verification Results

- `cargo check --features parakeet` — passed, no errors (one pre-existing MSVC cl warning from esaxx-rs unrelated)
- `ExecutionProvider::Cuda` confirmed in transcribe_parakeet.rs line 27
- `load_parakeet(&dir_str, true)` confirmed in lib.rs lines 258 and 1246 (two occurrences)
- `features = ["cuda"]` confirmed in Cargo.toml line 71 on parakeet-rs dependency

## Self-Check: PASSED

- src-tauri/Cargo.toml — exists, contains `features = ["cuda"]` on parakeet-rs
- src-tauri/src/transcribe_parakeet.rs — exists, contains `ExecutionProvider::Cuda`
- src-tauri/src/lib.rs — exists, both call sites use `true`
- Commit 1f2a9c5 — confirmed in git log
