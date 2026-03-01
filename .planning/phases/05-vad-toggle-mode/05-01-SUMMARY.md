---
phase: 05-vad-toggle-mode
plan: 01
subsystem: vad
tags: [vad, silero, onnx, pipeline, hold-to-talk, speech-detection]
dependency_graph:
  requires: []
  provides: [vad.rs, VadWorker, VadWorkerHandle, vad_gate_check, voice_activity_detector-dependency]
  affects: [pipeline.rs, lib.rs, Cargo.toml]
tech_stack:
  added: [voice_activity_detector@0.2.1, ort@2.0.0-rc.10 (transitive)]
  patterns: [streaming-vad-worker, post-hoc-vad-gate, inline-crate-paths-to-avoid-circular-imports]
key_files:
  created: [src-tauri/src/vad.rs]
  modified: [src-tauri/Cargo.toml, src-tauri/src/pipeline.rs, src-tauri/src/lib.rs]
decisions:
  - "No use crate::pipeline; import in vad.rs — reference pipeline via inline crate::pipeline:: paths only to avoid circular module coupling"
  - "SPEECH_PROBABILITY_THRESHOLD=0.5 (Silero default), SILENCE_FRAMES_THRESHOLD=47 (~1.5s), MIN_SPEECH_FRAMES=9 (~300ms), MAX_RECORDING_FRAMES=1875 (60s cap)"
  - "Fast-path 512-sample check before full VAD gate to avoid calling VoiceActivityDetector on trivially short buffers"
  - "Fresh VoiceActivityDetector per call/session — avoids stale LSTM state from previous recordings (Pitfall 4)"
  - "Blocking lock (not try_lock) in VAD worker — VAD is not a real-time thread; only cpal callback uses try_lock"
metrics:
  duration: "884 seconds (14.7 minutes, includes ONNX Runtime first download)"
  completed: "2026-03-01"
  tasks_completed: 2
  files_changed: 4
---

# Phase 05 Plan 01: Silero VAD Integration Summary

**One-liner:** Silero VAD V5 via voice_activity_detector@0.2.1 replaces crude 1600-sample gate with neural speech detection (VadWorker + vad_gate_check) in vad.rs, wired into run_pipeline().

## What Was Built

### Task 1: voice_activity_detector dependency + vad.rs module

Added `voice_activity_detector = "0.2.1"` to `src-tauri/Cargo.toml`. Created `src-tauri/src/vad.rs` with:

**Constants** (per plan spec):
- `SPEECH_PROBABILITY_THRESHOLD: f32 = 0.5` — Silero default
- `SILENCE_FRAMES_THRESHOLD: u32 = 47` — ~1.5s of silence at 32ms/chunk
- `MIN_SPEECH_FRAMES: u32 = 9` — ~300ms minimum speech gate
- `MAX_RECORDING_FRAMES: u32 = 1875` — 60s safety cap
- `CHUNK_SIZE: usize = 512` — Silero V5 fixed chunk size at 16kHz

**`vad_gate_check(samples: &[f32]) -> bool`** — synchronous post-hoc VAD gate for hold-to-talk mode. Creates fresh VoiceActivityDetector, iterates 512-sample chunks, counts speech frames, returns true if `speech_frames >= MIN_SPEECH_FRAMES`. Logs speech/total frame counts. Fails open (returns true) if VAD initialization fails.

**`VadWorkerHandle`** struct with `cancel_tx: Option<tokio::sync::oneshot::Sender<()>>` and `cancel()` method. Ready for Plan 02 toggle mode managed state (`Arc<Mutex<Option<VadWorkerHandle>>>`).

**`spawn_vad_worker(app, buffer) -> VadWorkerHandle`** — streaming VAD worker for toggle mode. Spawns async task that reads 512-sample chunks by cursor position (blocking lock, not try_lock), runs VAD, tracks silence/speech frames, and calls `trigger_auto_stop()` when silence threshold or safety cap is reached.

**`trigger_auto_stop()`** — handles VAD auto-stop. If `speech_frames >= MIN_SPEECH_FRAMES`: CAS RECORDING→PROCESSING, stop level stream, emit pill-state processing, update tray, spawn `crate::pipeline::run_pipeline()`. If insufficient speech: CAS RECORDING→IDLE, stop level stream, emit pill-result error, reset tray/pill. CAS failure (another handler won) exits silently.

