# Phase 6: Vocabulary + Settings - Context

**Gathered:** 2026-02-28
**Status:** Ready for planning

<domain>
## Phase Boundary

Word correction dictionary, vocabulary profiles with Structural Engineering and General presets, and a full settings panel with sidebar navigation — the differentiating layer that makes VoiceType accurate for structural engineering work. Custom profile creation is deferred to v2.

</domain>

<decisions>
## Implementation Decisions

### Correction Dictionary Editor
- Inline two-column table (From / To) directly in the settings panel
- Add and delete rows in-place
- Whole word matching only — no substring matching (prevents "mpa" → "MPa" from corrupting "compare")
- Case-insensitive matching — handles whisper's variable capitalization; replacement uses exact casing from the "To" column
- Multi-word phrases supported on both From and To sides — critical for engineering terms ("why section" → "W-section", "aci three eighteen" → "ACI 318", "pounds per square inch" → "PSI")

### Profile System
- Two shipped profiles only for v1: Structural Engineering and General
- Custom profile creation deferred to v2
- Profile switching takes effect immediately — next dictation uses the new profile's initial_prompt, corrections, and formatting
- Each profile has its own separate corrections dictionary — no shared global dictionary
- Each profile bundles: whisper initial_prompt, corrections dictionary, ALL CAPS output flag

### Settings Panel Layout
- Sidebar navigation instead of single scrollable column
- No scrolling within content area — each section fits its view
- Settings window size increased from current dimensions to accommodate sidebar + content
- Section grouping: Claude's discretion

### Model Selection
- Curated list of 3 known models: large-v3-turbo (GPU, best accuracy), medium (balanced), small.en (CPU/fast)
- Each model shows name + description + recommended badge based on detected GPU hardware
- If a model file isn't downloaded yet, show it greyed out with download hint
- Model reload timing on selection change: Claude's discretion

### Microphone Selection
- Dropdown listing available input devices by OS-reported name
- Default option = system default microphone
- Immediate switch — audio stream restarts with selected device on change, no app restart needed
- Selected device persists across restarts

### Claude's Discretion
- Profile switcher UI component design
- Settings sidebar section grouping and naming
- Settings window dimensions
- Model reload behavior (immediate vs restart)
- Loading/transition states during device and model switches
- Dictionary table styling and empty state

</decisions>

<specifics>
## Specific Ideas

- Use the `/frontend-design` skill for all UI/UX work — settings panel layout, sidebar navigation, profile switcher, dictionary editor, model selector, microphone dropdown
- Structural Engineering profile initial_prompt should bias whisper toward: I-beam, W-section, MPa, rebar, AISC, ACI 318, kips, PSI, prestressed, rebar, shear, moment, deflection
- ALL CAPS mode is per-profile — enables engineering drawing annotation and PDF markup workflows

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `RecordingModeToggle.tsx`: Radio-card pattern with indigo selection state — reusable for profile switcher
- `HotkeyCapture.tsx`: Interactive capture box pattern with error state handling
- `ThemeToggle.tsx`, `AutostartToggle.tsx`: Toggle switch patterns
- `store.ts`: `tauri-plugin-store` wrapper with defaults and auto-save — extend for new settings

### Established Patterns
- Dual storage: Rust reads `settings.json` directly (`read_saved_hotkey`, `read_saved_mode`), frontend uses `tauri-plugin-store` via `getStore()`
- Tauri commands for backend state sync: `set_recording_mode` / `get_recording_mode` pattern
- Settings UI: section header (uppercase label) + description + component, Tailwind CSS, dark mode support
- Pipeline state machine: AtomicU8 with CAS transitions in `pipeline.rs`

### Integration Points
- `pipeline.rs:142-150`: Corrections engine inserts between text formatting (trim + trailing space) and text injection — apply corrections to `trimmed` before building `to_inject`
- `transcribe.rs:145-156`: `FullParams` setup — add `params.set_initial_prompt()` per active profile
- `audio.rs`: `start_persistent_stream()` currently uses default device — needs refactor to accept device selection
- `lib.rs:481-492`: `invoke_handler` — register new Tauri commands for profiles, corrections, model/mic selection
- `lib.rs:493-717`: `setup()` — load active profile, corrections dictionary, and selected mic/model at startup
- `tray.rs`: System tray context menu — "Settings" item already opens the settings window

</code_context>

<deferred>
## Deferred Ideas

- Custom profile creation/deletion — v2 feature (PROF-01)
- Per-app profile auto-switching — v2 feature (PROF-02)
- Regex-based corrections for phonetic patterns — v2 feature (ECOR-01)
- Quick-add to dictionary from system tray — v2 feature (ECOR-03)

</deferred>

---

*Phase: 06-vocabulary-settings*
*Context gathered: 2026-02-28*
