# Phase 1: Foundation - Context

**Gathered:** 2026-02-27
**Status:** Ready for planning

<domain>
## Phase Boundary

A working Tauri 2.0 app that registers a system-wide hotkey, shows in the system tray, and persists its configuration. This is the verified container for all future work — no audio, no transcription, no overlay yet.

</domain>

<decisions>
## Implementation Decisions

### Default hotkey & rebinding
- Default hotkey: Ctrl+Shift+Space
- Rebinding via key capture UI — click a box, press desired combo, it captures it
- Hotkey swap is immediate — old combo unregisters, new one registers instantly, no restart or Apply button needed
- Changed hotkey persists to settings store immediately on capture

### System tray
- Right-click menu: Settings and Quit (two items only for Phase 1)
- Left-click on tray icon does nothing
- Tray icon: microphone silhouette

### Settings window
- Theme toggle in settings: light and dark mode
- Default theme: light
- Theme preference persists across restarts

### App lifecycle
- No auto-start with Windows by default; toggle available in settings
- Closing the settings window minimizes to tray (app keeps running)
- Single instance enforced — second launch focuses the existing instance
- Quit only via tray menu > Quit

### Claude's Discretion
- Settings window layout (single page vs sidebar tabs) — pick what fits the Phase 1 settings count
- Settings window size (fixed vs resizable)
- Tray icon state planning (whether to design for future state-aware icons now or defer)
- Hotkey conflict validation approach
- Startup notification behavior (silent vs brief toast)

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 01-foundation*
*Context gathered: 2026-02-27*
