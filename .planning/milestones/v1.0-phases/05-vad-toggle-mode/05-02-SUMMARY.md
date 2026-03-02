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
  modified: [src-tauri/src/lib.rs, src-tauri/src/pipeline.rs, src/App.tsx, src/lib/store.ts, src-tauri/src/vad.rs]
key-decisions:
  - "cancel_stale_vad_worker extracted as separate fn in pipeline.rs — avoids State borrow lifetime issue (E0597) when inlining the block in run_pipeline()"
  - "let result = ...; result pattern in cancel_stale_vad_worker forces MutexGuard temporary to drop before State binding goes out of scope (compiler-suggested fix)"
  - "RecordingModeToggle uses radio-card UI (not toggle switch) — two cards with descriptions, indigo border for selected"
  - "set_recording_mode persists to settings.json via serde_json merge — same pattern as hotkey persistence"
  - "SILENCE_FRAMES_THRESHOLD increased from 47 to 94 (1.5s -> 3.0s) — user feedback: 1.5s auto-stop too aggressive during natural speech pauses"
requirements-completed: [REC-02, REC-04]
metrics:
  duration: "~20 minutes (including checkpoint and continuation)"
  completed: "2026-02-28"
  tasks_completed: 3
  tasks_total: 3
  files_changed: 6
---

# Phase 05 Plan 02: Toggle Recording Mode Summary

**Toggle mode hotkey handler with VAD auto-stop on 3.0s silence, second tap for instant stop, RecordingMode managed state, settings persistence, radio-card settings UI — fully verified end-to-end with real hardware.**

## Performance

- **Duration:** ~20 minutes (including checkpoint and continuation)
- **Completed:** 2026-02-28
- **Tasks:** 3/3
- **Files modified:** 6

## Accomplishments

- Toggle recording mode: tap hotkey to start, VAD auto-stops after 3.0s silence (increased from 1.5s on user feedback), second tap for instant hard stop
- RecordingMode managed state backed by AtomicU8, persisted to settings.json across restarts
- Mode-aware hotkey handlers in setup() and rebind_hotkey() — hold-to-talk behavior unchanged (no regression)
- RecordingModeToggle radio-card UI in settings panel with indigo-highlighted selected state
- VAD worker cancellation on second tap prevents double-transcription

## Task Commits

Each task was committed atomically:

1. **Task 1: RecordingMode state, mode-aware hotkey handlers, VadWorker wiring** - `81c40fe` (feat)
2. **Task 2: Settings UI toggle for recording mode selection** - `648e52a` (feat)
3. **Task 3: Verify toggle mode, user feedback — silence timeout 1.5s -> 3.0s** - `725e792` (fix)

**Plan metadata:** (committed after state update)

## Files Created/Modified

- `src-tauri/src/lib.rs` - Mode enum, RecordingMode/VadWorkerState managed state, mode-aware hotkey handlers, set_recording_mode/get_recording_mode Tauri commands, read_saved_mode()
- `src-tauri/src/pipeline.rs` - cancel_stale_vad_worker() helper called at run_pipeline() entry to prevent double-trigger
- `src-tauri/src/vad.rs` - SILENCE_FRAMES_THRESHOLD: 47 -> 94 (1.5s -> 3.0s)
- `src/lib/store.ts` - recordingMode field in AppSettings and DEFAULTS
- `src/components/RecordingModeToggle.tsx` - Created: radio-card toggle for mode selection, invokes set_recording_mode Tauri command
- `src/App.tsx` - Imports RecordingModeToggle, loads recordingMode from store, adds Recording Mode settings section

## Decisions Made

- `cancel_stale_vad_worker` extracted as a separate fn in pipeline.rs to avoid E0597 borrow lifetime error (State binding dropped while MutexGuard temporary still borrows it — compiler-suggested fix: separate fn forces correct drop order)
- `let result = ...; result` pattern in MutexGuard operations forces temporary drop before State binding scope ends
- RecordingModeToggle uses radio-card layout (not toggle switch) — cards with descriptions convey the behavioral difference better than a simple on/off toggle
- `set_recording_mode` persists via serde_json merge into settings.json — same pattern as hotkey persistence, no extra deps
- SILENCE_FRAMES_THRESHOLD raised to 94 frames (3.0s) after user verified 1.5s was too aggressive — people naturally pause mid-thought

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Borrow checker E0597: `vad_state` does not live long enough in pipeline.rs**
- **Found during:** Task 1 cargo check
- **Issue:** Inlining the VAD cancel block in `run_pipeline()` caused E0597 — `State<'_, VadWorkerState>` binding dropped while `Result<MutexGuard>` temporary still borrows it
- **Fix:** Extracted `cancel_stale_vad_worker()` as a separate `fn`. Inside, used `let result = match vad_state.0.lock() { ... }; result` to force the `MutexGuard` temporary to drop before `vad_state` goes out of scope
- **Files modified:** src-tauri/src/pipeline.rs
- **Commit:** 81c40fe (fixed before commit)

### User Feedback Applied

**Silence timeout adjustment (applied at Task 3 verification):**
- User confirmed toggle mode worked end-to-end but reported 1.5s silence timeout was too aggressive — recordings stopped during natural speech pauses
- Changed `SILENCE_FRAMES_THRESHOLD` from 47 to 94 (3.0s at 32ms/chunk)
- Committed: 725e792

## Issues Encountered

None beyond the E0597 borrow checker issue (auto-fixed, documented above).

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Phase 05 complete: Silero VAD integration (Plan 01) and toggle mode (Plan 02) both verified
- Phase 06 (Win32 WS_EX_NOACTIVATE focus isolation) is next — pre-phase blocker noted in STATE.md: exact Rust API call needs identification from Tauri source

---
*Phase: 05-vad-toggle-mode*
*Completed: 2026-02-28*

## Self-Check: PASSED

- src-tauri/src/lib.rs: FOUND
- src-tauri/src/pipeline.rs: FOUND
- src/lib/store.ts: FOUND
- src/components/RecordingModeToggle.tsx: FOUND
- src/App.tsx: FOUND
- src-tauri/src/vad.rs: FOUND
- Commit 81c40fe: FOUND
- Commit 648e52a: FOUND
- Commit 725e792: FOUND
