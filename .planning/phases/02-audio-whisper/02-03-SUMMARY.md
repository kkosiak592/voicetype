---
phase: 02-audio-whisper
plan: "03"
subsystem: transcription
tags: [nvml-wrapper, gpu-detection, cpu-fallback, whisper-rs, cuda, rust, tauri, inference]

requires:
  - phase: 02-audio-whisper
    plan: "02"
    provides: transcribe.rs with load_whisper_context and resolve_model_path, WhisperState managed state

provides:
  - ModelMode enum (Gpu/Cpu) for runtime inference mode selection
  - detect_gpu() using nvml-wrapper NVML API for runtime NVIDIA GPU detection
  - resolve_model_path(mode) selecting large-v3-turbo-q5_0 (GPU) or small.en-q5_1 (CPU)
  - load_whisper_context(path, mode) setting use_gpu(true/false) per detected mode
  - force_cpu_transcribe Tauri command for CPU fallback verification on GPU machines
affects: [03-pipeline, 04-injection, 05-vad]

tech-stack:
  added:
    - nvml-wrapper 0.10 (runtime NVIDIA GPU detection via NVML library)
  patterns:
    - detect_gpu() called once at startup, returns ModelMode passed through to model resolution and context loading
    - nvml-wrapper optional in whisper feature — no extra build overhead for non-whisper builds
    - force_cpu_transcribe creates temporary WhisperContext with use_gpu(false) for testing CPU path on GPU machines

key-files:
  created: []
  modified:
    - src-tauri/src/transcribe.rs
    - src-tauri/src/lib.rs
    - src-tauri/Cargo.toml

key-decisions:
  - "nvml-wrapper 0.10 tied to whisper Cargo feature — no NVML dependency for non-whisper builds"
  - "detect_gpu() uses Nvml::init() + device_by_index(0) — Cpu fallback on any NVML error (no GPU, no drivers, init failure)"
  - "force_cpu_transcribe creates fresh WhisperContext per call with use_gpu(false) — not stored in managed state, test-only command"
  - "read_wav_to_f32() extracted as shared helper — avoids duplication between transcribe_test_file and force_cpu_transcribe"

patterns-established:
  - "Pattern: Runtime GPU detection — detect_gpu() called once in setup(), ModelMode flows through resolve_model_path and load_whisper_context"
  - "Pattern: Test command for CPU fallback — force_cpu_transcribe loads CPU model independently of main WhisperState"

requirements-completed: [CORE-04]

duration: 14min
completed: 2026-02-28
---

# Phase 2 Plan 03: GPU Detection + CPU Fallback Summary

**nvml-wrapper runtime GPU detection with ModelMode-driven model selection — large-v3-turbo-q5_0 for NVIDIA GPU, small.en-q5_1 for CPU — and force_cpu_transcribe command for verifying CPU fallback on GPU machines**

## Performance

- **Duration:** 14 min
- **Started:** 2026-02-28T13:24:33Z
- **Completed:** 2026-02-28T13:38:33Z
- **Tasks:** 1 of 1
- **Files modified:** 3 (transcribe.rs, lib.rs, Cargo.toml + Cargo.lock)

## Accomplishments

- Added nvml-wrapper 0.10 to the whisper Cargo feature — GPU detection uses the NVIDIA Management Library which is available whenever NVIDIA drivers are installed, no CUDA dependency
- Added `ModelMode` enum (Gpu/Cpu) and `detect_gpu()` function that tries `Nvml::init()` then `device_by_index(0)` — falls back to Cpu on any error with informative log messages
- Refactored `resolve_model_path(mode)` to select the right model filename and download URL based on mode
- Refactored `load_whisper_context(path, mode)` to set `use_gpu(true/false)` per mode and log "Loading whisper model: ... (GPU=true/false)"
- Added `force_cpu_transcribe` Tauri command that loads the CPU model with `use_gpu(false)` for verifying CORE-04 on the dev machine (which has a Quadro P2000)
- Extracted `read_wav_to_f32()` as a shared private helper used by both `transcribe_test_file` and `force_cpu_transcribe`
- Updated `setup()` in lib.rs to call `detect_gpu()` and pass the mode through the chain

## Task Commits

1. **Task 1: GPU detection + model selection + force_cpu_transcribe** - `e29150d` (feat)

## Files Created/Modified

- `src-tauri/src/transcribe.rs` — ModelMode enum, detect_gpu(), refactored resolve_model_path(mode) and load_whisper_context(path, mode)
- `src-tauri/src/lib.rs` — detect_gpu() call in setup(), force_cpu_transcribe command, read_wav_to_f32() helper
- `src-tauri/Cargo.toml` — nvml-wrapper = "0.10" added to whisper feature

## Decisions Made

- nvml-wrapper 0.10 tied to whisper feature: no extra dependency for audio-only builds
- detect_gpu() falls back to Cpu on any NVML error — this is correct behavior (no GPU = use CPU)
- force_cpu_transcribe creates a fresh WhisperContext per call rather than storing it as managed state, since it's a test-only command and CPU model load (small.en, ~76MB) is fast enough

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Extracted read_wav_to_f32() as shared helper**
- **Found during:** Task 1 (adding force_cpu_transcribe)
- **Issue:** WAV decoding logic duplicated between transcribe_test_file and force_cpu_transcribe
- **Fix:** Extracted to private `read_wav_to_f32(path) -> Result<(Vec<f32>, u32), String>` helper
- **Files modified:** src-tauri/src/lib.rs
- **Verification:** Build passes, both commands call the shared helper
- **Committed in:** e29150d

---

**Total deviations:** 1 auto-fixed (1 missing critical — code quality/correctness)
**Impact on plan:** No scope creep. The extraction was necessary to avoid code duplication when adding force_cpu_transcribe.

## Issues Encountered

- Build env vars (LIBCLANG_PATH, BINDGEN_EXTRA_CLANG_ARGS) are set as Windows user env vars but don't propagate to bash shell spawned by Claude Code — build must be run via PowerShell script (`build-whisper.ps1`) to pick up the registry values. This is a pre-existing known issue from Plan 02-02.

## User Setup Required

To verify CPU fallback, the small.en model must be downloaded:

```powershell
Invoke-WebRequest -Uri 'https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en-q5_1.bin' -OutFile "$env:APPDATA\VoiceType\models\ggml-small.en-q5_1.bin"
```

Then invoke `force_cpu_transcribe` from DevTools with a WAV file path:
```js
window.__TAURI__.core.invoke('force_cpu_transcribe', { path: 'C:\\path\\to\\test.wav' })
```

Expected: `[XXXXms CPU] <transcription text>` — CPU inference on small model takes 2-10s for a 5s clip.

## Next Phase Readiness

- GPU detection and model selection logic complete — transcribe.rs is ready to be wired into the hold-to-talk pipeline in Phase 3
- force_cpu_transcribe command available for CORE-04 verification once small.en model is downloaded
- CPU fallback path is code-complete and confirmed to compile; runtime verification (actual transcription) requires manual model download step

## Self-Check: PASSED

- FOUND: src-tauri/src/transcribe.rs (ModelMode: 11 refs, detect_gpu: 2 refs)
- FOUND: src-tauri/src/lib.rs (force_cpu_transcribe: 5 refs)
- FOUND: src-tauri/Cargo.toml (nvml-wrapper: 2 refs)
- FOUND: .planning/phases/02-audio-whisper/02-03-SUMMARY.md
- FOUND: commit e29150d (feat(02-03): GPU detection + CPU fallback)

---
*Phase: 02-audio-whisper*
*Completed: 2026-02-28*
