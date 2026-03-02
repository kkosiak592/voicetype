---
phase: 08-add-parakeet-tdt-model-and-optimize-transcription-latency
plan: 01
subsystem: backend
tags: [parakeet-rs, onnx, rust, transcription, download, huggingface]

# Dependency graph
requires:
  - phase: 07-distribution
    provides: download.rs streaming download infrastructure (DownloadEvent, reqwest, atomic write pattern)
provides:
  - Parakeet TDT inference wrapper (load_parakeet, transcribe_with_parakeet) in transcribe_parakeet.rs
  - parakeet-rs 0.1.9 optional dependency under "parakeet" feature in Cargo.toml
  - Multi-file Parakeet model download command (download_parakeet_model) in download.rs
  - parakeet_model_exists() and parakeet_model_dir() public helpers in download.rs
affects:
  - 08-02 (pipeline wiring — will import transcribe_parakeet.rs and register download command)
  - 08-03 (frontend download UI — will invoke download_parakeet_model command)

# Tech tracking
tech-stack:
  added:
    - "parakeet-rs 0.1.9 — Parakeet TDT ONNX inference via ort 2.0.0-rc.10"
  patterns:
    - "Optional Cargo feature gate: parakeet = [dep:parakeet-rs] kept out of defaults until Plan 02 wires it in"
    - "Cumulative progress across multi-file download: single DownloadEvent::Progress stream with running total"
    - "Atomic write (.tmp-then-rename) per file in multi-file download sequence"
    - "Directory-level cleanup on error: remove entire parakeet-tdt-v2/ dir so next launch re-shows download prompt"

key-files:
  created:
    - src-tauri/src/transcribe_parakeet.rs
  modified:
    - src-tauri/Cargo.toml
    - src-tauri/src/download.rs

key-decisions:
  - "parakeet-rs 0.1.9 used instead of 0.3.x: 0.3.x requires ort ^2.0.0-rc.11 which conflicts with voice_activity_detector pinning ort =2.0.0-rc.10; 0.1.9 uses ort ^2.0.0-rc.10"
  - "transcribe_with_parakeet takes &mut ParakeetTDT: parakeet-rs 0.1.x transcribe_samples requires &mut self, so callers use Mutex<ParakeetTDT>"
  - "No cuda feature on parakeet-rs dep: cuda requires CUDA toolkit at link time; CPU EP compiles without CUDA; feature can be added once deployment environment confirmed"
  - "parakeet_model_exists() checks encoder-model.int8.onnx only: encoder is largest and last file atomically renamed, so its presence reliably indicates a complete download"
  - "download_parakeet_model sends cumulative progress (not per-file progress): single progress bar on frontend avoids UX complexity of 5-file download sequencing"
  - "No SHA256 validation for Parakeet files: HuggingFace does not expose pre-computed SHA256 for LFS files; size validation via content-length; TODO to add checksums post-verified-download"

patterns-established:
  - "Multi-file download with cumulative DownloadEvent::Progress: sum all file sizes for total_bytes, track cumulative_downloaded across file loop"
  - "Error cleanup in multi-file download: remove entire model directory, not just current file, to prevent partial directory state"

requirements-completed: [PKT-01, PKT-02, PKT-04]

# Metrics
duration: 7min
completed: 2026-03-01
---

# Phase 08 Plan 01: Parakeet TDT Inference Wrapper and Download Infrastructure Summary

**parakeet-rs 0.1.9 inference wrapper and 5-file HuggingFace ONNX download command, both optional and not yet wired into the pipeline**

## Performance

- **Duration:** 7 min
- **Started:** 2026-03-01T17:20:10Z
- **Completed:** 2026-03-01T17:27:24Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Created `transcribe_parakeet.rs` with `load_parakeet()` (ExecutionConfig CUDA/CPU) and `transcribe_with_parakeet()` (`&mut self` call, owns audio Vec)
- Added `parakeet-rs 0.1.9` as optional Cargo dependency under the new `parakeet` feature (not in defaults)
- Extended `download.rs` with `download_parakeet_model` Tauri command: streams 5 int8 ONNX files from HuggingFace with cumulative progress events and atomic write per file
- Added `parakeet_model_exists()` and `parakeet_model_dir()` public helpers for pipeline and frontend use

