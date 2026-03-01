---
phase: 07-distribution
plan: 01
subsystem: infra
tags: [rust, tauri, reqwest, sha2, download, autostart, whisper, gpu-detection]

# Dependency graph
requires:
  - phase: 06-vocabulary-settings
    provides: WhisperStateMutex, transcribe::detect_gpu/models_dir/ModelMode, model switching pattern
provides:
  - download.rs module with streaming HTTP download, SHA256 validation, Channel<DownloadEvent> progress events
  - check_first_run Tauri command surfacing needs_setup/gpu_detected/recommended_model
  - enable_autostart Tauri command for Windows startup registration
  - medium model removed — only large-v3-turbo and small-en remain
  - model descriptions updated with file sizes
affects: [07-02-frontend-first-run, 07-03-installer]

# Tech tracking
tech-stack:
  added: [reqwest 0.12 with stream feature, sha2 0.10, futures-util 0.3]
  patterns:
    - Channel<DownloadEvent> for streaming download progress from Rust to frontend
    - models_dir() duplicated in download.rs to avoid whisper feature-gate coupling
    - check_first_run gated behind whisper feature (calls detect_gpu); download_model and enable_autostart ungated

key-files:
  created:
    - src-tauri/src/download.rs
  modified:
    - src-tauri/Cargo.toml
    - src-tauri/src/lib.rs
    - src-tauri/src/transcribe.rs

key-decisions:
  - "download.rs duplicates models_dir() (3 lines) rather than importing from transcribe — avoids whisper feature-gate coupling on a pure HTTP/IO module"
  - "download_model and enable_autostart are not whisper-gated; check_first_run is whisper-gated (calls detect_gpu/models_dir from transcribe)"
  - "SHA256 checksums hardcoded in model_info() alongside expected file sizes — content_length() fallback avoids progress bar stalling when server omits header"
  - "Corrupt/failed downloads delete .tmp file atomically so next launch correctly detects needs_setup=true"
  - "resolve_model_path() PowerShell error message replaced with simple user-friendly message — app now handles downloads"

patterns-established:
  - "Channel<T> pattern: DownloadEvent enum tagged serde for easy frontend discrimination"
  - "Temp-file-then-rename atomic write pattern for model download"

requirements-completed: [DIST-01, DIST-02]

# Metrics
duration: 25min
completed: 2026-03-01
---

# Phase 07 Plan 01: Distribution Backend Summary

**Rust backend for model download with reqwest streaming, SHA256 validation, Channel<DownloadEvent> progress, GPU-detection-based first-run status, and Windows autostart registration**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-03-01T14:12:00Z
- **Completed:** 2026-03-01T14:40:04Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- Created `download.rs` with streaming HTTP download, chunk-level SHA256 validation, and Tauri Channel progress events — corrupt/failed downloads clean up .tmp files
- Added `check_first_run` Tauri command returning needs_setup, gpu_detected, recommended_model for the frontend first-run flow
- Added `enable_autostart` Tauri command wiring tauri-plugin-autostart enable() for Windows startup registration
- Removed medium model from list_models/model_id_to_path; updated model descriptions with file sizes; unknown IDs return Err gracefully
- Replaced PowerShell download instructions in transcribe.rs with a user-friendly "use app Model settings" message

## Task Commits

Each task was committed atomically:

1. **Task 1: Create download.rs with streaming download + SHA256 validation** - `672e0e1` (feat)
2. **Task 2: Add check_first_run/enable_autostart, remove medium model, register commands** - `ff5355f` (feat)

## Files Created/Modified

- `src-tauri/src/download.rs` - DownloadEvent enum, model_info(), download_url(), download_model Tauri command with reqwest streaming and SHA256 validation
- `src-tauri/Cargo.toml` - Added reqwest[stream], sha2, futures-util dependencies
- `src-tauri/src/lib.rs` - Added mod download, check_first_run, enable_autostart; removed medium model; registered all three new commands in invoke_handler
- `src-tauri/src/transcribe.rs` - Simplified resolve_model_path error message; cleaned up unused download_url variable

## Decisions Made

- Duplicated `models_dir()` in download.rs rather than importing from transcribe — keeps the download module feature-gate-independent (no whisper-rs needed for HTTP download)
- `download_model` and `enable_autostart` not whisper-gated; `check_first_run` is whisper-gated because it calls `detect_gpu()` which requires nvml-wrapper
- SHA256 checksums and expected sizes hardcoded in `model_info()` — `content_length()` fallback used when server omits the header to keep progress denominator accurate
- Temp file pattern (.tmp extension) with atomic rename on success; .tmp deleted on checksum mismatch or stream error so next launch re-shows setup flow

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Removed unused `download_url` variable in transcribe.rs**
- **Found during:** Task 2 (transcribe.rs error message update)
- **Issue:** After removing the PowerShell error message that used `download_url`, the tuple destructuring left an unused binding that would generate a compiler warning
- **Fix:** Replaced tuple `(filename, download_url)` with scalar `filename` match
- **Files modified:** src-tauri/src/transcribe.rs
- **Verification:** `cargo check --features whisper` passes with no warnings
- **Committed in:** ff5355f (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - bug/warning fix)
**Impact on plan:** Trivial cleanup required by the planned error message change. No scope creep.

## Issues Encountered

None — `cargo check --features whisper` passed on both tasks on first attempt. The `futures-util` dependency was listed as potentially transitive in the plan notes but adding it explicitly was straightforward.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Backend commands ready: `check_first_run`, `download_model`, `enable_autostart`, `list_models` (2 models only)
- Frontend (07-02) can invoke these commands to build the first-run setup UI and model download progress display
- Installer (07-03) can rely on autostart being configurable via `enable_autostart` command

---
*Phase: 07-distribution*
*Completed: 2026-03-01*
