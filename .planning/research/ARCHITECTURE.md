# Architecture Research

**Domain:** Per-app settings with foreground window detection for Tauri 2.0 voice-to-text app
**Researched:** 2026-03-07
**Confidence:** HIGH

## System Overview

```
                          EXISTING                              NEW
                     +-----------------+                 +-------------------+
                     |   Frontend      |                 |  AppRulesSection  |
                     |   (React/TS)    |                 |  (new sidebar pg) |
                     +--------+--------+                 +---------+---------+
                              |                                    |
                     Tauri IPC (invoke)                   Tauri IPC (invoke)
                              |                                    |
                     +--------v--------+                 +---------v---------+
                     |   lib.rs        |                 | New commands:      |
                     |   Tauri cmds    |                 | get_app_rules      |
                     |                 |                 | set_app_rule       |
                     +--------+--------+                 | remove_app_rule    |
                              |                          | detect_foreground  |
                     +--------v--------+                 | list_running_apps  |
                     | pipeline.rs     |                 +---------+---------+
                     | run_pipeline()  |                           |
                     |                 |                 +---------v---------+
                     | Step 5: ALL CAPS| <-- MODIFY -->  | app_rules.rs (NEW) |
                     | reads ActiveProf|                 | AppOverrides state |
                     +--------+--------+                 | foreground detect  |
                              |                          +-------------------+
                     +--------v--------+
                     | inject.rs       |
                     | inject_text()   |
                     +--------+--------+
                              |
                     clipboard paste
                     into foreground app
```

### Component Responsibilities

| Component | Responsibility | Status |
|-----------|----------------|--------|
| `app_rules.rs` (NEW) | AppOverride types, AppRulesState, foreground window detection, process enumeration | New file |
| `pipeline.rs` lines 395-404 | ALL CAPS decision point -- currently reads `ActiveProfile.all_caps` | Modify: add override lookup before ALL CAPS decision |
| `profiles.rs` | Global `ActiveProfile` with `all_caps`, `filler_removal`, `corrections` | Unchanged (serves as global default) |
| `lib.rs` | Tauri command registration, managed state, settings persistence | Add new commands + `AppRulesState` managed state |
| `Sidebar.tsx` | Navigation -- `SectionId` union type, `ITEMS` array | Add `'app-rules'` entry |
| `App.tsx` | Section routing via `activeSection` state | Add `AppRulesSection` render case |
| `AppRulesSection.tsx` (NEW) | UI for managing per-app overrides | New component |
| `settings.json` | Persistence -- flat keys (`all_caps`, `corrections.default`) | Add `app_rules` key |

## Integration Architecture

### Where Foreground Detection Hooks In

The detection point is **pipeline.rs line 395**, immediately before the ALL CAPS decision. This is the correct location because:

1. Transcription is complete (text is final after filler removal + corrections)
2. The foreground window at this moment is the window that will receive the paste
3. `inject_text()` is called on the next line -- same foreground window context

**Do NOT detect in inject.rs.** inject.rs is a pure text injection function (clipboard + Ctrl+V). Mixing detection logic there violates its single responsibility and would require inject to return metadata about what it detected.

**Do NOT detect earlier in the pipeline.** The user may switch windows during recording/transcription. The only reliable moment is right before injection.

### AppOverrides State Structure

```rust
// app_rules.rs (NEW)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Per-application setting overrides.
/// Each field is Option -- None means "use global default."
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AppOverride {
    pub all_caps: Option<bool>,
    // Future: filler_removal, corrections profile, etc.
}

/// Map from process name (lowercase, e.g. "bluebeam.exe") to overrides.
/// Process name is the match key -- not window title (titles change).
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct AppRules(pub HashMap<String, AppOverride>);

/// Tauri managed state.
pub struct AppRulesState(pub std::sync::Mutex<AppRules>);
```

**Design decisions:**

- **Process name as key (not window title, not exe path).** Window titles are dynamic and locale-dependent. Full paths break on reinstall or user directory differences. Process name (`bluebeam.exe`, `code.exe`) is stable and human-readable. Stored lowercase for case-insensitive matching.
- **`Option<bool>` not `bool` for override fields.** `None` = inherit global default. `Some(true)` = force on. `Some(false)` = force off. This three-state model avoids the "is false an override or a default?" ambiguity. Critical for UX: users need to distinguish "I explicitly set this off" from "I haven't configured this."
- **`HashMap` not `Vec`.** O(1) lookup by process name at injection time. The number of rules will be small (5-20 apps), so HashMap overhead is negligible but the API is cleaner.
- **Separate `AppRulesState` from `ActiveProfile`.** ActiveProfile holds the global defaults. AppRulesState holds per-app overrides. They are independent managed states -- no mutation of globals during override resolution.