## Task Commits

Each task was committed atomically:

1. **Task 1: Create transcribe_parakeet.rs and add parakeet-rs dependency** - `d47e8b6` (feat)
2. **Task 2: Extend download.rs with multi-file Parakeet model download** - `f585fa1` (feat)

**Plan metadata:** _(pending — created in final commit)_

## Files Created/Modified
- `src-tauri/src/transcribe_parakeet.rs` - Parakeet TDT inference wrapper (load_parakeet, transcribe_with_parakeet)
- `src-tauri/Cargo.toml` - Added parakeet feature and parakeet-rs 0.1.9 optional dependency
- `src-tauri/src/download.rs` - Added PARAKEET_FILES, parakeet_download_url, download_parakeet_model, parakeet_model_exists, parakeet_model_dir

## Decisions Made
- Used parakeet-rs 0.1.9 instead of 0.3.x to avoid ort version conflict with voice_activity_detector
- `transcribe_with_parakeet` takes `&mut ParakeetTDT` — parakeet-rs 0.1.x `transcribe_samples` requires `&mut self`; Plan 02 will use `Mutex<ParakeetTDT>` in managed state
- No `cuda` feature on parakeet-rs dep — CPU EP compiles without CUDA toolkit; cuda feature added in deployment when environment is confirmed
- Cumulative progress across all 5 files (single total_bytes denominator) for simpler frontend progress bar
- No SHA256 for Parakeet files — HuggingFace does not expose LFS file checksums; TODO added in code

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Used parakeet-rs 0.1.9 instead of 0.3.x due to ort version conflict**
- **Found during:** Task 1 (adding Cargo.toml dependency)
- **Issue:** parakeet-rs 0.3.x requires `ort = "^2.0.0-rc.11"` but `voice_activity_detector 0.2.1` pins `ort = "=2.0.0-rc.10"` (exact). Even optional dependencies participate in Cargo resolution, causing a hard conflict.
- **Fix:** Pinned parakeet-rs to 0.1.9 which uses `ort = "^2.0.0-rc.10"`. Adapted `transcribe_parakeet.rs` API: no `Transcriber` trait, `transcribe_samples` takes `&mut self` (documented with Mutex note).
- **Files modified:** src-tauri/Cargo.toml, src-tauri/src/transcribe_parakeet.rs
- **Verification:** `cargo check` passes with default features; `cargo check --features parakeet` also passes
- **Committed in:** d47e8b6 (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (blocking dependency conflict)
**Impact on plan:** Version downgrade from 0.3.x to 0.1.9 changes API signature (`&mut self` vs `&self` on transcribe_samples, no Transcriber trait). Plan 02 will use `Mutex<ParakeetTDT>` instead of `Arc<ParakeetTDT>` for managed state. All planned functionality is preserved.

## Issues Encountered
- parakeet-rs 0.3.x and voice_activity_detector 0.2.1 cannot coexist due to incompatible ort version pinning. Resolved by using parakeet-rs 0.1.9.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Plan 02 can import `transcribe_parakeet.rs` via `#[cfg(feature = "parakeet")] mod transcribe_parakeet;` in lib.rs
- Plan 02 must register `download_parakeet_model` in the Tauri command handler
- Plan 02 managed state for Parakeet should use `Arc<Mutex<ParakeetTDT>>` (not `Arc<ParakeetTDT>`) due to `&mut self` requirement
- Whisper functionality unchanged — all existing tests and pipeline continue to work

---
*Phase: 08-add-parakeet-tdt-model-and-optimize-transcription-latency*
*Completed: 2026-03-01*
