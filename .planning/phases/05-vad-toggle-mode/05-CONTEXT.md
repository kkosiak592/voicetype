# Phase 5: VAD + Toggle Mode - Context

**Gathered:** 2026-02-28
**Status:** Ready for planning

<domain>
## Phase Boundary

Silero VAD silence detection enables toggle mode — tap to start, auto-stop on silence — and gates whisper against hallucination on empty audio buffers. Hold-to-talk mode continues to work with VAD gating applied. No new UI windows or settings panel (that's Phase 6).

</domain>

<decisions>
## Implementation Decisions

### Silence detection tuning
- ~1.5 second silence threshold before auto-stop in toggle mode
- Fixed threshold, not user-adjustable (no sensitivity slider)

### Mode switching UX
- Settings panel toggle (radio button or switch) to choose hold-to-talk vs toggle mode
- Same hotkey for both modes — behavior changes based on selected mode
- Hold-to-talk is the default mode for new installs
- Second tap in toggle mode = instant hard stop, goes straight to transcription (no grace period)

### Pill feedback in toggle mode
- Same pill visuals for both modes — no mode indicator or badge
- No visual hint before auto-stop — silence detected, immediately transition to processing
- Pill only appears during active recording, not while idle in toggle mode

### Speech gate strictness
- VAD replaces the current crude 100ms/1600-sample minimum gate entirely
- VAD speech gate applies to both hold-to-talk and toggle modes (prevents whisper hallucination in either)
- ~300ms minimum detected speech required before buffer is sent to whisper — below that, discard (coughs, clicks, breaths)

### Claude's Discretion
- Maximum recording duration safety cap in toggle mode (prevent runaway recordings)
- VAD sensitivity/noise handling approach (use Silero defaults or tune thresholds)
- Silero VAD integration details (ONNX runtime, chunk processing strategy)

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches.

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `PipelineState` (pipeline.rs): AtomicU8 state machine with IDLE/RECORDING/PROCESSING — will need new states or toggle-aware transitions
- `AudioCapture` (audio.rs): Persistent cpal stream with `recording` AtomicBool flag and 16kHz resampled buffer — VAD hooks into this stream
- `run_pipeline()` (pipeline.rs): Orchestrates stop → gate → whisper → inject → idle — VAD gate replaces the existing sample-count gate
- Settings persistence via `settings.json` in app data dir — mode selection follows same pattern as hotkey storage

### Established Patterns
- Hotkey handler in lib.rs uses `ShortcutState::Pressed`/`Released` events — toggle mode needs tap detection instead of hold detection
- `pill-show`/`pill-state`/`pill-hide` event system for pill state transitions — same events work for toggle mode
- `tray::set_tray_state()` for tray icon state — same Recording/Processing/Idle states apply
- `tauri::async_runtime::spawn` for async pipeline execution after hotkey release

### Integration Points
- Hotkey handler in `lib.rs` (both `setup()` and `rebind_hotkey()`) — needs mode-aware branching
- `run_pipeline()` in pipeline.rs — VAD gate replaces line 55-65 sample-count check
- Audio callback in `audio.rs` — VAD needs access to audio chunks (30ms at 16kHz = 480 samples)
- Settings read/write for `recording_mode` field alongside existing `hotkey` field

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 05-vad-toggle-mode*
*Context gathered: 2026-02-28*
