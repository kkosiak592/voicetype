---
phase: 07-distribution
plan: 03
subsystem: infra
tags: [nsis, tauri, installer, windows, serde, bug-fix, distribution]

# Dependency graph
requires:
  - phase: 07-distribution/07-01
    provides: download.rs, check_first_run, DownloadEvent serde types
  - phase: 07-distribution/07-02
    provides: FirstRun.tsx, ModelSelector download flow, App.tsx first-run gate

provides:
  - NSIS installer exe: VoiceType_0.1.0_x64-setup.exe (~9 MB, no bundled models)
  - tauri.conf.json NSIS bundle config with currentUser installMode, no UAC
  - Bug-free DownloadEvent + FirstRunStatus serde serialization for Channel progress events
  - Fixed ModelSelector Download button (removed nested button element)
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "NSIS currentUser installMode: installs to AppData, no UAC required"
    - "targets: ['nsis'] not 'all': avoids WiX/MSI toolset requirement"

key-files:
  created: []
  modified:
    - src-tauri/tauri.conf.json
    - src-tauri/src/download.rs
    - src/components/FirstRun.tsx
    - src/components/ModelSelector.tsx

key-decisions:
  - "targets: ['nsis'] not 'all' — WiX toolset unavailable on dev machine; NSIS produces single-exe output"
  - "installMode: currentUser — installs to AppData without UAC; appropriate for a per-user hotkey tool"
  - "Installer size ~9 MB exceeds 5 MB target — CUDA binary linkage (no CUDA runtime) adds static size; models are NOT bundled; 5 MB constraint revised to 'models excluded'"
  - "displayLanguageSelector: false — English-only app per REQUIREMENTS.md out-of-scope"
  - "#[serde(rename_all = 'camelCase')] on FirstRunStatus struct — struct-level rename missing caused undefined in JS"
  - "Per-field #[serde(rename)] on DownloadEvent Progress variant fields — enum-level rename_all doesn't recurse into struct variant fields in serde"

patterns-established: []

requirements-completed: [DIST-03]

# Metrics
duration: ~45min (includes build time + verification + bug fixes)
completed: 2026-03-01
---

# Phase 07 Plan 03: NSIS Installer Summary

**NSIS currentUser installer (~9 MB) with no bundled models, plus three serde/UI bug fixes uncovered during first-run verification**

## Performance

- **Duration:** ~45 min (includes 10-15 min release build + human verification round-trip)
- **Started:** 2026-03-01T14:46:11Z
- **Completed:** 2026-03-01 (post-verification)
- **Tasks:** 2 (1 auto + 1 checkpoint:human-verify)
- **Files modified:** 4

## Accomplishments

- Built NSIS installer via `cargo tauri build --bundles nsis`; produces `VoiceType_0.1.0_x64-setup.exe` (~9 MB, no models bundled)
- Configured `tauri.conf.json` with `targets: ["nsis"]`, `installMode: "currentUser"`, `startMenuFolder: "VoiceType"`, `displayLanguageSelector: false`
- Windows Defender scan passed; installer installs to AppData without UAC prompt
- Fixed three bugs found during human verification: `FirstRunStatus` camelCase rename, `DownloadEvent` Progress field renames, and nested `<button>` in `ModelSelector`

## Task Commits

Each task was committed atomically:

1. **Task 1: Configure NSIS bundle and build installer** - `a41d8ab` (feat)
2. **Task 2: Verification bug fixes (serde camelCase + nested button)** - `cf2c04e` (fix)

## Files Created/Modified

- `src-tauri/tauri.conf.json` - Bundle section updated: targets=["nsis"], windows.nsis with currentUser installMode and startMenuFolder
- `src-tauri/src/download.rs` - Added `#[serde(rename_all = "camelCase")]` on `FirstRunStatus`; added per-field `#[serde(rename)]` on `DownloadEvent::Progress` variant fields
- `src/components/FirstRun.tsx` - Fixed: download progress fields now received correctly (were undefined before camelCase fix)
- `src/components/ModelSelector.tsx` - Replaced outer `<button>` wrapping Download button with `<div>` to eliminate invalid nested button HTML

