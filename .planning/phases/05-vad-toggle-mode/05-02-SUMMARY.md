---
phase: 05-vad-toggle-mode
plan: 02
subsystem: vad, hotkey, settings-ui
tags: [toggle-mode, vad, hotkey, recording-mode, settings, hold-to-talk]
dependency_graph:
  requires: [05-01 (vad.rs, VadWorkerHandle, spawn_vad_worker)]
  provides: [RecordingMode state, VadWorkerState, toggle hotkey handler, RecordingModeToggle UI, set_recording_mode command]
  affects: [src-tauri/src/lib.rs, src-tauri/src/pipeline.rs, src/App.tsx, src/lib/store.ts, src/components/RecordingModeToggle.tsx]
tech_stack:
  added: []
  patterns: [AtomicU8-backed-mode-state, CAS-based-pipeline-transition, radio-card-UI, settings-persistence-merge-pattern]
key_files:
  created: [src/components/RecordingModeToggle.tsx]
  modified: [src-tauri/src/lib.rs, src-tauri/src/pipeline.rs, src/App.tsx, src/lib/store.ts]
decisions:
  - "cancel_stale_vad_worker extracted as separate fn in pipeline.rs — avoids State borrow lifetime issue (E0597) when inlining the block in run_pipeline()"
  - "let result = ...; result pattern in cancel_stale_vad_worker forces MutexGuard temporary to drop before State binding goes out of scope (compiler-suggested fix)"
  - "RecordingModeToggle uses radio-card UI (not toggle switch) — two cards with descriptions, indigo border for selected"
  - "set_recording_mode persists to settings.json via serde_json merge — same pattern as hotkey persistence"
metrics:
  duration: "410 seconds (~7 minutes)"
  completed: "2026-03-01"
  tasks_completed: 2
  tasks_total: 3
  files_changed: 5
  checkpoint: "Paused at Task 3 (human-verify)"
---

# Phase 05 Plan 02: Toggle Recording Mode Summary

**One-liner:** Toggle mode hotkey handler with VAD auto-stop on ~1.5s silence, second tap for instant stop, RecordingMode managed state, settings persistence, and radio-card settings UI.

**Status:** Tasks 1-2 complete. Paused at Task 3 (human verification checkpoint).

## What Was Built

### Task 1: RecordingMode state, mode-aware hotkey handlers, VadWorker wiring

**In `src-tauri/src/lib.rs`:**

Added `Mode` enum (`HoldToTalk = 0`, `Toggle = 1`) and `RecordingMode` struct backed by `AtomicU8`. The `RecordingMode::get()` / `set()` methods use `Relaxed` ordering — acceptable since mode changes are low-frequency user actions, not synchronization primitives.

Added `VadWorkerState(pub std::sync::Mutex<Option<vad::VadWorkerHandle>>)` managed state. The `Mutex<Option<...>>` pattern lets the handle be taken (replaced with `None`) by any cancellation path without cloning.

Added `read_saved_mode(app: &tauri::App) -> Mode` — reads `"recording_mode"` key from `settings.json`, returns `Mode::HoldToTalk` on any error (hold-to-talk is the default per CONTEXT.md).

Added `set_recording_mode` Tauri command: updates `RecordingMode` managed state atomically, then merges `"recording_mode"` into `settings.json` (same serde_json merge pattern as hotkey persistence). Added `get_recording_mode` command: returns `"toggle"` or `"hold"` string.

Registered both commands in `generate_handler![]`.

In `setup()`: reads saved mode via `read_saved_mode()`, manages `RecordingMode::new(saved_mode)` and `VadWorkerState(Mutex::new(None))` after `PipelineState`.

**Hotkey handler rewrite (both `setup()` and `rebind_hotkey()`):**

`ShortcutState::Pressed`:
- `HoldToTalk`: unchanged — CAS IDLE→RECORDING, start audio + level stream
- `Toggle`:
  - First tap (IDLE→RECORDING): start audio + level stream + spawn VAD worker, store handle in `VadWorkerState`
  - Second tap (RECORDING→PROCESSING): take + cancel VAD worker, stop level stream, emit pill-state processing, update tray, spawn `run_pipeline()`
  - PROCESSING state: ignored (CAS fails silently — OS key repeat safe)

`ShortcutState::Released`:
- `HoldToTalk`: unchanged — CAS RECORDING→PROCESSING, stop level stream, spawn `run_pipeline()`
- `Toggle`: no-op (release ignored — VAD or second tap controls stop)

**In `src-tauri/src/pipeline.rs`:**

Added `cancel_stale_vad_worker(app: &tauri::AppHandle)` helper function. Called at the top of `run_pipeline()` before the VAD gate check. Takes the `VadWorkerHandle` out of managed state and cancels it — prevents double-pipeline-trigger if the VAD worker fires just as `run_pipeline()` is entered from a second tap.

Key implementation detail: used `let result = match vad_state.0.lock() { ... }; result` pattern (compiler E0597 fix — forces `MutexGuard` temporary to drop before the `State` binding leaves scope).

### Task 2: Settings UI toggle for recording mode selection

**`src/lib/store.ts`:** Added `recordingMode: 'hold' | 'toggle'` to `AppSettings` interface and `recordingMode: 'hold'` to `DEFAULTS`.

**`src/components/RecordingModeToggle.tsx`:** New component with two radio-card options side by side:
- "Hold to talk" + "Hold the hotkey while speaking. Release to transcribe."
- "Toggle" + "Tap to start. Tap again or wait for auto-stop."

Selected card: `border-indigo-500 bg-indigo-50` (light) / `border-indigo-400 bg-indigo-950` (dark). Unselected: `border-gray-200` with hover. On select: `invoke('set_recording_mode', { mode })` then `store.set('recordingMode', mode)`.

**`src/App.tsx`:**
- Imported `RecordingModeToggle`
- Added `recordingMode` state initialized to `DEFAULTS.recordingMode`
- `loadSettings()` reads `recordingMode` from store
- Recording Mode section added between Hotkey and Appearance sections (with hr dividers)

## Commits

| Task | Commit | Description |
|------|--------|-------------|
| 1 | 81c40fe | feat(05-02): add RecordingMode state, mode-aware hotkey handlers, VadWorker wiring |
| 2 | 648e52a | feat(05-02): add recording mode settings UI — RecordingModeToggle component and App.tsx wiring |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Borrow checker E0597: `vad_state` does not live long enough in pipeline.rs**
- **Found during:** Task 1 cargo check
- **Issue:** Inlining the VAD cancel block in `run_pipeline()` caused E0597 — `State<'_, VadWorkerState>` binding dropped while `Result<MutexGuard>` temporary still borrows it
- **Fix:** Extracted `cancel_stale_vad_worker()` as a separate `fn`. Inside, used `let result = match vad_state.0.lock() { ... }; result` to force the `MutexGuard` temporary to drop before `vad_state` goes out of scope (compiler-suggested pattern)
- **Files modified:** src-tauri/src/pipeline.rs
- **Commit:** 81c40fe (fixed before commit)

## Task 3 (Checkpoint — Pending)

Task 3 is a `checkpoint:human-verify` — human verification with real hardware required. See checkpoint message below.

## Self-Check: PASSED

- src-tauri/src/lib.rs: FOUND
- src-tauri/src/pipeline.rs: FOUND
- src/lib/store.ts: FOUND
- src/components/RecordingModeToggle.tsx: FOUND
- src/App.tsx: FOUND
- Commit 81c40fe: FOUND
- Commit 648e52a: FOUND
