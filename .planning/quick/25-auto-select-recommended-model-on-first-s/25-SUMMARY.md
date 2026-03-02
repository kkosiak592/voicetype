---
phase: quick-25
plan: 01
subsystem: gpu-detection, first-run-ui
tags: [gpu, dxgi, directml, first-run, ux]
dependency_graph:
  requires: []
  provides: [DXGI discrete GPU detection, corrected DirectML availability, Download Recommended button]
  affects: [FirstRun screen, check_first_run IPC command, GPU detection pipeline]
tech_stack:
  added: [windows crate 0.58 (Win32_Graphics_Dxgi)]
  patterns: [DXGI adapter enumeration, VRAM threshold detection]
key_files:
  created: []
  modified:
    - src-tauri/Cargo.toml
    - src-tauri/src/transcribe.rs
    - src-tauri/src/lib.rs
    - src/components/FirstRun.tsx
decisions:
  - "512 MB VRAM threshold chosen to reliably exclude integrated GPUs (Intel UHD, AMD APU share system RAM) while including discrete GPUs (AMD RX, Intel Arc typically 2+ GB)"
  - "DXGI factory creation failure falls back to false (recommends small-en) — safe degradation on unusual Windows configurations"
  - "Download Recommended button hidden during/after download to prevent duplicate UI; model cards remain for power users"
metrics:
  duration: ~8 minutes
  completed: 2026-03-02
  tasks_completed: 2
  files_modified: 4
---

# Phase quick-25 Plan 01: Auto-Select Recommended Model on First Start Summary

**One-liner:** DXGI-based discrete GPU detection with 512 MB VRAM threshold fixes integrated GPU false-positive for DirectML, plus a one-click "Download Recommended" button on the FirstRun screen.

## What Was Built

**Task 1 — DXGI discrete GPU detection and recommendation fix (b826c91)**

Added `has_discrete_gpu: bool` to `GpuDetection` struct. Added `has_discrete_gpu()` function that enumerates DXGI adapters via `IDXGIFactory1::EnumAdapters1`, skips software adapters (`DXGI_ADAPTER_FLAG_SOFTWARE`), and returns true if any non-software adapter has `DedicatedVideoMemory > 512 MB`. Each adapter is logged with name, VRAM MB, and software flag.

`detect_gpu_full()` now:
- NVIDIA path: `has_discrete_gpu: true` (NVIDIA = always discrete)
- Non-NVIDIA paths: calls `has_discrete_gpu()`, sets `parakeet_provider` to "directml" if discrete else "cpu", sets `gpu_name` to "DirectML (auto-detected)" or "Integrated GPU"

`check_first_run()` now:
- `directml_available = detection.0.has_discrete_gpu && !detection.0.is_nvidia` (was `!gpu_mode`)
- Recommends Parakeet for NVIDIA OR any discrete GPU; recommends small-en for integrated-only/no GPU

**Task 2 — Download Recommended button (1d9f729)**

Added a prominent indigo button (`bg-indigo-600`, `rounded-xl`, `px-8 py-3`) between the GPU badge and model cards. Shows "Download Recommended" with the model name and size on a smaller line. Calls `handleDownload(recommendedModel)` on click. Hides when `downloadState !== 'idle'`. Divider text "or choose a different model below" guides power users to the cards grid.

## Deviations from Plan

None — plan executed exactly as written.

## Verification

- `cargo check`: Passed (5.57s, no errors)
- `npx tsc --noEmit`: Passed (no output)

## Self-Check: PASSED

- src-tauri/src/transcribe.rs: FOUND
- src-tauri/src/lib.rs: FOUND
- src/components/FirstRun.tsx: FOUND
- src-tauri/Cargo.toml: FOUND
- Commit b826c91: FOUND
- Commit 1d9f729: FOUND