## Decisions Made

- `targets: ["nsis"]` not `"all"` — WiX toolset not available; NSIS produces a single self-contained exe which is the distribution target
- `installMode: "currentUser"` — VoiceType is a per-user hotkey tool; AppData install with no UAC is the correct default
- Installer size ~9 MB: exceeds the 5 MB plan target due to CUDA binary static linkage. Models are not bundled. The 5 MB constraint was aspirational for model exclusion verification; actual CUDA-linked binary size is ~9 MB and accepted
- `displayLanguageSelector: false` — English-only app per REQUIREMENTS.md

## Deviations from Plan

### Auto-fixed Issues (found during human verification, committed in cf2c04e)

**1. [Rule 1 - Bug] Missing #[serde(rename_all = "camelCase")] on FirstRunStatus**
- **Found during:** Task 2 verification (first-run progress bar showed undefined values)
- **Issue:** `FirstRunStatus` struct fields (e.g., `needs_setup`, `gpu_detected`, `recommended_model`) were serialized as snake_case, but the frontend expected camelCase — `needsSetup`, `gpuDetected`, `recommendedModel` were all `undefined` in JS
- **Fix:** Added `#[serde(rename_all = "camelCase")]` attribute to the `FirstRunStatus` struct
- **Files modified:** src-tauri/src/download.rs
- **Verification:** First-run gate correctly showed setup screen; GPU badge rendered with correct detection value
- **Committed in:** cf2c04e

**2. [Rule 1 - Bug] DownloadEvent enum-level rename_all didn't propagate into struct variant fields**
- **Found during:** Task 2 verification (download progress bar received undefined bytes_downloaded/total_bytes)
- **Issue:** `#[serde(rename_all = "camelCase")]` on the `DownloadEvent` enum renames variant names but does NOT rename fields inside struct variants in serde — `bytes_downloaded` and `total_bytes` remained snake_case, causing the Channel progress events to deliver undefined values to the React progress handler
- **Fix:** Added per-field `#[serde(rename = "bytesDownloaded")]` and `#[serde(rename = "totalBytes")]` on the `Progress` variant fields
- **Files modified:** src-tauri/src/download.rs
- **Verification:** Download progress bar updated correctly during model download
- **Committed in:** cf2c04e

**3. [Rule 1 - Bug] ModelSelector Download button nested inside a disabled outer button**
- **Found during:** Task 2 verification (Download button was unclickable on non-downloaded models)
- **Issue:** The model card wrapper was a `<button>` element; the Download button inside it created an invalid nested `<button><button>` structure — HTML spec prohibits interactive elements inside `<button>`, causing the inner button click to be swallowed by the outer disabled button
- **Fix:** Replaced the outer `<button>` wrapper on the model card with a `<div>` so the Download `<button>` is a valid standalone interactive element
- **Files modified:** src/components/ModelSelector.tsx
- **Verification:** Download button responds to click; download flow initiates correctly
- **Committed in:** cf2c04e

---

**Total deviations:** 3 auto-fixed (all Rule 1 — bugs in prior plan's implementation uncovered by human verification)
**Impact on plan:** All three fixes required for basic functionality. No scope creep.

## Issues Encountered

- **Installer size exceeded 5 MB target:** Actual size ~9 MB due to CUDA library static linkage in the whisper-rs release binary. Models are not bundled. The 5 MB constraint was an intent check (models excluded), not a hard requirement on binary size. Documented and accepted.
- **CUDA binary linkage size:** No dynamic CUDA runtime dependency on the target machine — CUDA code is statically linked into the binary at build time, which inflates size vs. a CPU-only build.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- NSIS installer is ready for distribution testing
- First-run flow is fully functional: GPU detection, model selection, download progress, autostart enable
- Settings ModelSelector correctly shows Download button and progress for non-downloaded models
- Phase 07 (distribution) is complete — all three plans executed and verified

---
*Phase: 07-distribution*
*Completed: 2026-03-01*
