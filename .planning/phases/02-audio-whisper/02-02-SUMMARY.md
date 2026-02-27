---
phase: 02-audio-whisper
plan: "02"
subsystem: transcription
tags: [whisper-rs, cuda, gpu, whisper-cpp, rust, tauri, inference]

# Dependency graph
requires:
  - phase: 01-foundation
    provides: Tauri app skeleton, lib.rs managed state pattern, tray setup
provides:
  - whisper-rs 0.15 CUDA transcription module (transcribe.rs)
  - WhisperState managed Tauri state for Arc<WhisperContext>
  - transcribe_test_file Tauri command for WAV inference testing
  - Graceful startup when model file is missing
affects: [03-pipeline, 04-injection, 05-vad]

# Tech tracking
tech-stack:
  added:
    - whisper-rs 0.15 with cuda feature (GPU inference via whisper.cpp)
    - env_logger 0.11 (RUST_LOG-controlled verbose logging)
  patterns:
    - WhisperContext wrapped in Arc for thread-safe sharing across Tauri commands
    - spawn_blocking for synchronous whisper inference on Tauri async runtime
    - Graceful feature degradation: app starts even if model file is missing

key-files:
  created:
    - src-tauri/src/transcribe.rs
  modified:
    - src-tauri/Cargo.toml
    - src-tauri/src/lib.rs

key-decisions:
  - "whisper-rs cuda feature requires CUDA_PATH env var at build time — CUDA Toolkit must be installed before cargo build succeeds"
  - "WhisperState wraps Option<Arc<WhisperContext>> — None when model is missing, allowing app to start without model"
  - "transcribe_audio creates fresh WhisperState per call (not reused) — per RESEARCH.md Pitfall 6 for thread safety"
  - "spawn_blocking wraps all whisper inference — whisper.cpp is synchronous and would block the tokio runtime"
  - "env_logger initialized in run() — RUST_LOG=info controls verbosity for all log! macros"

patterns-established:
  - "Pattern: WhisperContext managed state — Arc<WhisperContext> stored as Tauri managed state, cloned per command call"
  - "Pattern: Blocking inference — all whisper::state.full() calls wrapped in tokio::task::spawn_blocking"
  - "Pattern: Graceful model absence — startup attempts model load, logs warning if missing, continues normally"

requirements-completed: [CORE-03]

# Metrics
duration: 15min
completed: 2026-02-27
---

# Phase 2 Plan 2: Whisper-rs CUDA Transcription Module Summary

**whisper-rs 0.15 CUDA module with GPU-accelerated inference, WAV file decoding, and graceful model-absent startup — blocked on CUDA Toolkit installation for build verification**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-02-27T16:28:34Z
- **Completed:** 2026-02-27T16:43:00Z
- **Tasks:** 1 of 2 (Task 2 is human-verify checkpoint — awaiting build + GPU verification)
- **Files modified:** 4

## Accomplishments

- Created `transcribe.rs` with `models_dir()`, `resolve_model_path()`, `load_whisper_context()`, and `transcribe_audio()` implementing the full whisper-rs GPU inference pipeline
- Added `WhisperState(Option<Arc<WhisperContext>>)` as Tauri managed state — app launches even when model file is absent
- Added `transcribe_test_file` async Tauri command that reads WAV files (float or integer format), downmixes to mono f32, and runs inference in `spawn_blocking`
- Added `env_logger` initialization in `run()` — all logging now controlled via `RUST_LOG` env var

## Task Commits

1. **Task 1: Add whisper-rs dependency and create transcribe.rs** - `1e13ccd` (feat)

## Files Created/Modified

- `src-tauri/src/transcribe.rs` — Whisper context loading, GPU inference, model path resolution with download instructions
- `src-tauri/Cargo.toml` — Added whisper-rs 0.15 with cuda feature
- `src-tauri/src/lib.rs` — Added mod transcribe, WhisperState managed state, transcribe_test_file command, env_logger init
- `src-tauri/Cargo.lock` — Updated with whisper-rs, whisper-rs-sys, bindgen, cmake, clang-sys dependencies

