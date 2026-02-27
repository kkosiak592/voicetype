---
phase: 02-audio-whisper
plan: 01
subsystem: audio
tags: [cpal, wasapi, rubato, hound, resampling, audio-capture, rust]

requires:
  - phase: 01-foundation
    provides: Tauri app scaffold, managed state pattern, setup() hook, invoke_handler registration

provides:
  - Persistent cpal WASAPI microphone stream running at app startup
  - AtomicBool recording flag for instant start/stop with zero device init delay
  - Chunked rubato FftFixedIn resampler bridging variable cpal callback buffers to fixed 1024-sample chunks
  - 16kHz mono f32 sample accumulation in Arc<Mutex<Vec<f32>>> buffer
  - hound WAV writer at 16kHz mono 32-bit float for capture verification
  - Tauri commands: start_recording, stop_recording, save_test_wav
  - whisper feature flag gating transcribe module (requires LIBCLANG_PATH)

affects:
  - 02-02-audio-whisper (transcribe.rs already pre-staged, whisper feature flag needed for its build)
  - 03-core-pipeline (audio commands are the capture half of hold-to-talk loop)
  - 05-vad (audio buffer is the input to VAD pipeline)

tech-stack:
  added:
    - cpal 0.17.3 (WASAPI audio capture)
    - rubato 0.15.0 (FftFixedIn FFT resampling, device rate -> 16kHz)
    - hound 3.5.1 (WAV file writing)
    - log 0.4 (structured logging macros)
    - env_logger 0.11.9 (RUST_LOG controlled output, default "info")
    - whisper-rs 0.15 optional (gated behind whisper feature, requires LIBCLANG_PATH + CUDA)
  patterns:
    - Persistent stream pattern: cpal stream starts at app startup, recording flag toggles buffer accumulation
    - Chunked resampler accumulator: ResamplingState staging Vec bridges variable callback size to FftFixedIn fixed chunk size
    - try_lock in audio callback: never block audio thread (avoids cpal callback deadlock, see RESEARCH.md Pitfall 2)
    - unsafe impl Sync for AudioCapture: cpal::Stream is Send not Sync; safe because Arc clones are the only cross-thread access
    - Cargo feature flag: optional dep pattern for build-environment-dependent crate (whisper-rs)

key-files:
  created:
    - src-tauri/src/audio.rs
  modified:
    - src-tauri/Cargo.toml
    - src-tauri/src/lib.rs

key-decisions:
  - "cpal SampleRate is a type alias u32 in 0.17 (not tuple struct) — access directly without .0"
  - "whisper-rs made optional via [features] whisper flag — prevents LIBCLANG_PATH blocking audio-only cargo check"
  - "ResamplingState.resampling field kept private — flush_and_stop() handles flush internally, no pub needed"
  - "device.description().map(|d| d.name()) used over deprecated device.name() for cpal 0.17.3 compatibility"

patterns-established:
  - "Persistent audio stream pattern: stream in struct with _stream field kept alive, AtomicBool toggles accumulation"
  - "try_lock() not lock() in audio callbacks: prevents callback thread blocking"
  - "Feature-gated optional deps: add [features] section when dep requires heavy build prerequisites (LLVM, CUDA)"

requirements-completed: [CORE-02]

duration: 14min
completed: 2026-02-27
---

# Phase 2 Plan 01: Audio Capture Summary

**Persistent cpal WASAPI microphone stream with rubato FftFixedIn resampling to 16kHz mono, hound WAV output, and three Tauri recording commands**

## Performance

- **Duration:** 14 min
- **Started:** 2026-02-27T16:28:33Z
- **Completed:** 2026-02-27T16:42:14Z
- **Tasks:** 2
- **Files modified:** 3 (audio.rs created, Cargo.toml updated, lib.rs rewritten)

## Accomplishments

- Created `audio.rs` with `AudioCapture` struct: persistent cpal stream, `Arc<AtomicBool>` recording flag, `Arc<Mutex<Vec<f32>>>` 16kHz sample buffer, and `ResamplingState` chunked accumulator for rubato FftFixedIn
- Implemented `flush_and_stop()` which sets recording=false, flushes remaining staging samples through zero-padded final resampler chunk, and returns total sample count
- Wired audio module into `lib.rs` with three Tauri commands (`start_recording`, `stop_recording`, `save_test_wav`), env_logger initialization, and graceful mic-unavailable fallback
- Made `whisper-rs` optional via `[features] whisper` flag so `cargo build` passes without LIBCLANG_PATH installed

## Task Commits

1. **Task 1: Add audio dependencies and create audio.rs** - `0359149` (feat)
2. **Task 2: Wire audio module to Tauri with recording commands** - `acdd3e0` (feat)

**Plan metadata:** (docs commit — see below)