### Data Flow: Detection to Override Application

```
pipeline.rs run_pipeline() -- after corrections, before injection:

1. Get global all_caps from ActiveProfile (existing code)
   |
   v
2. Call app_rules::get_foreground_process_name()
   |  Win32: GetForegroundWindow -> GetWindowThreadProcessId ->
   |  OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION) ->
   |  QueryFullProcessImageNameW -> extract filename -> lowercase
   |  Returns Option<String> (None if detection fails)
   |
   v
3. Look up process name in AppRulesState
   |  let rules = app.state::<AppRulesState>();
   |  let guard = rules.0.lock();
   |  let maybe_override = guard.0.get(&process_name);
   |
   v
4. Resolve effective all_caps
   |  effective = override.all_caps.unwrap_or(global_all_caps)
   |  Falls back to global if: no rule exists, or rule exists but all_caps is None
   |
   v
5. Apply formatting (existing to_uppercase logic, unchanged)
   |
   v
6. inject_text() -- unchanged
```

### Pipeline.rs Modification

The current code at lines 395-404:

```rust
// CURRENT: reads global flag only
let formatted = {
    let profile = app.state::<crate::profiles::ActiveProfile>();
    let guard = profile.0.lock().unwrap_or_else(|e| e.into_inner());
    if guard.all_caps {
        corrected.to_uppercase()
    } else {
        corrected
    }
};
```

Becomes:

```rust
// NEW: check per-app override, fall back to global
let formatted = {
    let global_all_caps = {
        let profile = app.state::<crate::profiles::ActiveProfile>();
        let guard = profile.0.lock().unwrap_or_else(|e| e.into_inner());
        guard.all_caps
    };

    let effective_all_caps = {
        #[cfg(windows)]
        {
            match crate::app_rules::get_foreground_process_name() {
                Some(ref proc_name) => {
                    let rules = app.state::<crate::app_rules::AppRulesState>();
                    let guard = rules.0.lock().unwrap_or_else(|e| e.into_inner());
                    guard.0.get(proc_name)
                        .and_then(|r| r.all_caps)
                        .unwrap_or(global_all_caps)
                }
                None => global_all_caps,
            }
        }
        #[cfg(not(windows))]
        { global_all_caps }
    };

    if effective_all_caps {
        corrected.to_uppercase()
    } else {
        corrected
    }
};
```

Key points:
- Global lock is acquired and dropped before app_rules lock -- no nested locks, no deadlock risk
- `cfg(windows)` guard keeps non-Windows builds compiling (Tauri cross-platform future-proofing)
- Detection failure (`None`) silently falls back to global -- no error logging needed (desktop may be focused, or an inaccessible system process)

## Recommended Project Structure

### New Rust Files

```
src-tauri/src/
    app_rules.rs          # NEW: types, state, foreground detection, process enumeration
```

### New Frontend Files

```
src/
    components/
        sections/
            AppRulesSection.tsx    # NEW: main section component
```

### Structure Rationale

- **Single `app_rules.rs`** instead of splitting detection + state. The module is small (~100-150 lines total). Foreground detection is ~20 lines, process enumeration ~30 lines, state types ~30 lines. Splitting creates file navigation overhead with no modularity benefit.
- **Single `AppRulesSection.tsx`** for the first iteration. The detect button, app list, and add-app UI are small enough to colocate. Extract sub-components only if the file exceeds ~300 lines.

## Architectural Patterns

### Pattern 1: Override Resolution Chain

**What:** Settings resolve through a two-level chain: per-app override -> global default. Each link is optional; first non-None value wins.
**When to use:** Any setting that may have per-app overrides (all_caps now, potentially filler_removal or corrections profile later).
**Trade-offs:** Simple, predictable. If a third level is ever needed (per-profile per-app), the `unwrap_or` chain extends naturally. Avoids overengineering a priority/inheritance system.

```rust
fn resolve_setting<T: Copy>(
    app_override: Option<&AppOverride>,
    field: impl Fn(&AppOverride) -> Option<T>,
    global: T,
) -> T {
    app_override
        .and_then(|r| field(r))
        .unwrap_or(global)
}
```

### Pattern 2: Detect-at-Injection-Time (Not Polling)

**What:** Query the foreground window exactly once, right before injection. No background threads, no caching, no event subscriptions.
**When to use:** Always. The relevant foreground window is the one that will receive the paste.
**Trade-offs:** Adds ~0.1-1ms to the injection path (Win32 API calls are very fast). Zero CPU overhead at all other times. No stale-cache bugs on alt-tab.

