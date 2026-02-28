---
phase: 03-core-pipeline
plan: 02
subsystem: pipeline
tags: [pipeline, atomicu8, state-machine, hold-to-talk, hotkey, rust, tauri]

# Dependency graph
requires:
  - phase: 03-core-pipeline
    plan: 01
    provides: inject_text() and set_tray_state() from inject.rs and tray.rs
  - phase: 02-audio-whisper
    provides: AudioCapture (flush_and_stop, get_buffer) and WhisperContext (transcribe_audio)
  - phase: 01-foundation
    provides: global shortcut plugin, tray infrastructure

provides:
  - PipelineState AtomicU8 state machine (IDLE/RECORDING/PROCESSING) with compare_exchange transitions
  - run_pipeline() async orchestration — full hold-to-talk pipeline from audio to injection
  - Hold-to-talk hotkey handler (Pressed -> RECORDING, Released -> PROCESSING -> run_pipeline)

affects: [03-core-pipeline-03]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - PipelineState uses AtomicU8 compare_exchange (SeqCst) to prevent concurrent recordings
    - run_pipeline called via tauri::async_runtime::spawn from Released handler closure
    - Whisper inference and inject_text both wrapped in tokio::task::spawn_blocking (both are sync)
    - Every early-return path in run_pipeline calls reset_to_idle() — no stuck states by design
    - use tauri::Manager must be imported in pipeline.rs to call app.state() on AppHandle

key-files:
  created:
    - src-tauri/src/pipeline.rs
  modified:
    - src-tauri/src/lib.rs

key-decisions:
  - "use tauri::Manager required in pipeline.rs — app.state() on AppHandle is gated behind Manager trait (discovered via E0599 on first cargo check)"
  - "Emitter import removed from lib.rs — app.emit('hotkey-triggered') fully replaced by backend pipeline; no frontend emit needed"
  - "Tray tooltip set on successful injection for development debugging — shows last transcription text"
  - "reset_to_idle also sets tray tooltip to 'VoiceType — idle' for clear state feedback during development"

# Metrics
duration: 2min
completed: 2026-02-28
---

# Phase 03 Plan 02: Hold-to-Talk Pipeline Orchestration Summary

**PipelineState AtomicU8 state machine + run_pipeline async orchestration wiring audio, whisper, and injection into the full hold-to-talk pipeline**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-28T14:19:01Z
- **Completed:** 2026-02-28T14:21:21Z
- **Tasks:** 2 of 3 (paused at Task 3: human verification checkpoint)
- **Files modified:** 2 (1 created, 1 modified)

## Accomplishments

- Created pipeline.rs with PipelineState (AtomicU8, IDLE/RECORDING/PROCESSING) and compare_exchange transition guard
- Implemented run_pipeline async fn: flush audio -> 100ms gate -> whisper inference (spawn_blocking) -> text formatting -> inject_text (spawn_blocking) -> reset_to_idle
- Every error/early-return path (short audio, model not loaded, whisper error, spawn panic) calls reset_to_idle() — no stuck states
- Text formatting: trim_start() + trailing space appended per CONTEXT.md locked decisions
- Empty/whitespace-only transcriptions silently discarded without touching clipboard
- Replaced single Pressed-only hotkey handler with Pressed/Released pipeline state machine
- rebind_hotkey updated to use same pipeline-aware handler pattern
- Removed app.emit("hotkey-triggered") — pipeline is fully backend-driven
- Removed unused tauri::Emitter import from lib.rs

## Task Commits

Each task was committed atomically:

1. **Task 1: Create pipeline.rs state machine and run_pipeline orchestration** - `e821d65` (feat)
2. **Task 2: Refactor hotkey handler for hold-to-talk (Pressed + Released)** - `7e2de73` (feat)

**Paused at Task 3: checkpoint:human-verify** — end-to-end dictation verification required.

## Files Created/Modified

- `src-tauri/src/pipeline.rs` — PipelineState struct, IDLE/RECORDING/PROCESSING constants, run_pipeline() async, reset_to_idle() helper
- `src-tauri/src/lib.rs` — Added mod pipeline, PipelineState::new() managed state, new Pressed+Released hotkey handler, updated rebind_hotkey, removed Emitter import

## Decisions Made

- use tauri::Manager required in pipeline.rs — app.state() is gated behind Manager trait; discovered via E0599 on first cargo check, fixed inline (Rule 1)
- Emitter import removed — app.emit("hotkey-triggered") fully replaced by backend pipeline; frontend no longer receives hotkey events for pipeline control (Pitfall 2 from RESEARCH.md)
- Tray tooltip set after successful injection (last transcription text) — useful for development debugging without opening logs
- reset_to_idle() sets tooltip to "VoiceType — idle" to provide clear idle state feedback during verification

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Added `use tauri::Manager` to pipeline.rs**
- **Found during:** Task 1 (first cargo check)
- **Issue:** `app.state::<PipelineState>()` requires the Manager trait to be in scope. pipeline.rs did not import it, causing E0599 "no method named `state`".
- **Fix:** Added `use tauri::Manager;` at the top of pipeline.rs
- **Files modified:** src-tauri/src/pipeline.rs
- **Commit:** `e821d65` (included in Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - missing trait import causing compile error)
**Impact on plan:** Required for correctness. Manager is the standard Tauri 2 trait for state access; no scope creep.

## Issues Encountered

- Plan template did not include `use tauri::Manager` in the pipeline.rs imports — standard Tauri 2 pattern, same as discovered in Phase 1 (noted in STATE.md decisions).

## Checkpoint Status

**Paused at:** Task 3 — `checkpoint:human-verify`
**What was built:** Full hold-to-talk pipeline backend wired together. Hotkey Pressed starts recording, Released stops and fires run_pipeline which transcribes via Whisper and injects text via clipboard paste.
**What needs verification:** Build with whisper feature, test in Notepad/VS Code/Chrome, verify tray state transitions, verify clipboard restore, verify blocking during processing.

## Next Steps After Verification

- Resume signal: "approved" (or describe issues for debugging)
- If approved: proceed to Plan 03-03 (if exists) or mark phase complete

## Self-Check: PASSED

- pipeline.rs: FOUND
- 03-02-SUMMARY.md: FOUND
- Commit e821d65: FOUND
- Commit 7e2de73: FOUND

---
*Phase: 03-core-pipeline*
*Completed: 2026-02-28*
