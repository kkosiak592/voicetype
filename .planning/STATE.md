---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: unknown
last_updated: "2026-03-01T01:56:42.701Z"
progress:
  total_phases: 6
  completed_phases: 6
  total_plans: 14
  completed_plans: 14
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-27)

**Core value:** Voice dictation must feel instant — sub-1500ms from end-of-speech to text appearing at the cursor, with zero internet dependency.
**Current focus:** Phase 5 — VAD (Voice Activity Detection)

## Current Position

Phase: 5 of 7 (VAD + Toggle Mode) — COMPLETE
Plan: 2 of 2 in phase 05 complete — toggle mode hotkey handler, RecordingMode state, VadWorker wiring, settings UI, silence timeout 3.0s
Status: Plan 05-02 complete — RecordingMode/VadWorkerState managed state, mode-aware hotkey handlers, RecordingModeToggle UI, settings persistence. SILENCE_FRAMES_THRESHOLD=94 (3.0s). Phase 06 next.
Last activity: 2026-02-28 - Completed plan 05-02: toggle mode end-to-end verified, silence timeout adjusted to 3.0s

Progress: [██████████] 100%

## Performance Metrics

**Velocity:**
- Total plans completed: 0
- Average duration: —
- Total execution time: —

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| - | - | - | - |

**Recent Trend:**
- Last 5 plans: —
- Trend: —

