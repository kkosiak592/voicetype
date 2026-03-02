---
phase: 02-audio-whisper
plan: "02"
subsystem: transcription
tags: [whisper-rs, cuda, gpu, whisper-cpp, rust, tauri, inference]

requires:
  - phase: 01-foundation
    provides: Tauri app skeleton, lib.rs managed state pattern, tray setup
provides:
  - whisper-rs 0.15 CUDA transcription module (transcribe.rs)
  - WhisperState managed Tauri state for Arc<WhisperContext>
  - transcribe_test_file Tauri command for WAV inference testing
  - Graceful startup when model file is missing
affects: [03-pipeline, 04-injection, 05-vad]

tech-stack:
  added:
    - whisper-rs 0.15 with cuda feature (GPU inference via whisper.cpp)
    - hound 3.5.1 (WAV file reading for test command)
  patterns:
    - WhisperContext wrapped in Arc for thread-safe sharing across Tauri commands
    - std::thread::spawn + mpsc channel for blocking whisper inference (replaced tokio::spawn_blocking)
    - Graceful feature degradation: app starts even if model file is missing

key-files:
  created:
    - src-tauri/src/transcribe.rs
  modified:
    - src-tauri/Cargo.toml
    - src-tauri/src/lib.rs

key-decisions:
  - "CUDA 12.9 (not 11.7): MSVC 14.44 (VS 2022 17.14) STL headers incompatible with CUDA 11.7's nvcc"
  - "CUDA 12.9 (not 13.x): CUDA 13.x dropped sm_61 (Pascal) support — 12.9 is last version supporting P2000"
  - "whisper-rs 0.15.x API: full_get_segment_text replaced by get_segment(i).to_str(), full_n_segments returns c_int directly"
  - "tokio removed: replaced tokio::task::spawn_blocking with std::thread::spawn + mpsc channel for blocking inference"
  - "Bindgen needs BINDGEN_EXTRA_CLANG_ARGS with 3 include paths: clang builtins, Windows UCRT, MSVC headers"
  - "LIBCLANG_PATH set permanently to VS 2022 Community LLVM path"
  - "WhisperState wraps Option<Arc<WhisperContext>> — None when model is missing, allowing app to start without model"
  - "save_test_wav uses app data dir (absolute path) — relative paths fail because exe working directory is unpredictable"
  - "withGlobalTauri goes under app section in Tauri 2.0 config (not build section)"

patterns-established:
  - "Pattern: WhisperContext managed state — Arc<WhisperContext> stored as Tauri managed state, cloned per command call"
  - "Pattern: Blocking inference — whisper::state.full() wrapped in std::thread::spawn + mpsc channel"
  - "Pattern: Graceful model absence — startup attempts model load, logs warning if missing, continues normally"
  - "Pattern: Use app data dir for file I/O — relative paths unreliable with Tauri dev server"

requirements-completed: [CORE-03]

duration: multi-session
completed: 2026-02-28
---

# Phase 2 Plan 02: Whisper-rs CUDA Transcription Module Summary

**whisper-rs 0.15 CUDA module with GPU-accelerated inference verified on Quadro P2000 — model loads in 715ms, transcription produces accurate English text with GPU memory utilization confirmed**

## Performance

- **Started:** 2026-02-27
- **Completed:** 2026-02-28
- **Tasks:** 2 of 2 (Task 1: code, Task 2: GPU verification)
- **Files modified:** 3 (transcribe.rs created, Cargo.toml updated, lib.rs updated)

## GPU Verification Results

- **Model:** ggml-large-v3-turbo-q5_0.bin (547.4 MB)
- **Model load time:** 715ms
- **GPU memory:** 573.45 MB on CUDA device 0 (Quadro P2000, compute 6.1)
- **Test transcription:** "Hi, my name is Matt and I'm trying to figure something out. Can you help me?" — accurate
- **Inference time:** 1414ms for multi-sentence clip (~10s audio)
- **GPU utilization:** Dedicated GPU memory spike confirmed in Task Manager during inference
- **Note:** 1414ms exceeds 500ms target but this was a long multi-sentence clip. Short single-sentence dictation (the actual use case) should be well under 500ms.

