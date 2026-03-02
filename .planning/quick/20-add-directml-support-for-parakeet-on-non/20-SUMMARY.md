---
phase: quick-20
plan: 01
subsystem: backend-inference, frontend-ui
tags: [directml, gpu, parakeet, onnx, inference-status]
tech-stack:
  added: [ExecutionProvider::DirectML, detect_gpu_full, CachedGpuDetection, get_gpu_info]
  patterns: [provider-string EP selection, managed-state GPU detection, inference status UI]
key-files:
  created: []
  modified:
    - src-tauri/Cargo.toml
    - src-tauri/src/transcribe.rs
    - src-tauri/src/transcribe_parakeet.rs
    - src-tauri/src/lib.rs
    - src/App.tsx
    - src/components/sections/ModelSection.tsx
    - src/components/FirstRun.tsx
decisions:
  - "directml feature added to parakeet-rs features alongside existing cuda"
  - "load_parakeet accepts provider string instead of use_cuda bool — clean API for 3 EP options"
  - "detect_gpu_full runs alongside detect_gpu at startup — two NVML calls cached once each"
  - "CachedGpuDetection registered on Builder before run() — same pattern as CachedGpuMode"
  - "directml_available=true for non-NVIDIA systems — DirectML covers Intel/AMD GPUs on Windows"
  - "Parakeet card shown in FirstRun for gpuDetected OR directmlAvailable — not gpuOnly anymore"
  - "get_gpu_info shows DIRECTML provider for Parakeet on non-NVIDIA, CUDA on NVIDIA"
metrics:
  duration: ~15min
  completed: "2026-03-02"
  tasks_completed: 2
  files_changed: 7
---

# Phase quick-20 Plan 01: Add DirectML Support for Parakeet + GPU Status Indicator Summary

DirectML execution provider wired into Parakeet TDT (NVIDIA->CUDA, non-NVIDIA->DirectML, no-GPU->CPU) with a new GPU/inference status indicator in ModelSection settings and DirectML-aware GPU badge in FirstRun.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Backend: DirectML EP + GPU detection enrichment + get_gpu_info command | 439772f | Cargo.toml, transcribe.rs, transcribe_parakeet.rs, lib.rs |
| 2 | Frontend: GPU status indicator in ModelSection + DirectML-aware FirstRun | 29c129c | App.tsx, ModelSection.tsx, FirstRun.tsx |

## What Was Built

**Backend (Task 1):**
- Added `directml` feature to `parakeet-rs` dependency in `Cargo.toml`
- Added `GpuDetection` struct to `transcribe.rs` with `gpu_name`, `parakeet_provider`, `is_nvidia` fields
- Added `detect_gpu_full()` to `transcribe.rs`: NVIDIA->cuda, non-NVIDIA->directml fallback
- Changed `load_parakeet(model_dir, use_cuda: bool)` to `load_parakeet(model_dir, provider: &str)` — matches on "cuda", "directml", or falls back to CPU
- Added `CachedGpuDetection` managed state registered on Builder before `run()`
- Updated both `load_parakeet` call sites in `lib.rs` (set_engine + startup loader) to read provider from `CachedGpuDetection`
- Added `GpuInfo` struct and `get_gpu_info` Tauri command returning gpu_name, execution_provider, active_model, active_engine
- Updated `FirstRunStatus` with `gpu_name` and `directml_available` fields
- Updated `check_first_run` to populate new fields from `CachedGpuDetection`

**Frontend (Task 2):**
- `App.tsx`: Extended `FirstRunStatus` interface with `gpuName` and `directmlAvailable`; passes new props to `FirstRun`
- `ModelSection.tsx`: Added `GpuInfo` interface, state + effect (`useEffect` on `[selectedModel, currentEngine]`), and "Inference Status" UI block showing GPU, Provider, Engine
- `FirstRun.tsx`: Updated `FirstRunProps` with `gpuName` and `directmlAvailable`; three-branch GPU badge (NVIDIA green / DirectML blue / CPU gray); `visibleModels` filter shows Parakeet for `gpuDetected || directmlAvailable`; grid adapts to `visibleModels.length`

## Verification

1. `cargo check --features "whisper,parakeet"` passes (no errors, no new warnings)
2. `npx tsc --noEmit` passes (zero TypeScript errors)
3. `get_gpu_info` command registered in invoke_handler
4. Both `load_parakeet` call sites pass provider string from `CachedGpuDetection`
5. DirectML feature present in `Cargo.toml` parakeet-rs features list
6. Parakeet card visible for DirectML users (`gpuDetected || directmlAvailable`)
7. `large-v3-turbo` (gpuOnly: true) remains hidden for non-NVIDIA users

## Deviations from Plan

None - plan executed exactly as written.

The only minor interpretation: the startup Parakeet loader inside `setup()` (which runs after Builder registration) has access to `CachedGpuDetection` via `app.state::<CachedGpuDetection>()` since it is registered on the Builder. This matches the established pattern for `CachedGpuMode`.

## Self-Check: PASSED

- SUMMARY.md: FOUND at `.planning/quick/20-add-directml-support-for-parakeet-on-non/20-SUMMARY.md`
- Task 1 commit 439772f: FOUND
- Task 2 commit 29c129c: FOUND