### Pattern 3: Process Name Matching

**What:** Match rules by process executable name (e.g., `bluebeam.exe`), not window title or class name.
**When to use:** Always for app identification.
**Trade-offs:** Cannot distinguish multiple windows of the same app (e.g., two Chrome profiles). This is the correct trade-off -- per-window rules are fragile and confusing for users.

## Win32 API Integration

### Required Cargo.toml Feature Additions

The `windows` crate v0.58 is already a dependency. Two features need adding:

```toml
windows = { version = "0.58", features = [
    "Win32_Graphics_Dxgi",
    "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging",
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_System_Threading",
    # NEW for per-app settings:
    "Win32_System_ProcessStatus",
] }
```

Note: `PROCESS_QUERY_LIMITED_INFORMATION` and `QueryFullProcessImageNameW` are in `Win32_System_Threading`, which is already enabled. `Win32_System_ProcessStatus` provides `EnumProcesses` for the running-app list. Verify at implementation time whether `PROCESS_NAME_WIN32` requires additional features -- it may already be covered by `Win32_System_Threading`.

### Foreground Detection Implementation

```rust
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
use windows::Win32::System::Threading::*;

pub fn get_foreground_process_name() -> Option<String> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 260]; // MAX_PATH
        let mut len = buf.len() as u32;
        let result = QueryFullProcessImageNameW(
            process, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len
        );
        let _ = CloseHandle(process);
        result.ok()?;
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        path.rsplit('\\').next().map(|s| s.to_lowercase())
    }
}
```

This follows the same pattern as `keyboard_hook.rs` -- unsafe Win32 calls, same `windows` crate, same import style.

### Running Process Enumeration

For the searchable dropdown, enumerate windowed processes:

```rust
pub fn list_running_process_names() -> Vec<String> {
    // Use EnumProcesses to get all PIDs
    // For each: OpenProcess -> QueryFullProcessImageNameW
    // Filter to processes with visible windows (EnumWindows + IsWindowVisible)
    // Deduplicate by name, sort alphabetically
    // Return vec of lowercase process names
}
```

## New Tauri Commands

| Command | Signature | Purpose |
|---------|-----------|---------|
| `get_app_rules` | `() -> Result<HashMap<String, AppOverride>>` | Load all rules for UI display |
| `set_app_rule` | `(process_name: String, all_caps: Option<bool>) -> Result<()>` | Add/update a rule + persist to settings.json |
| `remove_app_rule` | `(process_name: String) -> Result<()>` | Delete a rule + persist |
| `detect_foreground_app` | `() -> Result<Option<String>>` | One-shot foreground detection for "Detect Active App" button |
| `list_running_apps` | `() -> Result<Vec<String>>` | Enumerate windowed processes for searchable dropdown |

### Settings Persistence

Rules persist to `settings.json` under key `app_rules`:

```json
{
    "all_caps": true,
    "app_rules": {
        "bluebeam.exe": { "all_caps": true },
        "code.exe": { "all_caps": false },
        "outlook.exe": { "all_caps": null }
    }
}
```

Follows existing flat-key convention. The `app_rules` key holds the full map -- loaded at startup into `AppRulesState`, flushed via existing `write_settings()` after every mutation.

### lib.rs Registration

```rust
// In setup():
let app_rules = load_app_rules_from_settings(&json);
app.manage(app_rules::AppRulesState(std::sync::Mutex::new(app_rules)));

// In invoke_handler:
builder.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    get_app_rules,
    set_app_rule,
    remove_app_rule,
    detect_foreground_app,
    list_running_apps,
]);
```

## Frontend Integration

### Sidebar Changes

`Sidebar.tsx`:
- Extend `SectionId` union: `'general' | 'dictionary' | 'model' | 'appearance' | 'system' | 'history' | 'app-rules'`
- Add to `ITEMS` array: `{ id: 'app-rules', label: 'App Rules', icon: AppWindow }` (from `lucide-react`)
- Position after General, before Dictionary (these are both "behavior" settings)

`App.tsx`:
- Import `AppRulesSection`
- Add render case in section switch

### AppRulesSection UI Flow

