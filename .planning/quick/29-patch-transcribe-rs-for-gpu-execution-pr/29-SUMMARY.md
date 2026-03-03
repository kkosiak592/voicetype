---
phase: quick-29
plan: 01
subsystem: benchmark
tags: [transcribe-rs, ort, cuda, directml, moonshine, sensevoice, gpu, onnx]

requires:
  - phase: quick-28
    provides: "Moonshine and SenseVoice benchmark sections behind bench_extra feature"
provides:
  - "Local transcribe-rs patch with configurable execution providers (CUDA/DirectML/CPU)"
  - "GPU-accelerated Moonshine and SenseVoice benchmarks"
affects: [quick-28, benchmark, transcribe-rs]

tech-stack:
  added: [ort (direct dep for bench_extra)]
  patterns: [local crate patch with execution provider injection, provider vec passthrough]

key-files:
  created:
    - src-tauri/patches/transcribe-rs/ (full crate patch)
  modified:
    - src-tauri/patches/transcribe-rs/src/engines/moonshine/model.rs
    - src-tauri/patches/transcribe-rs/src/engines/moonshine/engine.rs
    - src-tauri/patches/transcribe-rs/src/engines/sense_voice/model.rs
    - src-tauri/patches/transcribe-rs/src/engines/sense_voice/engine.rs
    - src-tauri/patches/transcribe-rs/Cargo.toml
    - src-tauri/Cargo.toml
    - src-tauri/src/bin/benchmark.rs

key-decisions:
  - "ort added as direct optional dependency for bench_extra — Rust 2021 edition requires explicit dep to use ort types in benchmark binary"

patterns-established:
  - "ExecutionProvider passthrough: Option<Vec<ExecutionProviderDispatch>> on ModelParams structs, forwarded to init_session with CPU fallback default"

requirements-completed: [QUICK-29]

duration: 8min
completed: 2026-03-03
---

# Quick Task 29: Patch transcribe-rs for GPU Execution Summary

**Local transcribe-rs patch exposing configurable CUDA/DirectML execution providers for Moonshine and SenseVoice benchmark GPU acceleration**

## Performance

- **Duration:** 8 min
- **Started:** 2026-03-03T18:57:23Z
- **Completed:** 2026-03-03T19:05:34Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments
- Created local transcribe-rs v0.2.8 patch at src-tauri/patches/transcribe-rs/ with cuda/directml feature flags
- Added execution_providers field to MoonshineModelParams and SenseVoiceModelParams
- Updated benchmark.rs to pass CUDA+CPU fallback providers to all bench_extra models when GPU detected
- Section headers now display active provider (cuda/cpu) for clear benchmark output

## Task Commits

Each task was committed atomically:

1. **Task 1: Create local transcribe-rs patch with configurable execution providers** - `521e3f0` (feat)
2. **Task 2: Update benchmark.rs to pass GPU execution providers to Moonshine and SenseVoice** - `222db00` (feat)

## Files Created/Modified
- `src-tauri/patches/transcribe-rs/` - Full crate copy of transcribe-rs v0.2.8 with EP patches
- `src-tauri/patches/transcribe-rs/Cargo.toml` - Added cuda and directml feature flags
- `src-tauri/patches/transcribe-rs/src/engines/moonshine/model.rs` - MoonshineModel::new() and init_session() accept optional providers
- `src-tauri/patches/transcribe-rs/src/engines/moonshine/engine.rs` - MoonshineModelParams gains execution_providers field
- `src-tauri/patches/transcribe-rs/src/engines/sense_voice/model.rs` - SenseVoiceModel::new() and init_session() accept optional providers
- `src-tauri/patches/transcribe-rs/src/engines/sense_voice/engine.rs` - SenseVoiceModelParams gains execution_providers field
- `src-tauri/Cargo.toml` - Patch entry, cuda/directml features, ort direct dep for bench_extra
- `src-tauri/src/bin/benchmark.rs` - CUDA providers vec creation and passthrough to all bench_extra models

## Decisions Made
- Added `ort` as a direct optional dependency (gated by bench_extra feature) because Rust 2021 edition does not expose transitive dependencies. The benchmark binary needs to construct `CUDAExecutionProvider` and `CPUExecutionProvider` directly.
- Removed `#[derive(Debug, Clone)]` from patched ModelParams structs and implemented Default manually, since the struct now contains `Vec<ExecutionProviderDispatch>` which has custom Debug impl.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added ort as direct dependency for bench_extra feature**
- **Found during:** Task 2 (benchmark.rs compilation)
- **Issue:** `ort::execution_providers::ExecutionProviderDispatch` not resolvable from benchmark binary because ort is only a transitive dependency through transcribe-rs, and Rust 2021 edition requires explicit dependency declarations
- **Fix:** Added `ort = { version = "2.0.0-rc.10", optional = true }` to Cargo.toml dependencies and `"dep:ort"` to bench_extra feature
- **Files modified:** src-tauri/Cargo.toml
- **Verification:** `cargo check --features whisper,parakeet,bench_extra --release` succeeds
- **Committed in:** 222db00 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Essential fix for Rust 2021 module resolution. No scope creep.

## Issues Encountered
None beyond the auto-fixed deviation above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Benchmark binary now runs all models (Whisper, Parakeet, Moonshine, SenseVoice) with GPU acceleration when NVIDIA GPU is detected
- Run benchmark with: `cd src-tauri && cargo run --bin benchmark --features whisper,parakeet,bench_extra --release`

---
*Quick Task: 29-patch-transcribe-rs-for-gpu-execution-pr*
*Completed: 2026-03-03*
