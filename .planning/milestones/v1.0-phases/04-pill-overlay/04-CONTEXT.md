# Phase 4: Pill Overlay - Context

**Gathered:** 2026-02-28
**Status:** Ready for planning

<domain>
## Phase Boundary

A floating transparent pill window that shows recording state and audio levels during dictation, without ever stealing focus from the application being dictated into. The pill appears during recording/processing and hides when idle. No new capabilities (VAD, toggle mode, settings UI) — purely visual feedback for the existing hold-to-talk pipeline.

</domain>

<decisions>
## Implementation Decisions

### Pill appearance
- Compact size (~120x40px)
- Always dark solid background — does not follow light/dark theme setting
- Fully rounded pill/capsule shape (semicircular ends)
- Heavily frosted opacity (~70-80%) — mostly opaque, slight see-through

### Visualizer design
- Frequency bars style — classic vertical equalizer bars
- ~15 bars to fill the compact pill width
- Bar color: Claude's discretion (pick what complements dark pill)
- Animation: follow best practice/standard for audio visualizers (smooth interpolation typical)

### Screen position & behavior
- Default position: bottom center of screen
- Draggable: user can drag the pill anywhere on screen
- Position persistence: remember last drag position across sessions (tauri-plugin-store)
- Show/hide transition: fade in/out (smooth opacity transition)
- Hidden when idle — pill only visible during recording and processing states

### State display
- **Recording**: frequency bars animate with mic input + small red recording dot indicator
- **Processing**: wavy/animated pill border effect — modern, fluid border animation while whisper transcribes (bars go static or disappear)
- **Completion**: brief success flash (~300ms color flash or checkmark) then fade out
- **Error**: brief error flash (red/orange ~500ms) for no-speech-detected or whisper failures, then fade out

### Claude's Discretion
- Exact bar color choice for frequency visualizer
- Animation easing/timing curves
- Specific implementation of the wavy border processing effect (CSS animation, canvas, SVG — whichever achieves the modern fluid look)
- Opacity level fine-tuning within the 70-80% range
- Exact flash duration and color for success/error states
- Spacing and padding inside the pill

</decisions>

<specifics>
## Specific Ideas

- Processing state should feel "super modern" — user specifically wants the pill border/outline to animate in a wavy or fluid way during whisper processing, not a standard spinner
- The pill is a passive indicator only — no interactive elements, no buttons, no click actions beyond drag-to-reposition

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `pipeline::PipelineState` (AtomicU8): IDLE/RECORDING/PROCESSING state machine — pill mirrors these states
- `tray::TrayState` enum + `set_tray_state()`: existing pattern for updating visual state on pipeline transitions — pill state updates can follow the same pattern
- `audio::AudioCapture.buffer` (Arc<Mutex<Vec<f32>>>): 16kHz mono audio buffer — RMS/level data can be computed from recent samples for visualizer
- `tauri-plugin-store`: already used for settings persistence — reuse for pill position storage

### Established Patterns
- State transitions happen in hotkey handler (lib.rs:328-349) and `run_pipeline()` — pill show/hide/state events should be emitted from these same locations
- Tauri events (`app.emit()`) for frontend communication — use for streaming audio levels and state changes to the pill React component
- React + Tailwind CSS for all frontend — pill UI will use the same stack

### Integration Points
- `tauri.conf.json`: needs a second window definition for the pill (separate from `settings`)
- Win32 `WS_EX_NOACTIVATE` extended window style: must be set on the pill window to prevent focus steal — requires Rust-side window manipulation after Tauri creates the window
- Hotkey handler in `lib.rs`: emit pill-show/pill-hide events alongside existing tray state changes
- `pipeline::run_pipeline()`: emit processing-complete and error events for pill success/error flash states
- `audio.rs` callback: needs a parallel channel to stream RMS levels to the pill at ~30-60fps for visualizer

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 04-pill-overlay*
*Context gathered: 2026-02-28*