## Decisions Made

- Used `Option<Arc<WhisperContext>>` as managed state rather than failing startup when model is missing. App logs a warning with download instructions and continues running — consistent with how most voice apps handle missing models.
- `transcribe_audio` creates a fresh `WhisperState` per call (`ctx.create_state()`) rather than reusing one. This is the recommended pattern per RESEARCH.md Pitfall 6 — avoids concurrent state corruption on rapid hotkey presses.
- `env_logger::init()` called at the top of `run()` so all `log!` macros from this point forward respect `RUST_LOG`. Previous `println!` calls in hotkey handler replaced with `log::info!`.

## Deviations from Plan

None — plan executed as specified. Code structure matches plan exactly.

## Issues Encountered

**Build gate: CUDA Toolkit not installed**

- **During:** Task 1 verification (`cargo build`)
- **Error:** `whisper-rs-sys` build.rs panics at `env::var("CUDA_PATH").unwrap()` — `CUDA_PATH` not set, CUDA Toolkit 11.7 not installed
- **Also missing:** `LIBCLANG_PATH` (required by bindgen for whisper-rs-sys header generation)
- **Status:** Code is written and committed. Build cannot succeed until environment is set up.
- **Resolution:** User must install CUDA Toolkit and set environment variables (see User Setup Required section)

## User Setup Required

Before `cargo build` will succeed, the following must be set up:

**1. Install CUDA Toolkit 11.7**
- Download from: https://developer.nvidia.com/cuda-11-7-0-download-archive
- Install to default path (sets `CUDA_PATH` automatically)
- Verify: `nvcc --version` should show 11.7

**2. Install LLVM/clang (for bindgen)**
- Option A — Visual Studio: Install "C++ Clang Compiler for Windows" workload via VS Installer
  - Typical path: `C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\Llvm\x64\bin`
- Option B — Standalone LLVM: https://releases.llvm.org/download.html
- Set: `LIBCLANG_PATH=<path-to-directory-containing-clang.exe>`

**3. Set CMAKE_CUDA_ARCHITECTURES=61 before building**
```powershell
$env:CMAKE_CUDA_ARCHITECTURES = "61"
$env:LIBCLANG_PATH = "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\Llvm\x64\bin"
```

**4. Set CMAKE_CUDA_ARCHITECTURES permanently (recommended to avoid silent CPU fallback)**
- Add to system environment: `CMAKE_CUDA_ARCHITECTURES=61`
- This targets Pascal architecture (Quadro P2000)
- Without this, CUDA may compile but silently fall back to CPU

**5. Download the whisper model**
```powershell
New-Item -ItemType Directory -Force "$env:APPDATA\VoiceType\models"
Invoke-WebRequest -Uri "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin" `
  -OutFile "$env:APPDATA\VoiceType\models\ggml-large-v3-turbo-q5_0.bin"
```

**6. Build and test**
```powershell
# First build will take 10-30 minutes (compiles whisper.cpp with CUDA)
cargo build --manifest-path src-tauri/Cargo.toml

# Run the app with verbose logging
$env:RUST_LOG = "info"
cargo tauri dev
```

**7. Verify GPU (Task 2 checkpoint)**
- Check console for: "Whisper model loaded" and "CUDA whisper context initialized successfully"
- Open Task Manager → Performance → GPU
- Invoke `transcribe_test_file` with a WAV file path
- Confirm GPU compute spike during inference and duration < 500ms

## Next Phase Readiness

- `transcribe.rs` module is complete and ready for wiring into the audio pipeline (Phase 2 Plan 3)
- Blocked on CUDA Toolkit + LIBCLANG_PATH setup before build can be verified
- Once environment is set up and GPU verified, Task 2 checkpoint can be approved

---
*Phase: 02-audio-whisper*
*Completed: 2026-02-27 (partial — awaiting build environment setup)*