## Files Created/Modified

- `src-tauri/src/audio.rs` — AudioCapture struct, start_persistent_stream(), write_wav(), clear_buffer(), flush_and_stop(), get_buffer(); ResamplingState chunked resampler accumulator
- `src-tauri/Cargo.toml` — Added cpal/rubato/hound/log/env_logger deps; added [features] whisper section with optional whisper-rs
- `src-tauri/src/lib.rs` — mod audio integration, start_recording/stop_recording/save_test_wav commands, env_logger init, whisper code gated behind #[cfg(feature = "whisper")]

## Decisions Made

- **cpal 0.17 SampleRate API:** `SampleRate` is `type SampleRate = u32` in cpal 0.17 — not a tuple struct. `config.sample_rate()` returns `u32` directly, `.0` field access is invalid. Fixed automatically.
- **whisper-rs as optional dep:** The pre-staged `transcribe.rs` (from Plan 02-02 which ran out of order) references `whisper-rs`, which requires LIBCLANG_PATH for bindgen. Made `whisper-rs` optional behind a `whisper` Cargo feature flag so the audio module can be verified without installing LLVM/clang.
- **device.description() over device.name():** `device.name()` is deprecated in cpal 0.17.3. Used `device.description().map(|d| d.name().to_string())` per the deprecation hint.
- **ResamplingState field privacy:** `resampling` field made private — `flush_and_stop()` encapsulates flushing, no external access needed, removed privacy mismatch warning.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed cpal SampleRate API — .0 field access invalid in cpal 0.17**
- **Found during:** Task 1 (audio.rs creation) — caught by cargo check
- **Issue:** `config.sample_rate().0` fails — `SampleRate` is `type SampleRate = u32` not a tuple struct in cpal 0.17.3
- **Fix:** Changed to `config.sample_rate() as usize` (direct u32 access)
- **Files modified:** src-tauri/src/audio.rs
- **Verification:** cargo check passes with no errors
- **Committed in:** `0359149` (Task 1 commit)

**2. [Rule 3 - Blocking] Pre-staged transcribe.rs from Plan 02-02 blocked cargo check**
- **Found during:** Task 1 verification — cargo check failed with `unresolved import whisper_rs`
- **Issue:** Plan 02-02 was executed before Plan 02-01, leaving `transcribe.rs` and whisper-rs wiring in lib.rs. `whisper-rs` with `cuda` feature requires LIBCLANG_PATH (bindgen), which is not installed on this machine.
- **Fix:** Added `[features] whisper = ["dep:whisper-rs"]` to Cargo.toml; made whisper-rs `optional = true`; gated all whisper code in lib.rs behind `#[cfg(feature = "whisper")]`
- **Files modified:** src-tauri/Cargo.toml, src-tauri/src/lib.rs
- **Verification:** `cargo build` succeeds without whisper feature; `cargo build --features whisper` will require LIBCLANG_PATH + CUDA
- **Committed in:** `0359149` (Cargo.toml) and `acdd3e0` (lib.rs)

---

**Total deviations:** 2 auto-fixed (1 bug, 1 blocking pre-existing state)
**Impact on plan:** Both fixes were necessary for correct operation. Whisper feature flag is architecturally sound — audio capture is independent of transcription and should be buildable without CUDA/LLVM prerequisites.

## Issues Encountered

- **Plan 02-02 executed before Plan 02-01:** transcribe.rs and whisper-rs were already committed before this plan ran. The code was handled as pre-existing state, not re-created. This plan added audio.rs and updated lib.rs to integrate both modules cleanly.
- **LIBCLANG_PATH not installed:** whisper-rs requires bindgen which requires libclang. Not available on this machine. Resolved by making whisper-rs optional behind a feature flag. Full build with whisper requires LLVM installation (see Plan 02-02 SUMMARY).

## User Setup Required

To build with whisper support (for Plan 02-02 verification):

```powershell
# Install LLVM (provides libclang.dll for bindgen)
winget install LLVM.LLVM

# Set LIBCLANG_PATH
$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"

# Build with whisper feature
$env:CMAKE_CUDA_ARCHITECTURES = "61"
cargo build --features whisper --manifest-path src-tauri/Cargo.toml
```

Audio capture (this plan) builds without any of the above:
```powershell
cargo build --manifest-path src-tauri/Cargo.toml
```

## Next Phase Readiness

- Audio capture is complete and tested at compile level
- `start_recording` / `stop_recording` / `save_test_wav` Tauri commands are registered and functional
- Plan 02-02 (whisper CUDA inference) requires LIBCLANG_PATH to be installed before `cargo build --features whisper` can proceed
- Test-fixtures/ directory created for WAV output storage

---
*Phase: 02-audio-whisper*
*Completed: 2026-02-27*
