---
phase: 03-core-pipeline
plan: 01
subsystem: injection
tags: [arboard, enigo, clipboard, keyboard-simulation, tray-icon, rust, tauri]

# Dependency graph
requires:
  - phase: 01-foundation
    provides: tray.rs build_tray() and tray icon infrastructure
  - phase: 02-audio-whisper
    provides: audio capture and whisper inference (pipeline consumers of inject_text)

provides:
  - inject_text() — clipboard save/restore + Ctrl+V paste with Windows timing delays
  - TrayState enum + set_tray_state() — runtime tray icon switching for pipeline feedback
  - Three tray icon assets (idle/recording/processing)

affects: [03-core-pipeline-02, 03-core-pipeline-03]

# Tech tracking
tech-stack:
  added:
    - arboard = "3" — clipboard get_text/set_text for save/restore
    - enigo = "0.6" — Ctrl+V keyboard simulation via Key::Control + Key::Unicode('v')
  patterns:
    - inject_text is intentionally synchronous — callers wrap in tokio::task::spawn_blocking
    - Fresh Enigo instance per inject_text call (do not share instances across calls)
    - Clipboard restore failure: log warning and move on (per user decision — text already injected)
    - TrayIconBuilder::with_id("tray") required for tray_by_id() to work at runtime

key-files:
  created:
    - src-tauri/src/inject.rs
    - src-tauri/icons/tray-idle.png
    - src-tauri/icons/tray-recording.png
    - src-tauri/icons/tray-processing.png
  modified:
    - src-tauri/Cargo.toml
    - src-tauri/src/lib.rs
    - src-tauri/src/tray.rs

key-decisions:
  - "image-png feature added to tauri dependency — required for tauri::image::Image::from_bytes to compile (Image::from_bytes is gated on image-png or image-ico feature)"
  - "TrayIconBuilder::with_id(id) takes only the ID string, icon set separately via .icon() chained call — tauri 2.10.2 API verified from source"
  - "PNG format used for tray icons (not ICO) — tauri::image::Image::from_bytes accepts PNG when image-png feature is enabled"

patterns-established:
  - "Tray state changes: tray_by_id('tray') + Image::from_bytes(bytes) + tray.set_icon(Some(image)) — silent failure on None or error"
  - "Clipboard injection sequence: save (get_text().ok()) -> set -> sleep 75ms -> Ctrl+V -> sleep 120ms -> restore"

requirements-completed: [CORE-05, CORE-06]

# Metrics
duration: 35min
completed: 2026-02-28
---

# Phase 03 Plan 01: Text Injection and Tray State Summary

**arboard + enigo clipboard-paste injection with 75ms/120ms Windows timing delays, plus three-state tray icon switching via include_bytes PNG assets**

## Performance

- **Duration:** 35 min
- **Started:** 2026-02-28T14:06:48Z
- **Completed:** 2026-02-28T14:41:00Z
- **Tasks:** 2 of 2
- **Files modified:** 7 (4 created, 3 modified)

## Accomplishments
- Created inject.rs with inject_text() implementing the full clipboard save -> set -> 75ms sleep -> Ctrl+V -> 120ms sleep -> restore sequence
- Handled non-text clipboard content gracefully via get_text().ok() (returns None for images/rich-text)
- Extended tray.rs with TrayState enum (Idle/Recording/Processing), set_tray_state() using tray_by_id("tray"), and three embed PNG assets
- Fixed build_tray() to use TrayIconBuilder::with_id("tray") enabling runtime tray_by_id() access

## Task Commits

Each task was committed atomically:

1. **Task 1: Add arboard + enigo dependencies and create inject.rs** - `d32f002` (feat)
2. **Task 2: Create tray icon assets and extend tray.rs with runtime state switching** - `5577ad6` (feat)

**Plan metadata:** (docs commit — see below)

## Files Created/Modified
- `src-tauri/src/inject.rs` — Public inject_text() with full clipboard save/restore + Ctrl+V paste sequence
- `src-tauri/src/tray.rs` — Extended with TrayState enum, set_tray_state(), TrayIconBuilder::with_id("tray")
- `src-tauri/icons/tray-idle.png` — 16x16 grey PNG for idle state
- `src-tauri/icons/tray-recording.png` — 16x16 red PNG for recording state
- `src-tauri/icons/tray-processing.png` — 16x16 orange PNG for processing state
- `src-tauri/Cargo.toml` — Added arboard = "3", enigo = "0.6", image-png feature for tauri
- `src-tauri/src/lib.rs` — Added mod inject (no feature gate)

## Decisions Made
- image-png tauri feature added — Image::from_bytes is gated behind this feature in tauri 2.10.2 source; without it, the function does not exist
- PNG format chosen over ICO for tray icons — tauri accepts PNG via image-png feature, simpler to generate programmatically
- TrayIconBuilder::with_id("tray") verified from tauri 2.10.2 source: takes only ID, icon set via chained .icon()

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Added image-png feature to tauri dependency**
- **Found during:** Task 2 (tray.rs runtime state switching)
- **Issue:** `tauri::image::Image::from_bytes` is gated behind `#[cfg(any(feature = "image-ico", feature = "image-png"))]` in tauri 2.10.2. Without this feature, the function does not exist and cargo check fails with E0599.
- **Fix:** Added `image-png` to the `features` list of the tauri dependency in Cargo.toml
- **Files modified:** src-tauri/Cargo.toml
- **Verification:** cargo check passes after adding feature
- **Committed in:** `5577ad6` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - missing feature flag causing compile error)
**Impact on plan:** Required for correctness — Image::from_bytes is the standard Tauri 2 API for loading icon bytes at runtime. No scope creep.

## Issues Encountered
- RESEARCH.md referenced `Image::from_bytes` without noting it requires the `image-png` feature gate. Discovered via cargo error E0599 and confirmed by reading tauri 2.10.2 source.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- inject_text() is complete and ready for pipeline wiring in Plan 02
- set_tray_state() is complete and ready for pipeline state transitions in Plan 02
- Both are synchronous and must be called from tokio::task::spawn_blocking in the pipeline
- No blockers for Plan 02

---
*Phase: 03-core-pipeline*
*Completed: 2026-02-28*
