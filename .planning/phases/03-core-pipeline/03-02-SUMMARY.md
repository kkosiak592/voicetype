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
    - Blocking work (whisper inference + inject_text) wrapped in tauri::async_runtime::spawn_blocking — NOT tokio::task::spawn_blocking (tokio is not a direct dependency)
    - Every early-return path in run_pipeline calls reset_to_idle() — no stuck states by design
    - use tauri::Manager must be imported in pipeline.rs to call app.state() on AppHandle
    - cfg blocks on let-bindings require explicit type annotations to satisfy type inference

key-files:
  created:
    - src-tauri/src/pipeline.rs
  modified:
    - src-tauri/src/lib.rs

key-decisions:
  - "use tauri::Manager required in pipeline.rs — app.state() on AppHandle is gated behind Manager trait (discovered via E0599 on first cargo check)"
  - "tauri::async_runtime::spawn_blocking not tokio::task::spawn_blocking — tokio is not a direct dependency; tauri re-exports its own runtime API"
  - "let transcription: String explicit annotation required — cfg blocks across two #[cfg(feature = 'whisper')] blocks confuse Rust type inference without explicit annotation"
  - "Emitter import removed from lib.rs — app.emit('hotkey-triggered') fully replaced by backend pipeline; no frontend emit needed"
  - "Tray tooltip set on successful injection for development debugging — shows last transcription text"
  - "reset_to_idle also sets tray tooltip to 'VoiceType — idle' for clear state feedback during development"

# Metrics
duration: 10min
completed: 2026-02-28
---

# Phase 03 Plan 02: Hold-to-Talk Pipeline Orchestration Summary

**PipelineState AtomicU8 state machine + run_pipeline async orchestration wiring audio, whisper, and injection into the full hold-to-talk pipeline — verified end-to-end**

## Performance

- **Duration:** ~10 min (including post-checkpoint compile fixes and verification)
- **Started:** 2026-02-28T14:19:01Z
- **Completed:** 2026-02-28T14:30:00Z
- **Tasks:** 3 of 3 (including human-verify checkpoint — APPROVED)
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
- End-to-end dictation verified: hold hotkey -> speak -> release -> text appears at cursor

## Task Commits

Each task was committed atomically:

1. **Task 1: Create pipeline.rs state machine and run_pipeline orchestration** - `e821d65` (feat)
2. **Task 2: Refactor hotkey handler for hold-to-talk (Pressed + Released)** - `7e2de73` (feat)
3. **Task 3 (post-checkpoint compile fixes)** - `8cd2850` (fix)
4. **Task 3 (docs/state)** - to be committed below (docs)

## Files Created/Modified

- `src-tauri/src/pipeline.rs` — PipelineState struct, IDLE/RECORDING/PROCESSING constants, run_pipeline() async, reset_to_idle() helper
- `src-tauri/src/lib.rs` — Added mod pipeline, PipelineState::new() managed state, new Pressed+Released hotkey handler, updated rebind_hotkey, removed Emitter import

## Decisions Made

- use tauri::Manager required in pipeline.rs — app.state() is gated behind Manager trait; discovered via E0599 on first cargo check, fixed inline (Rule 1)
- tauri::async_runtime::spawn_blocking not tokio::task — tokio is not a direct project dependency; tauri re-exports its own runtime API that wraps tokio
- Explicit `: String` type annotation on `transcription` — two separate `#[cfg(feature = "whisper")]` blocks using the same binding confused Rust's type inference; annotation resolves it
- Emitter import removed — app.emit("hotkey-triggered") fully replaced by backend pipeline; frontend no longer receives hotkey events for pipeline control (Pitfall 2 from RESEARCH.md)
- Tray tooltip set after successful injection (last transcription text) — useful for development debugging without opening logs
- reset_to_idle() sets tooltip to "VoiceType — idle" to provide clear idle state feedback during verification

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Added `use tauri::Manager` to pipeline.rs**
- **Found during:** Task 1 (first cargo check)
- **Issue:** `app.state::<PipelineState>()` requires the Manager trait in scope. pipeline.rs did not import it, causing E0599 "no method named `state`".
- **Fix:** Added `use tauri::Manager;` at the top of pipeline.rs
- **Files modified:** src-tauri/src/pipeline.rs
- **Commit:** `e821d65` (included in Task 1 commit)

**2. [Rule 1 - Bug] Replaced `tokio::task::spawn_blocking` with `tauri::async_runtime::spawn_blocking`**
- **Found during:** Task 3 (post-checkpoint build for verification)
- **Issue:** `tokio` is not a direct dependency of the project — it is used transitively via tauri. Referencing `tokio::task::spawn_blocking` directly caused a compile error. Tauri exposes `tauri::async_runtime::spawn_blocking` as the correct API.
- **Fix:** Replaced both `tokio::task::spawn_blocking` call sites in pipeline.rs with `tauri::async_runtime::spawn_blocking`
- **Files modified:** src-tauri/src/pipeline.rs
- **Commit:** `8cd2850`

**3. [Rule 1 - Bug] Added `: String` type annotation to `transcription` binding**
- **Found during:** Task 3 (post-checkpoint build for verification)
- **Issue:** The `transcription` let-binding under `#[cfg(feature = "whisper")]` was referenced in a second `#[cfg(feature = "whisper")]` block further down. The cfg-gated multi-block structure confused Rust's type inference, requiring an explicit `: String` annotation.
- **Fix:** Changed `let transcription = {` to `let transcription: String = {`
- **Files modified:** src-tauri/src/pipeline.rs
- **Commit:** `8cd2850`

---

**Total deviations:** 3 auto-fixed (all Rule 1 — compile errors)
**Impact on plan:** Required for correctness; no scope creep. The spawn_blocking API change is a project-level pattern to document for future plans.

## Verification Result

**Task 3 checkpoint:human-verify — APPROVED**

User confirmed end-to-end dictation works:
- Hold hotkey -> tray turns red (recording)
- Release -> tray turns orange (processing)
- Text appears at cursor with trailing space
- Clipboard restored after injection
- Pipeline blocked during processing (no double-starts)
- Tray returns to idle after completion

## Self-Check: PASSED

- pipeline.rs: FOUND
- 03-02-SUMMARY.md: FOUND
- Commit e821d65 (Task 1): FOUND
- Commit 7e2de73 (Task 2): FOUND
- Commit 8cd2850 (Task 3 fixes): FOUND

---
*Phase: 03-core-pipeline*
*Completed: 2026-02-28*
