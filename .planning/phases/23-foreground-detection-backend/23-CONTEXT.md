# Phase 23: Foreground Detection Backend - Context

**Gathered:** 2026-03-07
**Status:** Ready for planning

<domain>
## Phase Boundary

Win32 foreground detection module, per-app override data model, settings.json persistence, and Tauri commands. This phase delivers the backend infrastructure only — no UI (Phase 25), no pipeline wiring (Phase 24), no process dropdown (Phase 26).

</domain>

<decisions>
## Implementation Decisions

### UWP Resolution Strategy
- Use EnumChildWindows to find the real child process inside ApplicationFrameHost.exe
- If EnumChildWindows fails or returns the same PID, fall back to "applicationframehost.exe" as the key — user can still create a rule for it
- UWP resolution is best-effort, not a hard requirement for v1.4 launch

### Tauri Command Surface
- `detect_foreground_app` returns a struct with `exe_name: Option<String>` and `window_title: Option<String>` — exe_name is the primary key, window_title is informational for the UI (helps user confirm they detected the right app)
- Individual CRUD commands: `get_app_rules`, `set_app_rule`, `remove_app_rule` — granular commands are simpler to reason about and match the three UI actions (list, add/update, delete)
- `get_app_rules` returns the full map; `set_app_rule` takes an exe name + override struct; `remove_app_rule` takes an exe name

### Persistence Schema
- Flat map in settings.json keyed by lowercase exe name: `"app_rules": {"acad.exe": {"all_caps": true}, "outlook.exe": {"all_caps": false}}`
- Bare exe name only (no path, no version) — stable across updates, matches how users think about apps
- Case-normalized at every boundary (detection, storage, lookup) per prior decision
- `Option<bool>` for `all_caps` in the Rust struct maps to JSON presence: key present = override set, key absent = inherit global default

### Fallback Behavior
- `detect_foreground_app` returns `Ok(DetectedApp { exe_name: None, window_title: None })` when detection fails — not an error, just empty
- Frontend shows "Could not detect app" message when exe_name is None, prompting user to try again
- Elevated processes, lockscreen, and desktop return None gracefully — no crash, no hang
- PROCESS_QUERY_LIMITED_INFORMATION access flag for OpenProcess (per prior decision) — works for most elevated processes without requiring admin privileges

### Claude's Discretion
- Internal module structure (single foreground.rs vs detection/ directory)
- Error logging verbosity for Win32 API failures
- Whether to cache the last detected app or always query fresh
- Serde field naming conventions for the override struct

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Profile` struct (`profiles.rs:12-16`): Has `all_caps: bool`, `filler_removal: bool`, `corrections: HashMap` — per-app overrides layer on top of this
- `ActiveProfile` (`profiles.rs:30`): Mutex-wrapped managed state pattern to follow for `AppOverrides`
- `keyboard_hook.rs`: Demonstrates the `windows` crate unsafe FFI pattern used for Win32 API calls
- `windows` crate v0.58 already in `Cargo.toml` with `Win32_UI_WindowsAndMessaging` and `Win32_System_Threading` features

### Established Patterns
- Managed state via `pub struct FooState(pub std::sync::Mutex<T>)` registered on Builder
- Feature-gated modules with `#[cfg(windows)]` for platform-specific code
- Tauri commands as `#[tauri::command]` async functions in lib.rs
- Settings persisted via frontend store.ts `get<T>(key)` / `set(key, value)` — backend reads JSON directly

### Integration Points
- `lib.rs`: New module declaration (`mod foreground;`), new managed state registration, new command registration
- `Cargo.toml`: May need `Win32_System_Diagnostics_ToolHelp` feature flag for CreateToolhelp32Snapshot (process enumeration for Phase 26)
- `settings.json`: New `app_rules` key alongside existing settings

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. User deferred all gray areas to best-practice recommendations.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 23-foreground-detection-backend*
*Context gathered: 2026-03-07*
