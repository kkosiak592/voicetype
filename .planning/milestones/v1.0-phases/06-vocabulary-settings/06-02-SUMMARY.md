---
phase: 06-vocabulary-settings
plan: 02
subsystem: api
tags: [rust, tauri, cpal, whisper, audio, microphone, settings]

# Dependency graph
requires:
  - phase: 06-vocabulary-settings plan 01
    provides: corrections engine and WhisperState managed state patterns established
  - phase: 02-whisper-integration
    provides: WhisperContext, load_whisper_context, detect_gpu, resolve_model_path
  - phase: 03-pipeline-integration
    provides: pipeline.rs AudioCapture state access pattern this plan replaces
provides:
  - audio.rs: AudioCaptureMutex wrapper and start_persistent_stream_with_device() for runtime device switching
  - lib.rs: WhisperStateMutex replacing WhisperState for runtime model switching
  - lib.rs: list_input_devices, set_microphone, list_models, set_model Tauri commands
  - Startup mic/model restoration from settings.json with silent fallback to defaults
affects:
  - 06-03: settings UI plan that calls list_input_devices, set_microphone, list_models, set_model

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Mutex swap pattern: lock guard, replace inner value (*guard = new_value), drop — outer Mutex for replacement, inner for callback access"
    - "Lock-clone-drop before spawn_blocking: clone Arc inside lock scope, drop guard, then move Arc into spawn_blocking closure"
    - "Device lookup: host.input_devices() iterator with DeviceTrait::description().name() comparison — DeviceTrait must be in scope"
    - "Model GPU mode inference: large-v3-turbo -> Gpu mode, all others -> Cpu mode"

key-files:
  created: []
  modified:
    - src-tauri/src/audio.rs
    - src-tauri/src/lib.rs
    - src-tauri/src/pipeline.rs

key-decisions:
  - "AudioCaptureMutex outer Mutex guards entire AudioCapture for replacement; inner Mutex inside AudioCapture guards buffer for audio callback — two separate, non-nesting locks"
  - "build_stream_from_device() extracted as private function shared by start_persistent_stream() and start_persistent_stream_with_device() to avoid code duplication"
  - "hotkey handlers: lock AudioCaptureMutex, clone buffer Arc, drop guard before tray/pill state updates to minimize lock hold time"
  - "GPU mode determined by model_id for set_model: large-v3-turbo -> Gpu, medium/small-en -> Cpu"
  - "DeviceTrait must be imported locally in each function using description() — top-level import unused, local imports preferred"
  - "Startup model loading: try saved model first, fall back to GPU auto-detection if file missing or no preference saved"

patterns-established:
  - "State swap: lock, replace, drop — same pattern for AudioCaptureMutex and WhisperStateMutex"
  - "Lock lifetime: always drop guards before any blocking or async operations"

requirements-completed: [SET-03, SET-04]

# Metrics
duration: 5min
completed: 2026-02-28
---

# Phase 6 Plan 02: Runtime Device and Model Switching Summary

**AudioCaptureMutex and WhisperStateMutex wrappers with four new Tauri commands (list_input_devices, set_microphone, list_models, set_model) enabling runtime mic and whisper model switching without app restart**

## Performance

- **Duration:** 5 min
- **Started:** 2026-03-01T02:57:13Z
- **Completed:** 2026-03-01T03:02:23Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Refactored AudioCapture into AudioCaptureMutex — all 15+ state access sites updated across hotkey handlers (2 sets), 3 Tauri commands, and pipeline.rs
- Replaced WhisperState with WhisperStateMutex — pipeline.rs and transcribe_test_file updated to lock/clone/drop pattern
- Added `start_persistent_stream_with_device()` in audio.rs sharing logic with `start_persistent_stream()` via extracted `build_stream_from_device()`
- Added `list_input_devices` and `set_microphone` commands — mic switch replaces inner AudioCapture, persists to settings.json
- Added `list_models` and `set_model` commands — model switch uses spawn_blocking, persists model_id to settings.json
- Startup restores saved mic and model from settings.json with silent fallback to system default / GPU auto-detection
- cargo check passes with zero errors

## Task Commits

Both tasks were implemented together (both touched lib.rs atomically):

1. **Task 1: Refactor AudioCapture to Mutex wrapper and add device selection** - `8031dee` (feat)
2. **Task 2: Add model selection with runtime reload** - `8031dee` (feat) — committed with Task 1

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `src-tauri/src/audio.rs` - Added AudioCaptureMutex wrapper, build_stream_from_device() private fn, start_persistent_stream_with_device() public fn
- `src-tauri/src/lib.rs` - WhisperStateMutex, 4 new Tauri commands, read_saved_mic/read_saved_model_id helpers, model_id_to_path(), ModelInfo struct, updated all AudioCapture/WhisperState access sites, updated setup() managed state registration
- `src-tauri/src/pipeline.rs` - Updated AudioCapture access to AudioCaptureMutex, WhisperState access to WhisperStateMutex with lock/clone/drop pattern

## Decisions Made
- `AudioCaptureMutex` outer Mutex is purely for replacement (atomic swap of entire AudioCapture); inner Mutex inside AudioCapture guards buffer for audio callback — two locks serve different purposes and do not nest
- `build_stream_from_device()` extracted as private fn shared by both `start_persistent_stream()` and `start_persistent_stream_with_device()` to avoid duplicating the 60-line stream setup
- Hotkey handlers: lock AudioCaptureMutex, clone buffer Arc, drop guard before tray/pill/pill_stream calls — keeps lock scope minimal, no blocking under lock
- GPU mode for `set_model`: large-v3-turbo → Gpu (CUDA needed), medium/small-en → Cpu
- `DeviceTrait` imported locally in each function using `description()` — compiler needs it in scope per call site
- Startup model loading tries saved model first (file existence check), falls back to auto-detect if file missing

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
- `DeviceTrait` in scope issue: `description()` calls in closures required explicit `use cpal::traits::DeviceTrait;` and type annotation `|desc: cpal::DeviceDescription|` because Rust's type inference can't resolve the closure parameter without it. Fixed inline per Rule 3 (blocking compilation).

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- AudioCaptureMutex and WhisperStateMutex are in managed state, all commands registered
- `list_input_devices`, `set_microphone`, `list_models`, `set_model` ready for UI consumption in Plan 03
- Settings JSON keys: `microphone_device` (string) and `whisper_model_id` (string)
- Startup reads both keys and restores selections with graceful fallback

---
*Phase: 06-vocabulary-settings*
*Completed: 2026-02-28*
