# Phase 16: Rebind and Coexistence - Context

**Gathered:** 2026-03-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Changing the hotkey in settings correctly switches between the hook backend (for modifier-only combos like Ctrl+Win) and tauri-plugin-global-shortcut (for standard combos like Ctrl+Shift+V) at runtime, with no double-firing, and surfaces hook installation failure as a visible status in settings. Frontend capture UI changes belong to Phase 17; hook internals belong to Phase 15.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion

All areas deferred to Claude's best judgment based on standard practices.

**Switchover behavior:**
- Atomic swap: unregister/stop the old backend before starting the new one — never have both active simultaneously
- If a recording session is in progress when the user changes hotkey, finish the current session before switching backends (don't interrupt mid-recording)
- Brief gap (~10ms) between old teardown and new registration is acceptable — user is in settings UI, not actively dictating

**Hook failure UX:**
- Inline warning below the hotkey field in settings: "Hook unavailable — using standard shortcut fallback"
- Auto-assign a fallback standard hotkey (Ctrl+Shift+Space, the v1.1 default) so the user isn't left with no working hotkey
- Log the failure reason for debugging but don't expose technical details in the UI
- If the user later selects a modifier-only combo and the hook is unavailable, show an error explaining that modifier-only combos require the hook and suggest a standard combo instead

**Backend indicator:**
- Hidden by default — users don't need to know which backend is active
- Only surface backend info in the hook-failure warning state (implicit: if no warning, hook is working)
- No "hook active" / "standard shortcut active" indicator in normal operation

**Hotkey format routing:**
- Route based on string parsing of the existing hotkey format — no new field needed
- A modifier-only combo contains only modifier tokens (ctrl, alt, shift, meta/win) with no base key
- "ctrl+win" → modifier-only → route to hook backend
- "ctrl+shift+v" → has base key → route to tauri-plugin-global-shortcut
- Store in settings.json as the same plain string field ("hotkey"), same as today

</decisions>

<specifics>
## Specific Ideas

No specific requirements — user gave Claude full discretion on all areas. Standard practices apply throughout.

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `rebind_hotkey()` (lib.rs:495): Current IPC command — needs branching logic to detect modifier-only format and route to hook vs global-shortcut
- `unregister_hotkey()` / `register_hotkey()` (lib.rs:523-544): Existing IPC commands — need equivalent hook start/stop paths
- `handle_shortcut()` (lib.rs:368): Shared handler body for Pressed/Released — hook backend must produce same event shape
- `read_saved_hotkey()` (lib.rs:141): Reads hotkey on startup — needs to route to correct backend at app launch
- `HotkeyCapture.tsx`: Frontend capture component — Phase 17 modifies this, but Phase 16 must ensure backend routing works for any hotkey string it might receive

### Established Patterns
- `tauri-plugin-global-shortcut` with `ShortcutState::Pressed/Released` events — hook backend must produce equivalent signals
- `Arc<AtomicBool>` for cross-thread state flags — can track hook-active vs standard-active
- `Mutex<Option<T>>` for optional managed state — pattern for hook handle storage
- Settings stored in settings.json via `read_settings()` / `write_settings()` helpers

### Integration Points
- `rebind_hotkey()` IPC command: primary modification target — add routing logic
- `setup()` closure in lib.rs: startup hotkey registration needs routing
- Phase 15's hook module: Phase 16 calls its start/stop functions
- Settings panel (frontend): needs hook status display for failure case
- `keyboard_hook` module from Phase 15: exposes install/uninstall/is_active API

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 16-rebind-and-coexistence*
*Context gathered: 2026-03-02*
