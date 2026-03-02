---
phase: quick-13
plan: 01
subsystem: transcription-engine
tags: [parakeet, fp32, model-variant, download, onnx]
dependency_graph:
  requires: [quick-10-cuda, 08-01-parakeet-download, 08-02-engine-dispatch]
  provides: [fp32-parakeet-variant, variant-aware-engine-dispatch]
  affects: [download.rs, lib.rs, ModelSection.tsx, ModelSelector.tsx, FirstRun.tsx]
tech_stack:
  added: []
  patterns:
    - ONNX external data format (encoder-model.onnx header + encoder-model.onnx.data weights)
    - variant-aware model directory resolution via resolve_parakeet_dir()
    - per-variant download state in React (independent fp32 vs int8 state machines)
key_files:
  created: []
  modified:
    - src-tauri/src/download.rs
    - src-tauri/src/lib.rs
    - src/components/sections/ModelSection.tsx
    - src/components/ModelSelector.tsx
    - src/components/FirstRun.tsx
decisions:
  - always-reload-parakeet-on-switch: set_engine always reloads Parakeet model (no is_none guard) — required for int8<->fp32 variant switching to take effect without app restart
  - fp32-dir-name: models/parakeet-tdt-v2-fp32 (separate from int8 dir models/parakeet-tdt-v2)
  - fp32-size-estimates: encoder header 41.8MB, data 2.44GB, decoder 35.8MB — estimates for progress bar denominator only, content-length fallback applies
  - int8-renamed: existing "Parakeet TDT" renamed to "Parakeet TDT (int8)" for disambiguation
  - isFastest-int8-only: Fastest badge kept on int8 only — quantization makes it faster than fp32
metrics:
  duration: ~15 minutes
  completed_date: "2026-03-01"
  tasks_completed: 3
  files_modified: 5
---

# Quick Task 13: Add fp32 Parakeet Model Variant as Selectable Option

**One-liner:** fp32 Parakeet variant with separate download dir, variant-aware engine dispatch, and independent per-variant download state in all three frontend surfaces.

## What Was Built

Two Parakeet TDT variants are now available as separate selectable entries:

- **Parakeet TDT (int8)** — existing 661 MB quantized model in `models/parakeet-tdt-v2/`
- **Parakeet TDT (fp32)** — new 2.56 GB full precision model in `models/parakeet-tdt-v2-fp32/`

Both appear in Settings model list and FirstRun flow. Both can be downloaded independently. Switching between them reloads the Parakeet model from the correct directory.

## Backend Changes (src-tauri/src/download.rs)

- `PARAKEET_FP32_FILES` const: 6 files (encoder-model.onnx header ~42MB, encoder-model.onnx.data ~2.44GB, decoder_joint-model.onnx ~35.8MB, nemo128.onnx, vocab.txt, config.json)
- `parakeet_fp32_model_dir()` — returns `models/parakeet-tdt-v2-fp32`
- `parakeet_fp32_model_exists()` — checks encoder-model.onnx in fp32 dir
- `download_parakeet_fp32_model` Tauri command — mirrors int8 download with cumulative progress across 6 files

## Backend Changes (src-tauri/src/lib.rs)

- `read_saved_parakeet_model()` — reads `parakeet_model` from settings.json via AppHandle
- `read_saved_parakeet_model_startup()` — same but reads via `&tauri::App` for startup use
- `resolve_parakeet_dir()` — maps model ID to directory (fp32 -> fp32 dir, default -> int8 dir)
- `set_engine` signature: now accepts `parakeet_model: Option<String>`; unconditional Parakeet reload (removed is_none guard); persists variant to settings.json when Some provided
- `list_models`: int8 renamed to "Parakeet TDT (int8)"; fp32 entry added
- `check_first_run`: also considers `parakeet_fp32_model_exists()` in needs_setup logic
- Startup loading: uses `read_saved_parakeet_model_startup` + `resolve_parakeet_dir` for variant-aware load
- `download_parakeet_fp32_model` registered in `invoke_handler`

## Frontend Changes

**ModelSection.tsx:**
- fp32 download state: `fp32Downloading`, `fp32Percent`, `fp32Error`
- `handleFp32Download()` — invokes `download_parakeet_fp32_model`, auto-selects on finish
- `handleModelSelect` — for any Parakeet variant, always calls `set_engine` with `parakeetModel` arg regardless of `currentEngine` (enables int8<->fp32 switching)
- Passes fp32 props to ModelSelector

**ModelSelector.tsx:**
- New props: `onFp32Download`, `fp32Downloading`, `fp32Percent`, `fp32Error`
- Per-variant state resolution: `isParakeetInt8`, `isParakeetFp32`, `thisDownloading/Percent/Error/OnDownload`
- `disabled` also checks `fp32Downloading`
- Progress/error rendering uses resolved per-variant variables

**FirstRun.tsx:**
- fp32 entry added to MODELS array (2.56 GB, Full precision (GPU), gpuOnly: true)
- int8 renamed to "Parakeet TDT (int8)"
- `handleDownload` dispatches `download_parakeet_fp32_model` for fp32 variant
- `handleComplete` passes `parakeetModel: downloadingId` to `set_engine` for both Parakeet variants
- Grid updated to `sm:grid-cols-2 lg:grid-cols-4` for 4 GPU model cards when GPU detected
- `isFastest` badge remains int8-only

## Verification

- `cargo check --features whisper,parakeet`: PASSED (5.62s)
- `npx tsc --noEmit`: PASSED (no errors)
- `cargo build --features whisper,parakeet`: PASSED (40.29s)

## Deviations from Plan

None — plan executed exactly as written.

## Commits

- `202ea24`: feat(quick-13): add fp32 Parakeet variant — backend download and variant-aware engine dispatch
- `c4e3cf0`: feat(quick-13): add fp32 Parakeet variant — frontend model cards and download handlers

## Self-Check: PASSED

Files exist:
- src-tauri/src/download.rs: FOUND (PARAKEET_FP32_FILES, parakeet_fp32_model_dir, parakeet_fp32_model_exists, download_parakeet_fp32_model)
- src-tauri/src/lib.rs: FOUND (parakeet-tdt-v2-fp32 in list_models, set_engine with Option<String>, resolve_parakeet_dir)
- src/components/sections/ModelSection.tsx: FOUND (fp32Downloading, handleFp32Download, parakeetModel in set_engine call)
- src/components/ModelSelector.tsx: FOUND (onFp32Download, fp32Downloading, isParakeetFp32)
- src/components/FirstRun.tsx: FOUND (parakeet-tdt-v2-fp32 in MODELS, download_parakeet_fp32_model dispatch)

Commits exist:
- 202ea24: FOUND
- c4e3cf0: FOUND
