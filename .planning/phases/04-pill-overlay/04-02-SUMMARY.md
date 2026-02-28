---
phase: 04-pill-overlay
plan: 02
subsystem: ui
tags: [tauri, react, audio-visualizer, css-animation, rms, pill-overlay]

# Dependency graph
requires:
  - phase: 04-pill-overlay
    plan: 01
    provides: Frameless transparent pill window with no-focus-steal guarantee, ready for content
  - phase: 03-core-pipeline
    provides: PipelineState state machine + run_pipeline() + reset_to_idle() in pipeline.rs

provides:
  - RMS level streaming from audio buffer at ~30fps via pill.rs start_level_stream()
  - Pill state events emitted from backend (pill-show, pill-hide, pill-state, pill-level, pill-result)
  - FrequencyBars visualizer: 15 animated bars driven by real microphone RMS level
  - Full pill state machine: hidden/recording/processing/success/error
  - CSS @property animated gradient border for processing state
  - Success/error flash animations via CSS keyframes

affects:
  - Phase 05+ (pill overlay is now a complete live status indicator)

# Tech tracking
tech-stack:
  added:
    - tokio = { version = "1", features = ["time"] } — async sleep in RMS level stream loop
  patterns:
    - AtomicBool-controlled async loop for RMS streaming (start/stop via store/load Ordering::Relaxed)
    - try_lock() (not lock()) on audio buffer in async context — prevents deadlock with cpal callback
    - CSS @property for smooth angle interpolation in conic-gradient animated border
    - PillDisplayState type + useRef hideTimerRef for race-condition-free state transitions
    - Ignore "idle" pill-state event — let pill-hide handle hidden transition (avoids flash race)

key-files:
  created:
    - src-tauri/src/pill.rs
    - src/components/FrequencyBars.tsx
  modified:
    - src-tauri/src/lib.rs
    - src-tauri/src/pipeline.rs
    - src-tauri/Cargo.toml
    - src/Pill.tsx
    - src/pill.css

key-decisions:
  - "tokio added as explicit dep with time feature — tauri re-exports its runtime but tokio crate not directly available for tokio::time::sleep"
  - "load() called without options in Pill.tsx — autoSave: false is not a valid StoreOptions shape in this plugin version (requires defaults field)"
  - "idle pill-state event ignored in Pill.tsx — pill-hide from reset_to_idle() handles hidden transition, preventing race where idle clears success/error flash"
  - "try_lock() used in compute_rms — audio callback thread holds buffer lock briefly; lock() would deadlock the tokio worker thread"
  - "core:window:allow-show, allow-hide, allow-set-position must be explicitly granted in capabilities — not included in core:default (same pattern as allow-set-focusable from 04-01)"

requirements-completed: [UI-03, UI-04]

# Metrics
duration: ~60 minutes
completed: 2026-02-28
---

# Phase 4 Plan 02: Pill Visualizer + Pipeline State Display Summary

**RMS level streaming loop in Rust + FrequencyBars visualizer + full pill state machine (recording/processing/success/error) wired to the hold-to-talk pipeline — verified end-to-end with no-focus-steal confirmed**

## Performance

- **Duration:** ~60 min
- **Started:** 2026-02-28
- **Completed:** 2026-02-28
- **Tasks:** 3 (2 auto + 1 human-verify checkpoint, all complete)
- **Files modified:** 8

## Accomplishments

- Created `pill.rs` with `start_level_stream()` — AtomicBool-controlled async loop reading audio buffer at ~30fps, computing RMS, emitting normalized 0.0-1.0 levels to pill window
- Wired both hotkey handlers (setup + rebind_hotkey) in `lib.rs` to emit pill-show/pill-state/start level stream on IDLE->RECORDING, and stop stream + emit pill-state:processing on RECORDING->PROCESSING
- Modified `pipeline.rs` `reset_to_idle()` to emit pill-state:idle + pill-hide; added pill-result:success after successful injection and pill-result:error on all 5 error/discard paths
- Created `FrequencyBars.tsx` with 15 animated vertical bars, center bars taller (simulates speech frequency energy distribution), 2px min height, jitter variation per render
- Updated `pill.css` with CSS `@property --border-angle` animated gradient border (indigo/purple/cyan, 2s rotation) for processing state, plus success-flash (300ms green glow) and error-flash (500ms red glow)
- Replaced Pill.tsx placeholder with full state machine: 5 states (hidden/recording/processing/success/error), all 5 event listeners, hideTimerRef for race-condition-free transitions
- End-to-end verification APPROVED: all visual states confirmed, frequency bars respond to voice, no-focus-steal maintained with Notepad/VS Code/Chrome

## Task Commits

Each task was committed atomically:

1. **Task 1: Create pill.rs RMS streaming + emit pill events from backend** - `2639be8` (feat)
2. **Task 2: Build frequency bars component + pill state rendering with CSS animations** - `0cdb625` (feat)
3. **Task 3: End-to-end pill overlay verification** - APPROVED by user (no separate commit — verification only)