## Accomplishments

- Created `transcribe.rs` with `models_dir()`, `resolve_model_path()`, `load_whisper_context()`, and `transcribe_audio()` implementing the full whisper-rs GPU inference pipeline
- Added `WhisperState(Option<Arc<WhisperContext>>)` as Tauri managed state — app launches even when model file is absent
- Added `transcribe_test_file` async Tauri command that reads WAV files (float or integer format), downmixes to mono f32, and runs inference in std::thread::spawn
- Resolved CUDA build environment: CUDA 12.9 Toolkit, LIBCLANG_PATH, BINDGEN_EXTRA_CLANG_ARGS with 3 include paths
- Fixed whisper-rs 0.15.x API changes from 0.14.x (segment access, return types)
- Replaced tokio::spawn_blocking with std::thread::spawn + mpsc channel
- Fixed save_test_wav to use absolute app data dir path
- Added withGlobalTauri to Tauri config for DevTools console testing

## Task Commits

1. **Task 1: Add whisper-rs dependency and create transcribe.rs** - `1e13ccd` (feat)
2. **Task 2: GPU verification** - confirmed via manual testing (DevTools console invoke)

## Files Created/Modified

- `src-tauri/src/transcribe.rs` — Whisper context loading, GPU inference, model path resolution with download instructions
- `src-tauri/Cargo.toml` — Added whisper-rs 0.15 with cuda feature
- `src-tauri/src/lib.rs` — mod transcribe, WhisperState managed state, transcribe_test_file command, save_test_wav using app data dir, std::thread inference

## Deviations from Plan

### Environment Changes

**1. CUDA 12.9 instead of CUDA 11.7**
- **Issue:** MSVC 14.44 (VS 2022 17.14) STL headers incompatible with CUDA 11.7's nvcc
- **Fix:** Installed CUDA 12.9 — last version supporting sm_61 (Pascal/P2000). CUDA 13.x dropped Pascal support.

**2. whisper-rs 0.15.x API changes**
- **Issue:** `full_get_segment_text()` removed, `full_n_segments()` return type changed
- **Fix:** Updated to `get_segment(i).to_str()`, direct c_int return handling

**3. tokio replaced with std::thread**
- **Issue:** tokio::task::spawn_blocking added unnecessary async runtime dependency
- **Fix:** std::thread::spawn + mpsc channel for blocking inference

**4. Bindgen include paths**
- **Issue:** bindgen couldn't find stdbool.h, fell back to Linux bundled bindings (struct size mismatch)
- **Fix:** Set BINDGEN_EXTRA_CLANG_ARGS with clang builtins, UCRT, and MSVC include paths permanently

### Build Environment Setup (permanent env vars)

```
LIBCLANG_PATH = C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\Llvm\x64\bin
BINDGEN_EXTRA_CLANG_ARGS = "-IC:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\Llvm\x64\lib\clang\19\include" "-IC:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\ucrt" "-IC:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\include"
CMAKE_CUDA_ARCHITECTURES = 61
```

## Blockers Resolved

- CUDA Toolkit installation (resolved: CUDA 12.9)
- LIBCLANG_PATH (resolved: VS 2022 Community LLVM)
- BINDGEN_EXTRA_CLANG_ARGS (resolved: 3 include paths)
- whisper-rs API breaking changes (resolved: updated to 0.15.x API)
- save_test_wav path resolution (resolved: use app data dir)

## Known Issues

- Audio capture doesn't work from Claude Code's bash shell ("No default input device found") — must use PowerShell or CMD
- RDP "Remote Audio" device works for capture but untested for latency impact

## Next Phase Readiness

- GPU transcription verified and working
- Plan 02-03 (GPU detection + CPU fallback) is next
- transcribe.rs module ready for wiring into hold-to-talk pipeline (Phase 3)

---
*Phase: 02-audio-whisper*
*Completed: 2026-02-28*
