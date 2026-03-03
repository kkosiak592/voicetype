---
phase: quick-30
plan: "01"
subsystem: audio
tags: [audio, privacy, on-demand, windows]
dependency_graph:
  requires: []
  provides: [on-demand-audio-capture]
  affects: [audio.rs, lib.rs, pipeline.rs]
tech_stack:
  added: []
  patterns: [on-demand resource lifecycle, open-on-use drop-on-done]
key_files:
  created: []
  modified:
    - src-tauri/src/audio.rs
    - src-tauri/src/lib.rs
    - src-tauri/src/pipeline.rs
decisions:
  - "On-demand stream: open in hotkey handler, drop in run_pipeline — no persistent mic access"
  - "open_recording_stream() helper extracts and stores stream atomically before returning buffer Arc"
  - "set_microphone validates device exists but does not open a stream — preference saved only"
  - "start_recording command opens stream on-demand using saved preference for frontend test UI"
metrics:
  duration: "~15 minutes"
  completed: "2026-03-03"
  tasks_completed: 2
  files_modified: 3
---

# Quick Task 30: Switch Audio Capture from Persistent Stream to On-Demand Summary

**One-liner:** On-demand audio capture: stream opens on hotkey press and drops after pipeline buffer extraction, eliminating the Windows microphone privacy tray icon when idle.

## What Was Done

Switched the audio capture lifecycle from persistent (open at startup, always running) to on-demand (open when recording starts, drop when recording ends). This removes the Windows microphone privacy indicator (tray icon) that previously appeared whenever the app was running, even when not recording.

## Tasks Completed

| Task | Name | Commit | Files Modified |
|------|------|--------|----------------|
| 1 | Refactor audio.rs to on-demand API | 8882be1 | src-tauri/src/audio.rs |
| 2 | Update lib.rs and pipeline.rs for on-demand lifecycle | db6fb28 | src-tauri/src/lib.rs, src-tauri/src/pipeline.rs |

## Changes Made

### audio.rs
- Renamed `start_persistent_stream()` to `open_stream()`
- Renamed `start_persistent_stream_with_device()` to `open_stream_with_device()`
- Updated struct doc comment: "Persistent" -> "On-demand", noting that dropping releases the microphone
- Updated log line: "(persistent)" -> "(on-demand)"
- Added `resolve_device_by_name(name: &str)` public function that centralizes device lookup (empty/"System Default" -> default device, named -> searched by description)

### lib.rs
- Removed startup stream creation (lines ~1894-1928) — replaced with `app.manage(audio::AudioCaptureMutex(Mutex::new(None)))`
- Deleted `read_saved_mic()` function — logic now done at recording time via `read_settings()`
- Added `open_recording_stream(app)` helper: reads microphone preference from settings, resolves device, opens stream, clears buffer, sets recording=true, stores AudioCapture in managed state, returns buffer Arc
- Updated `handle_hotkey_event` HoldToTalk branch: replaced audio mutex access + clear_buffer + recording.store() with `open_recording_stream()` call
- Updated `handle_hotkey_event` Toggle branch: same replacement
- Updated `set_microphone`: removed stream creation, now validates device existence and saves preference only
- Updated `start_recording` command: opens stream on demand if AudioCaptureMutex is None (for frontend test UI)

### pipeline.rs
- Changed `let guard` to `let mut guard` in `run_pipeline`
- Added `*guard = None` after `get_buffer()` call — drops AudioCapture, releases cpal::Stream, removes Windows microphone privacy indicator

## Decisions Made

- **On-demand stream lifecycle**: Open in `open_recording_stream()` (hotkey handler entry), drop in `run_pipeline` after buffer extraction. This ensures the microphone is held only for the minimal duration required.
- **`open_recording_stream()` helper**: Extracts the stream-open logic into a single helper to avoid duplicating code across HoldToTalk and Toggle branches. Returns `Option<Arc<Mutex<Vec<f32>>>>` — None signals failure, caller resets pipeline to idle.
- **set_microphone saves preference only**: No stream is opened when the user switches microphone in settings. The preference is validated (device must exist) but no stream is opened until the next recording.
- **start_recording opens on-demand**: The frontend test command also uses on-demand opening so it doesn't break when AudioCaptureMutex is None at startup.
- **open_stream() kept in public API**: `open_stream()` (no device argument) is kept as a public function even though it's unused internally. All internal callers go through `resolve_device_by_name()` + `open_stream_with_device()`.

## Deviations from Plan

None — plan executed exactly as written.

## Verification

- `cargo build` succeeds with no errors (1 dead_code warning for `open_stream` which is expected — public API surface not yet called internally)
- App starts with no audio stream (`AudioCaptureMutex` contains `None`)
- `handle_hotkey_event` opens stream on recording start via `open_recording_stream()` helper
- `run_pipeline` sets `AudioCaptureMutex` to `None` after extracting buffer, dropping stream
- `set_microphone` only saves preference to `settings.json`, no stream manipulation
- `start_recording` command opens stream on demand if none exists

## Self-Check: PASSED

Files modified:
- FOUND: src-tauri/src/audio.rs
- FOUND: src-tauri/src/lib.rs
- FOUND: src-tauri/src/pipeline.rs

Commits exist:
- FOUND: 8882be1 (feat(quick-30): refactor audio.rs to on-demand API)
- FOUND: db6fb28 (feat(quick-30): open/drop audio stream on-demand in lib.rs and pipeline.rs)