**Circular import prevention:** No `use crate::pipeline;` at module top level. All pipeline references use inline `crate::pipeline::` paths: `crate::pipeline::PipelineState`, `crate::pipeline::RECORDING`, `crate::pipeline::IDLE`, `crate::pipeline::PROCESSING`, `crate::pipeline::run_pipeline`.

Added `mod vad;` to `lib.rs` after `mod pipeline;`.

### Task 2: VAD gate in run_pipeline()

Modified `src-tauri/src/pipeline.rs`:

1. Added `use crate::vad;` import
2. Updated `run_pipeline()` doc comment: step 2 now reads "VAD speech gate (Silero V5 neural model, ~300ms minimum speech)"
3. Replaced the `samples.len() < 1600` gate with a two-stage approach:
   - **Fast-path:** `if samples.len() < 512` — discard immediately (not enough data for one VAD chunk)
   - **Full VAD gate:** `if !vad::vad_gate_check(&samples)` — runs Silero V5 post-hoc, rejects if insufficient speech

The old 1600-sample (~100ms duration) gate only checked recording length. Any silence or noise lasting >100ms would reach whisper and cause hallucination. The new VAD gate uses the neural model to verify actual speech content regardless of duration.

## Commits

| Task | Commit | Description |
|------|--------|-------------|
| 1 | 720b0f0 | feat(05-01): add voice_activity_detector dependency and create vad.rs module |
| 2 | f59658a | feat(05-01): replace 1600-sample gate with Silero VAD gate in run_pipeline() |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Called non-existent `reset_to_idle_from_handle` in initial vad.rs**
- **Found during:** Task 1 first cargo check
- **Issue:** Initial implementation of `trigger_auto_stop()` called `crate::pipeline::PipelineState::reset_to_idle_from_handle(app)` — this function does not exist (the private `reset_to_idle` in pipeline.rs is not a method on PipelineState)
- **Fix:** Rewrote the discard path to inline the reset logic: `set(IDLE)` via CAS, stop level stream via managed state, emit pill events, update tray — matching what `reset_to_idle()` does internally
- **Files modified:** src-tauri/src/vad.rs
- **Commit:** 720b0f0 (fixed before commit)

**2. [Rule 1 - Bug] Used non-existent `app.try_state()` API**
- **Found during:** Task 1 first cargo check (same check as above)
- **Issue:** Used `app.try_state::<T>()` which is not a Tauri 2 API — only `app.state::<T>()` exists for managed state
- **Fix:** Changed to `app.state::<crate::LevelStreamActive>()` — always available since it's registered unconditionally in setup()
- **Files modified:** src-tauri/src/vad.rs
- **Commit:** 720b0f0 (fixed before commit)

## Success Criteria Verification

- [x] `voice_activity_detector = "0.2.1"` in Cargo.toml
- [x] `src-tauri/src/vad.rs` exists with `vad_gate_check()`, `VadWorkerHandle`, `spawn_vad_worker()` exported
- [x] `pipeline.rs` uses `vad::vad_gate_check()` — `samples.len() < 1600` is removed
- [x] `vad.rs` has NO `use crate::pipeline;` top-level import — pipeline referenced only via `crate::pipeline::` inline paths
- [x] `cargo check --features whisper` passes
- [x] `mod vad;` declared in `lib.rs`

## Infrastructure Ready for Plan 02

- `VadWorkerHandle` and `spawn_vad_worker()` are complete and compiled — Plan 02 wires them into the toggle mode hotkey handler
- `trigger_auto_stop()` handles the full auto-stop lifecycle (CAS, level stream stop, pill/tray update, pipeline spawn or discard)
- Remaining dead_code warnings for toggle mode infrastructure will clear when Plan 02 adds managed state and wires the hotkey handler

## Self-Check: PASSED

- src-tauri/src/vad.rs: FOUND
- src-tauri/Cargo.toml: FOUND
- 05-01-SUMMARY.md: FOUND
- Commit 720b0f0: FOUND
- Commit f59658a: FOUND
