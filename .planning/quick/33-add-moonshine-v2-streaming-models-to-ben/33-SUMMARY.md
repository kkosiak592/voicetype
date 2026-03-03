---
phase: quick-33
plan: 01
subsystem: benchmark / transcribe-rs streaming engine
tags: [moonshine, streaming, benchmark, onnx, cuda, execution-providers]
dependency_graph:
  requires: [quick-29 (bench_extra GPU provider pattern), quick-28 (MoonshineEngine bench sections)]
  provides: [GPU-injectable streaming Moonshine engine, three streaming model bench sections]
  affects: [benchmark binary, streaming_engine.rs, streaming_model.rs, BENCHMARK.md]
tech_stack:
  added: []
  patterns: [Option<Vec<ExecutionProviderDispatch>> fallback pattern (matching model.rs), no-VAD bench section for streaming models]
key_files:
  created: []
  modified:
    - src-tauri/patches/transcribe-rs/src/engines/moonshine/streaming_engine.rs
    - src-tauri/patches/transcribe-rs/src/engines/moonshine/streaming_model.rs
    - src-tauri/src/bin/benchmark.rs
    - test-fixtures/BENCHMARK.md
decisions:
  - StreamingModelParams.execution_providers follows the same Option<Vec<ExecutionProviderDispatch>> shape as MoonshineModelParams — consistent API across both engines
  - load_session() fallback pattern identical to model.rs init_session() — None => CPU, Some(p) => use provided providers
  - Streaming bench sections omit VAD chunking — streaming engine handles arbitrary-length audio via sliding-window encoder, unlike batch Moonshine
  - StreamingModelParams::default() used (no tiny()/base() constructors exist for streaming variant)
metrics:
  duration: ~8 minutes
  completed: 2026-03-03
  tasks_completed: 2
  files_modified: 4
---

# Quick Task 33: Add Moonshine v2 Streaming Models to Benchmark Summary

**One-liner:** GPU-injectable execution providers wired into the 5-session streaming Moonshine engine, with three new moonshine-streaming-tiny/small/medium benchmark sections that bypass VAD chunking.

## What Was Done

### Task 1: execution_providers field in StreamingModelParams + StreamingModel threading

`streaming_engine.rs`:
- Added `execution_providers: Option<Vec<ort::execution_providers::ExecutionProviderDispatch>>` to `StreamingModelParams`
- Default value is `None` (falls back to CPU)
- `load_model_with_params` passes `params.execution_providers` to `StreamingModel::new()`

`streaming_model.rs`:
- `StreamingModel::new()` gains `providers: Option<Vec<ExecutionProviderDispatch>>` parameter
- Each of the 5 `load_session()` calls receives `providers.as_deref()`
- `load_session()` updated with `providers: Option<&[ExecutionProviderDispatch]>` parameter
- Hardcoded `CPUExecutionProvider::default().build()` replaced with fallback pattern: `None => default_providers (CPU), Some(p) => p.to_vec()`
- `CPUExecutionProvider` import retained (used in the CPU fallback path)

### Task 2: Benchmark binary + BENCHMARK.md

`benchmark.rs`:
- `bench_extra` use block: added `MoonshineStreamingEngine, StreamingModelParams` to moonshine import
- Three path discovery blocks added (after `sensevoice_path`): `moonshine-streaming-tiny`, `moonshine-streaming-small`, `moonshine-streaming-medium`
- Three bench sections inserted (after moonshine-base, before sensevoice-small):
  - Each uses `MoonshineStreamingEngine::new()` and `StreamingModelParams::default()`
  - `params.execution_providers = bench_extra_providers.clone()` for GPU support
  - No `vad_chunk_audio` call — full audio passed as `vec![audio]` (single chunk)

`BENCHMARK.md`:
- Three rows added to Extended table: moonshine-streaming-tiny/small/medium with directory names and HuggingFace source
- Note added below the table explaining 5-session pipeline and no VAD chunking requirement

## Verification

- `cargo check --features bench_extra,whisper,parakeet` compiles clean (pre-existing warnings only)
- `execution_providers` field confirmed in `StreamingModelParams` (streaming_engine.rs line 17)
- All three streaming model paths and bench sections present in benchmark.rs
- `vad_chunk_audio` appears only in function definition + moonshine-tiny/base/sensevoice sections — not in any streaming section

## Deviations from Plan

None — plan executed exactly as written.

## Self-Check

- [x] streaming_engine.rs modified: execution_providers field added
- [x] streaming_model.rs modified: new() and load_session() accept providers
- [x] benchmark.rs modified: 3 path vars + 3 bench sections
- [x] BENCHMARK.md modified: 3 rows + note added
- [x] Task 1 commit: 6f0d7a7
- [x] Task 2 commit: 61171c7
- [x] cargo check clean

## Self-Check: PASSED
