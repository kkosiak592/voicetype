---
phase: quick-28
plan: 01
subsystem: benchmark
tags: [benchmark, moonshine, sense-voice, transcribe-rs, onnx, feature-flag]
dependency_graph:
  requires: [quick-27]
  provides: [moonshine-benchmark, sensevoice-benchmark]
  affects: [src-tauri/src/bin/benchmark.rs, src-tauri/Cargo.toml]
tech_stack:
  added: [transcribe-rs 0.2.8]
  patterns: [optional-dependency, cfg-feature-flag, engine-trait-pattern]
key_files:
  modified:
    - src-tauri/Cargo.toml
    - src-tauri/src/bin/benchmark.rs
decisions:
  - "Use MoonshineModelParams::tiny() and MoonshineModelParams::base() constructors — cleaner than MoonshineModelParams::variant(ModelVariant::Tiny)"
  - "MoonshineEngine::new() and SenseVoiceEngine::new() return Self (not Result) — no error handling on construction"
  - "Removed ModelVariant alias import — unused after switching to tiny()/base() constructors"
metrics:
  duration: "~10 minutes"
  completed: "2026-03-03"
  tasks_completed: 2
  files_modified: 2
---

# Phase quick-28 Plan 01: Add Moonshine and SenseVoice to Benchmark Summary

**One-liner:** Moonshine tiny/base and SenseVoice ONNX benchmarks via transcribe-rs 0.2.8, gated behind opt-in `bench_extra` Cargo feature flag.

## What Was Built

Added two files' worth of changes to expand the standalone benchmark binary with three new model sections:

**`src-tauri/Cargo.toml`**
- Added `bench_extra` feature flag: `bench_extra = ["dep:transcribe-rs"]`
- Added `transcribe-rs = { version = "0.2.8", features = ["moonshine", "sense_voice"], optional = true }`
- Default feature list unchanged; `required-features` on `[[bin]]` unchanged

**`src-tauri/src/bin/benchmark.rs`**
- Added `#[cfg(feature = "bench_extra")]` import block for `MoonshineEngine`, `MoonshineModelParams`, `SenseVoiceEngine`, `SenseVoiceModelParams`, and the `TranscriptionEngine` trait
- Added model discovery for `moonshine-tiny-ONNX`, `moonshine-base-ONNX`, `sensevoice-small` directories under `%APPDATA%/VoiceType/models/`
- Added `(bench_extra feature disabled — skipping moonshine/sensevoice models)` message when feature is off
- Added three benchmark sections (moonshine-tiny, moonshine-base, sensevoice-small) each following the same pattern: load model once, iterate 5 times per clip, collect latency + WER, push to shared `results` vec
- Existing whisper and parakeet sections untouched

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add transcribe-rs dependency and bench_extra feature flag | 3f8a429 | src-tauri/Cargo.toml |
| 2 | Add Moonshine and SenseVoice benchmark sections | bc32349 | src-tauri/src/bin/benchmark.rs |

## Verification Results

1. `cargo check --features whisper,parakeet` — PASS (no regression)
2. `cargo check --features whisper,parakeet,bench_extra` — PASS (clean, no warnings)
3. `grep -c 'bench_extra' Cargo.toml` — 2 (feature def + dep line)
4. `grep -c 'moonshine' benchmark.rs` — 18 (imports + discovery + 2 model sections)
5. `grep -cE 'sense_voice|sensevoice|SenseVoice' benchmark.rs` — 14

## Usage

```bash
# Existing benchmark (unchanged)
cargo run --bin benchmark --features whisper,parakeet --release

# With Moonshine + SenseVoice models
cargo run --bin benchmark --features whisper,parakeet,bench_extra --release
```

Models must be placed in `%APPDATA%/VoiceType/models/`:
- `moonshine-tiny-ONNX/` — Moonshine Tiny ONNX directory
- `moonshine-base-ONNX/` — Moonshine Base ONNX directory
- `sensevoice-small/` — SenseVoice Small ONNX directory

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Removed unused ModelVariant import alias**
- **Found during:** Task 2 verification (cargo check warning)
- **Issue:** Plan suggested importing `ModelVariant as MoonshineVariant` but it was unused since `MoonshineModelParams::tiny()` and `MoonshineModelParams::base()` were used instead of `MoonshineModelParams::variant(ModelVariant::Tiny)`
- **Fix:** Removed `ModelVariant as MoonshineVariant` from the import; used `MoonshineModelParams::tiny()` and `MoonshineModelParams::base()` constructors per actual API
- **Files modified:** src-tauri/src/bin/benchmark.rs

The plan checker note was also accurate: `MoonshineEngine::new()` and `SenseVoiceEngine::new()` both return `Self` directly (confirmed from transcribe-rs 0.2.8 source), so no `Result` handling was added on construction — only on `load_model_with_params()`.

## Self-Check: PASSED

- `src-tauri/Cargo.toml` — FOUND, contains bench_extra feature and transcribe-rs dep
- `src-tauri/src/bin/benchmark.rs` — FOUND, contains moonshine and sensevoice sections
- Commit 3f8a429 — FOUND (Task 1)
- Commit bc32349 — FOUND (Task 2)