*Updated after each plan completion*
| Phase 01-foundation P01 | 22 | 1 tasks | 23 files |
| Phase 01-foundation P02 | 5 | 1 tasks | 4 files |
| Phase 01-foundation P03 | 4 | 1 tasks | 7 files |
| Phase 02-audio-whisper P01 | 14 | 2 tasks | 3 files |
| Phase 02-audio-whisper P03 | 14 | 1 tasks | 4 files |
| Phase 03-core-pipeline P01 | 35 | 2 tasks | 7 files |
| Phase 03-core-pipeline P02 | 2 | 2 tasks | 2 files |
| Phase 04-pill-overlay P02 | 60 | 3 tasks | 8 files |
| Phase 04.1 P01 | 3 | 2 tasks | 4 files |
| Phase 04.1 P02 | 25 | 2 tasks | 2 files |
| Phase 05-vad-toggle-mode P01 | 884 | 2 tasks | 4 files |
| Phase 05-vad-toggle-mode P02 | 1200 | 3 tasks | 6 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- [Init]: whisper.cpp over faster-whisper — CUDA 11.7 compatibility (faster-whisper requires CUDA 12)
- [Init]: Clipboard paste as primary injection — 50-100ms delays must be built in from day one, not retrofitted
- [Init]: Win32 WS_EX_NOACTIVATE required for overlay — Tauri config `focus: false` is insufficient on Windows (issue #11566)
- [Init]: CUDA build must set CMAKE_CUDA_ARCHITECTURES=61 explicitly — silent CPU fallback is a known failure mode
- [Phase 01-foundation]: show_menu_on_left_click(false) replaces deprecated menu_on_left_click in Tauri 2.10.x tray API
- [Phase 01-foundation]: use tauri::Manager must be explicitly imported to call get_webview_window on AppHandle — not re-exported from tauri prelude
- [Phase 01-foundation]: App identifier must not end in .app — use com.voicetype.desktop to avoid macOS bundle extension conflict
- [Phase 01-foundation 01-02]: Global-shortcut plugin must be registered in setup() via app.handle().plugin() with #[cfg(desktop)], not in builder chain — CLI auto-inserts incorrectly
- [Phase 01-foundation 01-02]: use tauri::Emitter required in shortcut handler closures — applies everywhere app.emit() is called
- [Phase 01-foundation 01-02]: desktop.json capability windows list must match actual window labels — CLI generates "main" but this app only has "settings"
- [Phase 01-foundation 01-03]: read_saved_hotkey() uses std::fs + serde_json directly — tauri-plugin-store Rust API requires async, not usable in synchronous setup()
- [Phase 01-foundation 01-03]: Tailwind v4 dark mode uses @variant dark in CSS — no tailwind.config.js, @variant replaces darkMode: 'class' config key
- [Phase 01-foundation 01-03]: e.code used for hotkey normalization in HotkeyCapture — layout-independent, maps directly to tauri shortcut format
- [Phase 02-audio-whisper 02-02]: whisper-rs cuda feature requires CUDA_PATH env var at build time — CUDA Toolkit must be installed (not just drivers)
- [Phase 02-audio-whisper 02-02]: WhisperState uses Option<Arc<WhisperContext>> so app starts without model, logs warning with download instructions
- [Phase 02-audio-whisper 02-02]: CMAKE_CUDA_ARCHITECTURES=61 must be set before build for Pascal arch (P2000) — silent CPU fallback if omitted
- [Phase 02-audio-whisper 02-01]: cpal 0.17 SampleRate is type alias u32, not tuple struct — access directly without .0 field
- [Phase 02-audio-whisper 02-01]: whisper-rs requires LIBCLANG_PATH even without cuda feature (bindgen generates C FFI) — make optional behind Cargo feature flag when env not available
- [Phase 02-audio-whisper 02-01]: try_lock() not lock() in cpal audio callbacks — lock() can deadlock the callback thread (cpal issue #970)
- [Phase 02-audio-whisper 02-03]: nvml-wrapper 0.10 tied to whisper Cargo feature — no extra build overhead for non-whisper builds
- [Phase 02-audio-whisper 02-03]: detect_gpu() falls back to Cpu on any NVML error (no GPU, no drivers, init failure) — safe default
- [Phase 02-audio-whisper 02-03]: force_cpu_transcribe creates fresh WhisperContext per call with use_gpu(false) — not stored in managed state, Phase 2 test-only command
- [Phase 02-audio-whisper 02-03]: LIBCLANG_PATH/BINDGEN_EXTRA_CLANG_ARGS Windows user env vars don't propagate to bash shell — build must run via PowerShell (build-whisper.ps1)
- [Phase 03-core-pipeline 03-01]: tauri::image::Image::from_bytes is gated behind image-png (or image-ico) Cargo feature — must add "image-png" to tauri features for runtime icon loading
- [Phase 03-core-pipeline 03-01]: TrayIconBuilder::with_id(id) takes only the ID string — icon set via separate .icon() chain; verified from tauri 2.10.2 source
- [Phase 03-core-pipeline 03-01]: PNG format accepted for tray icons when image-png feature enabled — no need for ICO conversion
- [Phase 03-core-pipeline]: use tauri::Manager required in pipeline.rs — app.state() on AppHandle is gated behind Manager trait (same pattern as Phase 01)
- [Phase 03-core-pipeline]: Emitter import removed from lib.rs — hotkey pipeline is fully backend-driven; frontend no longer receives hotkey events for pipeline control
- [Phase 03-core-pipeline 03-02]: tauri::async_runtime::spawn_blocking not tokio::task::spawn_blocking — tokio is not a direct project dep; tauri re-exports its own runtime API wrapping tokio
- [Phase 03-core-pipeline 03-02]: cfg-gated let-bindings require explicit type annotation — two #[cfg(feature = 'whisper')] blocks using same binding confuse Rust type inference; use 'let x: Type = {' pattern
- [Phase 04-pill-overlay 04-01]: set_focusable(false) blocks startDragging on Windows — must toggle focusable(true) before startDragging(), restore focusable(false) on mouseup
- [Phase 04-pill-overlay 04-01]: core:window:allow-set-focusable and core:window:allow-start-dragging must be added explicitly to capabilities — not granted by core:default
- [Phase 04-pill-overlay 04-01]: data-tauri-drag-region does not work on unfocusable windows — use startDragging() API for all overlay drag
- [Phase 04-pill-overlay 04-01]: pill.html has no devUrl — dist/ must be pre-built before npx tauri dev; run npx vite build first
- [Phase 04-pill-overlay]: tokio added as explicit dep with time feature — tauri re-exports its runtime but tokio crate not directly available for tokio::time::sleep
- [Phase 04-pill-overlay]: ignore idle pill-state event in Pill.tsx — pill-hide from reset_to_idle() handles hidden transition, preventing race where idle clears success/error flash before animation completes
- [Phase 04-pill-overlay 04-02]: core:window:allow-show, allow-hide, allow-set-position must be explicitly granted in capabilities — not included in core:default (same pattern as allow-set-focusable from 04-01)
- [Phase 04.1-01]: No backdrop-filter in pill.css — Windows WebView2 transparent window bug #4945 makes it silently fail
- [Phase 04.1-01]: FrequencyBars reads level via useRef in RAF loop — prevents restarting animation on every audio update
- [Phase 04.1-01]: Gaussian bell curve replaces BAND_MULTIPLIERS for bar height distribution — 12 fixed bars, more natural center-tall waveform
- [Phase 04.1-02]: animState ref (not state) separates entrance/exit lifecycle from displayState — prevents content flash during scale-down exit
- [Phase 04.1-02]: Error dismiss is silent — no displayState change, previous content scales away with pill per "no visual punishment" design principle
- [Phase 04.1-02]: Success auto-dismiss: 600ms hold (280ms draw + 320ms hold), then exit animation, then window hide
- [Phase 04.1-02]: pill window expanded 120x40 → 160x48 for FrequencyBars clearance and indigo glow room
- [Phase 05-01]: No use crate::pipeline; import in vad.rs — pipeline referenced via inline crate::pipeline:: paths to avoid circular module coupling
- [Phase 05-01]: VAD constants: SPEECH_PROBABILITY_THRESHOLD=0.5 (Silero default), SILENCE_FRAMES_THRESHOLD=94 (~3.0s, updated in 05-02), MIN_SPEECH_FRAMES=9 (~300ms), MAX_RECORDING_FRAMES=1875 (60s cap)
- [Phase 05-02]: cancel_stale_vad_worker extracted as separate fn in pipeline.rs to avoid State borrow lifetime issue (E0597)
- [Phase 05-02]: let result = ...; result pattern in MutexGuard operations forces temporary drop before State binding scope ends (compiler-suggested E0597 fix)
- [Phase 05-02]: SILENCE_FRAMES_THRESHOLD increased from 47 to 94 (1.5s -> 3.0s) — user feedback: 1.5s auto-stop too aggressive during natural speech pauses

### Roadmap Evolution

- Phase 04.1 inserted after Phase 04: Premium pill overlay UI polish (URGENT) — fix outline frame, premium waveform visualizer, modern aesthetic

### Pending Todos

None yet.

### Quick Tasks Completed

| # | Description | Date | Commit | Status | Directory |
|---|-------------|------|--------|--------|-----------|
| 1 | Premium pill UI rework — monochrome luxury redesign | 2026-02-28 | 52d726c | Verified | [1-premium-pill-ui-rework-monochrome-luxury](./quick/1-premium-pill-ui-rework-monochrome-luxury/) |
| 2 | Fix pill rounded corner haziness via set_shadow | 2026-02-28 | 527b0f9 | Awaiting verification | [2-fix-pill-rounded-corner-haziness-via-set](./quick/2-fix-pill-rounded-corner-haziness-via-set/) |
| 3 | Pill UI overhaul: smaller size, vibrant waveform, animated thinking dots, rainbow border | 2026-03-01 | 099f5c2 | Needs Review | [3-pill-ui-overhaul-smaller-size-vibrant-wa](./quick/3-pill-ui-overhaul-smaller-size-vibrant-wa/) |

### Blockers/Concerns

- [Pre-Phase 6]: Win32 WS_EX_NOACTIVATE exact Rust API call needs to be identified from Tauri source or reference projects (Keyless, Voquill) — config alone confirmed broken
- [Pre-Phase 6 RESOLVED]: voice_activity_detector@0.2.1 chosen (not silero-vad-rust) — Silero V5, ort 2.0.0-rc.10, Windows verified, added to Cargo.toml in Plan 05-01
- [Pre-Phase 7]: Code signing certificate (OV vs EV) decision and cost unresolved — budget needed before Phase 7 planning
- [Phase 02-02 RESOLVED]: CUDA 12.9 installed (not 11.7 — MSVC incompatibility; not 13.x — dropped Pascal support)
- [Phase 02-02 RESOLVED]: LIBCLANG_PATH and BINDGEN_EXTRA_CLANG_ARGS set permanently as user env vars

## Session Continuity

Last session: 2026-02-28
Stopped at: Plan 05-02 complete — toggle mode verified end-to-end. Commits: 81c40fe, 648e52a, 725e792. Phase 05 fully complete.
Resume signal: Phase 05 complete (both plans done). Execute Phase 06 (Win32 focus isolation / WS_EX_NOACTIVATE).
Resume file: .planning/phases/ (Phase 06 not yet planned)