**Deviation fix:** `02edda4` — add window show/hide/set-position capability permissions (required for pill to appear at all)

## Files Created/Modified

- `src-tauri/src/pill.rs` — New module: start_level_stream() + compute_rms(), AtomicBool loop, try_lock() on audio buffer
- `src-tauri/src/lib.rs` — Added mod pill, LevelStreamActive managed state, pill events in both hotkey handlers, Arc/AtomicBool imports unconditional
- `src-tauri/src/pipeline.rs` — Added use tauri::Emitter, pill-result events on all paths, pill-state:idle + pill-hide in reset_to_idle()
- `src-tauri/Cargo.toml` — Added tokio = { version = "1", features = ["time"] }
- `src-tauri/capabilities/default.json` — Added core:window:allow-show, allow-hide, allow-set-position
- `src/components/FrequencyBars.tsx` — New component: 15 bars, BAND_MULTIPLIERS array, jitter, transition-[height] animation
- `src/Pill.tsx` — Full state machine replacing placeholder, 5 event listeners, hideTimerRef, drag handling preserved
- `src/pill.css` — CSS @property animated border, success-flash, error-flash keyframes

## Decisions Made

- **tokio explicit dependency:** `tokio::time::sleep` required for async sleep in the RMS loop. Tauri's runtime is tokio but `tokio` is not directly available as a crate without explicit dep. Added with `features = ["time"]` only.
- **load() without options:** `{ autoSave: false }` is not a valid `StoreOptions` shape — the type requires a `defaults` field. Using `load("settings.json")` without options (same as original Pill.tsx).
- **Ignore idle pill-state event:** Backend emits pill-state:idle then pill-hide in reset_to_idle(). If pill-state:idle set displayState to hidden, it would cancel the success/error flash CSS animation before it completes. Solution: ignore "idle" from pill-state, let pill-hide handle the hidden transition.
- **try_lock() in compute_rms:** Audio callback thread can hold the buffer Mutex briefly during sample writes. Using try_lock() and returning 0.0 on contention avoids deadlock on the tokio worker thread. Same pattern as Phase 02 cpal callbacks.
- **Capability permissions for show/hide/set-position:** Same pattern as Plan 04-01 where allow-set-focusable and allow-start-dragging were missing. Every window API call in Tauri 2 requires an explicit allow-* grant in capabilities/default.json — core:default does not cover these operations.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] tokio not available as direct crate**
- **Found during:** Task 1 verification (cargo check)
- **Issue:** `tokio::time::sleep` in pill.rs failed with "use of unresolved module or unlinked crate tokio" — tokio is not a direct project dependency
- **Fix:** Added `tokio = { version = "1", features = ["time"] }` to src-tauri/Cargo.toml
- **Files modified:** src-tauri/Cargo.toml
- **Commit:** 2639be8

**2. [Rule 1 - Bug] load() options type mismatch**
- **Found during:** Task 2 verification (tsc --noEmit)
- **Issue:** Plan used `load("settings.json", { autoSave: false })` but `StoreOptions` type requires `defaults` field — TypeScript error TS2345
- **Fix:** Called `load("settings.json")` without options, matching original Pill.tsx pattern
- **Files modified:** src/Pill.tsx
- **Commit:** 0cdb625

**3. [Rule 1 - Bug] "idle" string not in PillDisplayState union**
- **Found during:** Task 2 verification (tsc --noEmit)
- **Issue:** Plan's `state === "idle"` comparison caused TS2367 (no overlap) because PillDisplayState doesn't include "idle"
- **Fix:** Changed to check `e.payload === "idle"` (string comparison on the raw payload) before casting to PillDisplayState
- **Files modified:** src/Pill.tsx
- **Commit:** 0cdb625

**4. [Rule 2 - Missing Critical] Explicit window capability permissions for show/hide/set-position**
- **Found during:** Task 3 (end-to-end verification)
- **Issue:** `appWindow.show()` silently failing — pill never appeared. Tauri 2 requires explicit allow-* grants for every window operation; allow-show, allow-hide, allow-set-position not included in core:default.
- **Fix:** Added `core:window:allow-show`, `core:window:allow-hide`, `core:window:allow-set-position` to capabilities/default.json
- **Files modified:** src-tauri/capabilities/default.json
- **Commit:** 02edda4

---

**Total deviations:** 4 auto-fixed (1 blocking dependency, 2 TypeScript type bugs, 1 missing critical capability)
**Impact on plan:** All fixes were necessary for compilation and correct operation. No scope changes.

## Issues Encountered

None beyond the four auto-fixed deviations documented above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 04 complete — pill overlay is a fully functional live status indicator, verified end-to-end
- Phase 05 (VAD) can reuse the AtomicBool streaming loop pattern from pill.rs for VAD triggering
- Pre-Phase 5 blocker remains: silero-vad-rust crate version needs verification on crates.io before writing Cargo.toml

---
*Phase: 04-pill-overlay*
*Completed: 2026-02-28*
