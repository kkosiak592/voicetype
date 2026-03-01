---
phase: 08-add-parakeet-tdt-model-and-optimize-transcription-latency
plan: 02
subsystem: backend
tags: [parakeet, whisper, onnx, pipeline, engine-dispatch, vad, managed-state, tauri]

# Dependency graph
requires:
  - phase: 08-add-parakeet-tdt-model-and-optimize-transcription-latency
    plan: 01
    provides: "transcribe_parakeet.rs with load_parakeet/transcribe_with_parakeet; download.rs with parakeet_model_exists/parakeet_model_dir/download_parakeet_model"
provides:
  - "TranscriptionEngine enum (Whisper/Parakeet, kebab-case serde)"
  - "ActiveEngine managed state registered on Builder for pre-setup IPC safety"
  - "ParakeetStateMutex (Arc<Mutex<ParakeetTDT>> pattern) for mutable inference access"
  - "get_engine / set_engine Tauri commands with settings.json persistence"
  - "On-demand Parakeet model loading on engine switch, startup loading if saved engine=parakeet"
  - "Engine dispatch in pipeline.rs routing to Whisper or Parakeet arm"
  - "VAD gate bypass for hold-to-talk mode (4800 sample check instead of 20-30ms Silero scan)"
  - "list_models includes Parakeet TDT entry with downloaded status"
  - "check_first_run recognizes Parakeet as valid installed model"
affects: [frontend-engine-selector, 08-03, settings-ui]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Arc<Mutex<T>> pattern for Tauri managed state where T requires &mut self (non-Sync types)"
    - "Builder-level manage() for state accessed before setup() fires (webview2 COM init issue)"
    - "Mode-conditional VAD: hold-to-talk uses sample count, toggle uses neural VAD"
    - "Engine dispatch via match on ActiveEngine — arms are mutually exclusive at runtime"

key-files:
  created: []
  modified:
    - src-tauri/src/lib.rs
    - src-tauri/src/pipeline.rs

key-decisions:
  - "Arc<Mutex<ParakeetTDT>> wrapping: outer Mutex<Option<Arc<Mutex<ParakeetTDT>>>> mirrors WhisperStateMutex; inner Mutex provides &mut for parakeet-rs 0.1.x transcribe_samples"
  - "ActiveEngine registered on Builder (not just setup) — frontend IPC can fire before setup() due to webview2 COM init"
  - "load_parakeet called with use_cuda=false on startup/switch — CUDA EP deferred until deployment environment confirmed"
  - "Hold-to-talk bypasses Silero VAD entirely, using 4800-sample (300ms) minimum only — saves 20-30ms latency"
  - "Text formatting (trim, corrections, ALL CAPS) moved outside cfg feature gates — applies to both Whisper and Parakeet outputs"
  - "set_engine reverts to Whisper and returns Err if Parakeet model fails to load or is not downloaded"

patterns-established:
  - "Engine dispatch pattern: read active engine before spawn_blocking (AppHandle not Send), then match on engine type"
  - "Parakeet inference: clone Arc from managed state, move Arc into spawn_blocking, lock inner Mutex for &mut access"

requirements-completed: [PKT-01, PKT-03, PKT-05, PKT-06]

# Metrics
duration: 5min
completed: 2026-03-01
---

# Phase 8 Plan 02: Engine Integration Summary

**Pipeline engine dispatch (Whisper/Parakeet) via ActiveEngine managed state with VAD bypass for hold-to-talk and on-demand Parakeet model loading**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-03-01T17:31:27Z
- **Completed:** 2026-03-01T17:35:44Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- Engine dispatch wired into pipeline.rs — routes to Whisper or Parakeet based on ActiveEngine managed state
- Hold-to-talk mode bypasses Silero VAD (4800 sample minimum only), saving 20-30ms per transcription
- get_engine/set_engine Tauri commands with settings.json persistence and on-demand Parakeet model loading
- Parakeet loaded at startup if saved engine is parakeet and model files exist
- list_models includes Parakeet TDT entry; check_first_run recognizes it as valid installed model

## Task Commits

Each task was committed atomically:

1. **Task 1: Add engine types, managed state, commands, model awareness to lib.rs** - `bbe3a58` (feat)
2. **Task 2: Add engine dispatch and VAD gate bypass to pipeline.rs** - `d8307d2` (feat)

**Plan metadata:** (docs commit — created below)

## Files Created/Modified

- `src-tauri/src/lib.rs` — TranscriptionEngine enum, ActiveEngine/ParakeetStateMutex managed states, read_saved_engine(), get_engine/set_engine commands, Parakeet startup loading, download_parakeet_model in invoke_handler, list_models Parakeet entry, check_first_run Parakeet awareness
- `src-tauri/src/pipeline.rs` — VAD gate bypass for hold-to-talk, engine dispatch match on ActiveEngine, Parakeet inference arm with Arc<Mutex<ParakeetTDT>>, text formatting moved outside feature gates

## Decisions Made

- `Arc<Mutex<ParakeetTDT>>` wrapping chosen over `Arc<ParakeetTDT>` because parakeet-rs 0.1.x `transcribe_samples` takes `&mut self` — `Arc` alone doesn't give mutable access
- `ActiveEngine` registered on Builder (pre-setup) because webview2 COM init in Tauri can pump the Win32 message loop before `setup()` fires, allowing frontend IPC before managed state is ready
- `load_parakeet` called with `use_cuda=false` on engine switch and startup — CUDA EP deferred until confirmed available in deployment environment
- Text formatting (trim, corrections, ALL CAPS) extracted from the whisper `#[cfg(feature = "whisper")]` block — both engines produce a `String transcription` so all post-processing is engine-agnostic

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Arc<ParakeetTDT> replaced with Arc<Mutex<ParakeetTDT>>**
- **Found during:** Task 1 (ParakeetStateMutex definition)
- **Issue:** Plan specified `Arc<ParakeetTDT>` in managed state, but `transcribe_with_parakeet` takes `&mut ParakeetTDT` — you cannot get `&mut` from `Arc` alone
- **Fix:** Changed `ParakeetStateMutex` to hold `Arc<Mutex<ParakeetTDT>>` instead; pipeline locks inner Mutex inside spawn_blocking to get `&mut` access
- **Files modified:** src-tauri/src/lib.rs, src-tauri/src/pipeline.rs
- **Verification:** cargo check passes; matches the critical_context note in the prompt
- **Committed in:** bbe3a58, d8307d2 (incorporated into task commits)

---

**Total deviations:** 1 auto-fixed (Rule 1 - Bug: incorrect Arc-only wrapping in plan)
**Impact on plan:** Required fix — plan's Arc<ParakeetTDT> would not compile. Fix aligns with critical_context note and transcribe_parakeet.rs header comment.

## Issues Encountered

None — both tasks compiled cleanly on first attempt after the Arc<Mutex<T>> correction.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Backend fully functional: Whisper and Parakeet both wired into the pipeline
- Engine switching works: get_engine/set_engine commands with persistence and on-demand loading
- Frontend UI for engine selector (toggle in Settings) is the next piece — it can invoke get_engine/set_engine directly
- No blockers

---
*Phase: 08-add-parakeet-tdt-model-and-optimize-transcription-latency*
*Completed: 2026-03-01*
