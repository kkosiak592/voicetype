# Phase 15: Hook Module - Context

**Gathered:** 2026-03-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Install WH_KEYBOARD_LL keyboard hook on a dedicated thread, implement modifier state machine with 50ms debounce and Start menu suppression, wire hold-to-talk end-to-end with the existing pipeline, and shut down cleanly. This phase delivers a working Ctrl+Win activation path — rebind routing (Phase 16), frontend capture UI (Phase 17), and integration testing (Phase 18) are separate phases.

</domain>

<decisions>
## Implementation Decisions

### Modifier key handling
- Any Ctrl (left or right) + Any Win (left or right) qualifies as the combo — Claude's discretion on implementation, user is left-handed but no hard side constraint
- Exact match only: Ctrl+Win with NO other modifiers held (Shift, Alt). Ctrl+Win+Shift does NOT activate. Prevents conflicts with system shortcuts like Ctrl+Win+D (virtual desktops)

### Default hotkey
- Ctrl+Win becomes the new default hotkey for fresh v1.2 installs, replacing Ctrl+Shift+Space
- Existing v1.1 users keep their saved hotkey on upgrade

### Claude's Discretion
- Left/right modifier distinction (any combination is acceptable)
- Extra keys pressed during active recording: ignore or cancel (pick based on hold-to-talk UX patterns)
- Release behavior: either key ends recording vs both must release (pick based on natural feel)
- Hook auto-install timing: always on startup vs only when modifier-only hotkey is configured
- Upgrade notification for existing users: silent, tray notification, or in-app banner
- Start menu suppression: release order handling, behavior on failed/short activation, Win+other shortcuts when Ctrl not held, behavior when app is paused
- Hook status indication: silent when working vs tray tooltip mention
- Hook failure behavior: silent fallback vs notify-and-fallback
- Settings panel backend indicator: show or hide
- Mid-session hook removal recovery: auto-reinstall, notify, or defer to v2

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. User gave Claude wide discretion on implementation details. The two locked decisions are:
1. Exact modifier match (no activation with extra modifiers held)
2. Ctrl+Win as the new default for fresh installs

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `handle_shortcut()` (lib.rs:368): Existing Pressed/Released handler — hook must bridge to this via mpsc channel and dispatcher thread
- `PipelineState` (pipeline.rs): AtomicU8 state machine (Idle/Recording/Processing) with CAS transitions — already handles concurrent recording prevention
- `rebind_hotkey()` / `unregister_hotkey()` / `register_hotkey()` (lib.rs:495-540): Existing IPC commands for hotkey management — Phase 16 will modify these for routing
- `read_saved_hotkey()` (lib.rs:141): Reads hotkey from settings.json — needs to handle new "ctrl+win" format
- `tray::set_tray_state()` / tray tooltip: Existing tray icon state management (Idle/Recording/Processing)
- `pill::show_pill()` / pill events: Existing overlay UI for recording feedback

### Established Patterns
- `tauri-plugin-global-shortcut` with `ShortcutState::Pressed/Released` events — hook must produce equivalent events
- `Arc<AtomicBool>` for cross-thread state (e.g., `LevelStreamActive`) — same pattern for hook state flags
- `Mutex<Option<T>>` for optional managed state (e.g., `AudioCaptureMutex`, `WhisperStateMutex`)
- `tauri::async_runtime::spawn` for async pipeline execution from sync handlers
- `windows` v0.58 crate already in Cargo.toml — needs 3 new feature flags: `Win32_Foundation`, `Win32_UI_WindowsAndMessaging`, `Win32_UI_Input_KeyboardAndMouse`

### Integration Points
- Tauri builder in `lib.rs` setup: add `device_event_filter(tauri::DeviceEventFilter::Always)` before `.build()`
- Hook thread spawned in `setup()` closure, conditional on saved hotkey format
- `mod keyboard_hook` added to lib.rs module declarations
- Hook shutdown called from app exit/cleanup path
- Default hotkey string changed from "ctrl+shift+space" to "ctrl+win" at lib.rs:1430

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 15-hook-module*
*Context gathered: 2026-03-02*