**Add App via Detection:**
1. User clicks "Detect Active App"
2. Show 3-second countdown overlay ("Switch to target app in 3... 2... 1...")
3. After countdown: `invoke('detect_foreground_app')`
4. If detected: pre-fill process name, show confirmation to add rule
5. If VoiceType detected (user didn't switch): show hint to switch first

The countdown is essential -- without it, the detected app is always VoiceType itself since the settings window has focus when the button is clicked.

**Add App via Dropdown:**
1. User clicks "+ Add App"
2. Call `invoke('list_running_apps')` to populate searchable dropdown
3. User selects a process name
4. Add rule with `all_caps: null` (inherit global default)

**Three-State Toggle:**
Each app rule's ALL CAPS toggle cycles through: Default (inherit) -> On -> Off -> Default. This maps to `Option<bool>`: `None` -> `Some(true)` -> `Some(false)` -> `None`.

## Anti-Patterns

### Anti-Pattern 1: Polling Foreground Window

**What people do:** Background thread polling GetForegroundWindow every 100ms, caching current app.
**Why it's wrong:** Unnecessary CPU, stale cache on fast alt-tab, complexity. Detection is needed exactly once per injection.
**Do this instead:** Single-shot detection at injection time in pipeline.rs.

### Anti-Pattern 2: Window Title Matching

**What people do:** Match rules by window title ("Document1 - Bluebeam Revu").
**Why it's wrong:** Titles change per document, are locale-dependent, break with app updates.
**Do this instead:** Match by process name (`bluebeam.exe`).

### Anti-Pattern 3: Mutating ActiveProfile for Overrides

**What people do:** Swap `ActiveProfile.all_caps` when foreground app changes, swap back after injection.
**Why it's wrong:** Race condition if two pipeline runs overlap. ActiveProfile becomes non-deterministic. Other code reading ActiveProfile gets wrong values during the swap window.
**Do this instead:** Keep ActiveProfile as immutable global default. Resolve overrides at read time in pipeline.rs, never mutating global state.

### Anti-Pattern 4: Separate Settings File

**What people do:** Create `app_rules.json` alongside `settings.json`.
**Why it's wrong:** Two files to load, flush, synchronize. Two Mutex states or one state spanning two files. Introduces consistency bugs.
**Do this instead:** Add `app_rules` key to existing `settings.json`.

### Anti-Pattern 5: Detecting in inject.rs

**What people do:** Put foreground detection inside `inject_text()` and return the detected app name alongside the result.
**Why it's wrong:** `inject_text()` is called via `spawn_blocking` -- it runs on a blocking thread, not the main thread. Win32 window queries work on any thread, but the function's responsibility is text injection, not app detection. Also, formatting decisions (ALL CAPS) happen before `inject_text()` is called, so detection there would be too late.
**Do this instead:** Detect in pipeline.rs before the formatting step.

## Build Order (Dependency-Ordered)

| Step | What | Depends On | Files Modified/Created |
|------|------|------------|----------------------|
| 1 | `app_rules.rs`: types (`AppOverride`, `AppRules`, `AppRulesState`) | Nothing | NEW: `app_rules.rs` |
| 2 | `app_rules.rs`: `get_foreground_process_name()` | Step 1 + Cargo.toml features | `app_rules.rs`, `Cargo.toml` |
| 3 | lib.rs: register `AppRulesState`, load from settings at startup | Steps 1-2 | `lib.rs` |
| 4 | Tauri commands: `detect_foreground_app`, `list_running_apps` | Steps 1-3 | `lib.rs` |
| 5 | Pipeline integration: override resolution at line 395 | Steps 1-3 | `pipeline.rs` |
| 6 | Tauri commands: `get_app_rules`, `set_app_rule`, `remove_app_rule` | Steps 1-3 | `lib.rs` |
| 7 | Sidebar + routing: add `'app-rules'` section | Nothing (frontend only) | `Sidebar.tsx`, `App.tsx` |
| 8 | `AppRulesSection.tsx`: UI with detect button, app list, three-state toggles | Steps 4-7 | NEW: `AppRulesSection.tsx` |

Steps 1-5 form the functional backend. Step 5 is the critical integration point where per-app overrides actually take effect. Steps 6-8 are the management UI.

Steps 1-3 can be built and tested via unit tests + logging before any UI exists. Step 5 can be tested by manually adding rules to settings.json and verifying override behavior.

## Sources

- Existing codebase: `pipeline.rs` (lines 395-404), `profiles.rs`, `inject.rs`, `keyboard_hook.rs` (Win32 patterns), `lib.rs` (managed state + command registration), `Sidebar.tsx`, `Cargo.toml`
- Win32 API: `GetForegroundWindow`, `GetWindowThreadProcessId`, `OpenProcess`, `QueryFullProcessImageNameW` -- standard Win32 process identification, verified available in `windows` crate v0.58
- `windows` crate features verified in existing `Cargo.toml` (line 103-109)

---
*Architecture research for: VoiceType v1.4 Per-App Settings*
*Researched: 2026-03-07*
