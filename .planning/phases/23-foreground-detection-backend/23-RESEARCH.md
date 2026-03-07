# Phase 23: Foreground Detection Backend - Research

**Researched:** 2026-03-07
**Domain:** Win32 foreground window detection, per-app override data model, Tauri command surface
**Confidence:** HIGH

## Summary

This phase requires a new Rust module that calls three Win32 APIs (`GetForegroundWindow`, `GetWindowThreadProcessId`, `OpenProcess` + `QueryFullProcessImageNameW`) to identify the foreground application, plus a data model for per-app overrides persisted in `settings.json`, plus Tauri commands exposing CRUD operations and detection to the frontend.

All required Win32 APIs are already available through the `windows` crate v0.58 with feature flags already in `Cargo.toml` (`Win32_UI_WindowsAndMessaging`, `Win32_System_Threading`). The existing codebase demonstrates the exact unsafe FFI pattern needed (`keyboard_hook.rs`), the managed state pattern (`ActiveProfile`, `SettingsState`), and the settings persistence pattern (`read_settings`/`write_settings`/`flush_settings` in `lib.rs`).

UWP resolution via `EnumChildWindows` is best-effort per user decision. The callback pattern requires an `unsafe extern "system"` function and `LPARAM` for context passing -- slightly more involved than the basic detection chain but well-documented.

**Primary recommendation:** Single `foreground.rs` module with `detect_foreground_app()` function plus `AppRule`/`AppRulesState` types. CRUD Tauri commands in `lib.rs` following the existing `get_setting`/`set_setting` pattern. No new crate dependencies.

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions
- UWP Resolution: EnumChildWindows to find real child process; fall back to "applicationframehost.exe" if it fails. Best-effort, not a hard requirement.
- Tauri Command Surface: `detect_foreground_app` returns `{exe_name: Option<String>, window_title: Option<String>}`. Individual CRUD: `get_app_rules`, `set_app_rule`, `remove_app_rule`.
- Persistence Schema: Flat map in settings.json keyed by lowercase exe name: `"app_rules": {"acad.exe": {"all_caps": true}}`. Bare exe name only. Case-normalized at every boundary. `Option<bool>` for `all_caps`.
- Fallback Behavior: Returns `Ok(DetectedApp { exe_name: None, window_title: None })` on failure -- not an error. PROCESS_QUERY_LIMITED_INFORMATION for OpenProcess.

### Claude's Discretion
- Internal module structure (single foreground.rs vs detection/ directory)
- Error logging verbosity for Win32 API failures
- Whether to cache the last detected app or always query fresh
- Serde field naming conventions for the override struct

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope.

</user_constraints>

<phase_requirements>

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| DET-01 | App auto-detects the foreground application at text injection time using Win32 APIs | Win32 API chain verified: GetForegroundWindow -> GetWindowThreadProcessId -> OpenProcess -> QueryFullProcessImageNameW. All available via existing `windows` crate features. |
| DET-02 | Detection resolves process executable name (e.g., "acad.exe") | QueryFullProcessImageNameW returns full path; extract filename, lowercase it. UWP apps resolved via EnumChildWindows best-effort. |
| DET-03 | Detection falls back to global defaults when process name cannot be resolved | PROCESS_QUERY_LIMITED_INFORMATION handles most elevated processes. Return `DetectedApp { exe_name: None, window_title: None }` on any failure -- caller falls back to global default. |
| OVR-04 | Per-app rules persist across app restarts via settings.json | Use existing `read_settings`/`write_settings` infrastructure. Store under `"app_rules"` key. Load into `AppRulesState` managed state at startup. |

</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `windows` | 0.58 | Win32 API bindings (GetForegroundWindow, OpenProcess, etc.) | Already in Cargo.toml with required feature flags |
| `serde` / `serde_json` | 1.x | Serialization for AppRule struct and settings.json | Already in Cargo.toml |
| `tauri` | 2.x | Command surface, managed state, app handle for settings | Already in Cargo.toml |

### Supporting
No new dependencies needed. Everything required is already available.

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Raw Win32 OpenProcess + QueryFullProcessImageNameW | `sysinfo` crate | Adds a dependency for something achievable in ~20 lines of unsafe code. The project already uses this pattern in keyboard_hook.rs. Not recommended. |

**Installation:**
No new packages needed. May need to add `Win32_System_Diagnostics_ToolHelp` feature flag to `windows` crate in Cargo.toml for Phase 26 (process enumeration), but Phase 23 does not need it.

## Architecture Patterns

### Recommended Project Structure

Single file approach (Claude's discretion -- recommended over a directory for this scope):

```
src-tauri/src/
  foreground.rs          # NEW: detect_foreground_app(), AppRule, AppRulesState, UWP resolution
  lib.rs                 # MODIFIED: mod foreground, manage(AppRulesState), 4 new commands
```

### Pattern 1: Win32 Detection Chain

**What:** Three-step unsafe Win32 API call sequence to get the foreground process name.
**When to use:** Every call to `detect_foreground_app`.

```rust
// Source: windows crate docs + keyboard_hook.rs pattern
use windows::Win32::Foundation::{HANDLE, HWND, CloseHandle};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_NAME_FORMAT,
};

#[derive(Clone, serde::Serialize)]
pub struct DetectedApp {
    pub exe_name: Option<String>,
    pub window_title: Option<String>,
}

pub fn detect_foreground_app() -> DetectedApp {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return DetectedApp { exe_name: None, window_title: None };
        }

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return DetectedApp { exe_name: None, window_title: None };
        }

        let title = get_window_title(hwnd);
        let exe = get_process_exe_name(pid);

        // UWP resolution: if exe is applicationframehost.exe, try to find child
        let exe = match exe.as_deref() {
            Some("applicationframehost.exe") => resolve_uwp_child(hwnd).or(exe),
            _ => exe,
        };

        DetectedApp { exe_name: exe, window_title: title }
    }
}
```

### Pattern 2: Managed State for App Rules

**What:** Mutex-wrapped HashMap following the ActiveProfile pattern.
**When to use:** Storing and retrieving per-app override rules.

```rust
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Per-app override. Fields are Option to support "inherit global default" semantics.
/// Key present = override set, key absent = inherit.
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct AppRule {
    pub all_caps: Option<bool>,
}

/// Managed state: HashMap keyed by lowercase exe name.
pub struct AppRulesState(pub std::sync::Mutex<HashMap<String, AppRule>>);
```

### Pattern 3: Tauri Commands Following Existing Convention

**What:** CRUD commands matching the existing `get_setting`/`set_setting` pattern.
**When to use:** Frontend interaction with app rules.

```rust
#[tauri::command]
fn get_app_rules(app: tauri::AppHandle) -> Result<HashMap<String, AppRule>, String> {
    let state = app.state::<foreground::AppRulesState>();
    let guard = state.0.lock().map_err(|e| format!("app_rules lock: {}", e))?;
    Ok(guard.clone())
}

#[tauri::command]
fn set_app_rule(app: tauri::AppHandle, exe_name: String, rule: AppRule) -> Result<(), String> {
    let key = exe_name.to_lowercase();
    let state = app.state::<foreground::AppRulesState>();
    let mut guard = state.0.lock().map_err(|e| format!("app_rules lock: {}", e))?;
    guard.insert(key.clone(), rule.clone());
    // Persist to settings.json
    let mut settings = read_settings(&app)?;
    let rules_json = serde_json::to_value(&*guard).map_err(|e| e.to_string())?;
    settings["app_rules"] = rules_json;
    write_settings(&app, &settings)
}

#[tauri::command]
fn remove_app_rule(app: tauri::AppHandle, exe_name: String) -> Result<(), String> {
    let key = exe_name.to_lowercase();
    let state = app.state::<foreground::AppRulesState>();
    let mut guard = state.0.lock().map_err(|e| format!("app_rules lock: {}", e))?;
    guard.remove(&key);
    // Persist
    let mut settings = read_settings(&app)?;
    let rules_json = serde_json::to_value(&*guard).map_err(|e| e.to_string())?;
    settings["app_rules"] = rules_json;
    write_settings(&app, &settings)
}

#[tauri::command]
fn detect_foreground_app() -> foreground::DetectedApp {
    foreground::detect_foreground_app()
}
```

### Pattern 4: UWP Child Window Resolution

**What:** EnumChildWindows callback to find the real process behind ApplicationFrameHost.
**When to use:** When the detected exe is "applicationframehost.exe".

```rust
use windows::Win32::UI::WindowsAndMessaging::{EnumChildWindows, GetWindowThreadProcessId};
use windows::Win32::Foundation::{BOOL, HWND, LPARAM};

unsafe extern "system" fn enum_child_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let result = &mut *(lparam.0 as *mut Option<String>);
    let mut child_pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut child_pid));
    if child_pid != 0 {
        if let Some(name) = get_process_exe_name(child_pid) {
            if name != "applicationframehost.exe" {
                *result = Some(name);
                return BOOL(0); // stop enumerating
            }
        }
    }
    BOOL(1) // continue
}

fn resolve_uwp_child(parent_hwnd: HWND) -> Option<String> {
    let mut result: Option<String> = None;
    unsafe {
        EnumChildWindows(
            Some(parent_hwnd),
            Some(enum_child_proc),
            LPARAM(&mut result as *mut _ as isize),
        );
    }
    result
}
```

### Anti-Patterns to Avoid
- **Using PROCESS_QUERY_INFORMATION instead of PROCESS_QUERY_LIMITED_INFORMATION:** The former requires more privileges and will fail for elevated processes. The limited variant is sufficient for QueryFullProcessImageNameW.
- **Returning Err from detect_foreground_app:** Detection failure is not an error -- it is expected for lockscreen, desktop, elevated processes. Return empty DetectedApp.
- **Storing full paths instead of bare exe names:** Paths change across updates, installs, user profiles. Bare lowercase exe name is the stable identifier.
- **Forgetting CloseHandle:** OpenProcess returns a HANDLE that must be closed. The `windows` crate's HANDLE implements Drop in some versions, but verify -- if not, call `CloseHandle` explicitly.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Settings persistence | Custom file I/O for app rules | Existing `read_settings`/`write_settings`/`flush_settings` | Already handles path resolution, atomic writes, Mutex locking |
| JSON serialization of HashMap | Manual JSON construction | `serde_json::to_value(&hashmap)` | Handles escaping, nesting, type conversion |
| Process name extraction | String parsing of full path | `std::path::Path::new(full_path).file_name()` | Handles edge cases with separators |

## Common Pitfalls

### Pitfall 1: HANDLE Leak from OpenProcess
**What goes wrong:** Every `OpenProcess` call returns a HANDLE that must be closed.
**Why it happens:** Easy to forget when early-returning on error paths.
**How to avoid:** Use a helper function that opens process, queries name, and closes handle in one scope. Or use a wrapper struct with Drop.
**Warning signs:** Handles accumulate in Task Manager over many detections.

### Pitfall 2: Buffer Size for QueryFullProcessImageNameW
**What goes wrong:** The `lpdwsize` parameter is both input (buffer capacity) and output (actual length). If the buffer is too small, the call fails.
**Why it happens:** Passing a too-small buffer or forgetting to initialize the size variable.
**How to avoid:** Use `MAX_PATH` (260) as initial buffer size. Initialize `let mut size: u32 = buf.len() as u32;` before the call.
**Warning signs:** Detection silently returns None for apps with long paths.

### Pitfall 3: Window Title Encoding
**What goes wrong:** `GetWindowTextW` returns UTF-16 wide strings. Incorrect conversion loses non-ASCII characters.
**Why it happens:** Using lossy conversion or wrong buffer size.
**How to avoid:** Use `String::from_utf16_lossy()` on the raw buffer, trimmed to actual length.
**Warning signs:** App names with accented characters or CJK display as garbage.

### Pitfall 4: Race Between Detection and Settings Mutation
**What goes wrong:** Frontend calls `set_app_rule` while backend reads `AppRulesState` for pipeline lookup.
**Why it happens:** Two Mutex lock acquisitions on different threads.
**How to avoid:** The Mutex already serializes access -- this is handled. Just don't hold the lock across await points (not applicable since these are sync commands).
**Warning signs:** None -- the Mutex pattern handles this correctly.

### Pitfall 5: Startup Load Order
**What goes wrong:** `AppRulesState` is registered on Builder with empty HashMap, but `setup()` loads from disk. Frontend IPC could read empty rules before setup completes.
**Why it happens:** Same issue documented for `SettingsState` (lib.rs:1700-1703).
**How to avoid:** Register empty on Builder, populate in setup() before profile loading. This is acceptable -- frontend reads rules lazily (on App Rules page load), not at app startup.
**Warning signs:** Empty rules list on first render, then correct after re-render.

## Code Examples

### Process Name Extraction Helper
```rust
// Source: Win32 API docs, adapted for windows crate v0.58
unsafe fn get_process_exe_name(pid: u32) -> Option<String> {
    let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
    let mut buf = [0u16; 260]; // MAX_PATH
    let mut size = buf.len() as u32;
    let result = QueryFullProcessImageNameW(
        handle,
        PROCESS_NAME_FORMAT(0), // 0 = Win32 format (not native NT path)
        windows::core::PWSTR(buf.as_mut_ptr()),
        &mut size,
    );
    let _ = CloseHandle(handle);
    result.ok()?;
    let full_path = String::from_utf16_lossy(&buf[..size as usize]);
    std::path::Path::new(&full_path)
        .file_name()
        .map(|f| f.to_string_lossy().to_lowercase())
}
```

### Window Title Extraction
```rust
use windows::Win32::UI::WindowsAndMessaging::GetWindowTextW;

unsafe fn get_window_title(hwnd: HWND) -> Option<String> {
    let mut buf = [0u16; 512];
    let len = GetWindowTextW(hwnd, &mut buf);
    if len > 0 {
        Some(String::from_utf16_lossy(&buf[..len as usize]))
    } else {
        None
    }
}
```

### Loading App Rules in setup()
```rust
// In setup() closure, after settings are loaded:
let app_rules: HashMap<String, foreground::AppRule> = {
    let state = app.state::<SettingsState>();
    let guard = state.0.lock().unwrap();
    guard.get("app_rules")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
};
app.manage(foreground::AppRulesState(
    std::sync::Mutex::new(app_rules),
));
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `winapi` crate | `windows` crate (Microsoft-maintained) | 2021+ | Direct Microsoft support, auto-generated from metadata, more idiomatic Rust |
| `PROCESS_QUERY_INFORMATION` | `PROCESS_QUERY_LIMITED_INFORMATION` | Vista+ | Works on elevated processes without admin privileges |

**Deprecated/outdated:**
- `winapi` crate: Still functional but unmaintained. This project already uses `windows` crate.
- `sysinfo` crate for process name lookup: Overkill for single-PID lookup. Pulls in system-wide process enumeration.

## Open Questions

1. **HANDLE Drop behavior in windows crate v0.58**
   - What we know: The `windows` crate's HANDLE type may or may not implement Drop for automatic cleanup. In v0.62+ it does not auto-close.
   - What's unclear: Exact behavior in v0.58.
   - Recommendation: Always call `CloseHandle` explicitly after `OpenProcess`. Safe regardless of Drop behavior.

2. **Caching vs fresh detection**
   - What we know: Detection is called at most on button click (UI) and at injection time (pipeline, Phase 24). Both are infrequent.
   - What's unclear: Whether caching provides any UX benefit.
   - Recommendation: Always query fresh. Three Win32 calls take microseconds. Caching adds complexity (stale data risk) for no measurable benefit.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[cfg(test)]` + `cargo test` |
| Config file | None (standard Cargo test runner) |
| Quick run command | `cd src-tauri && cargo test --lib -- foreground` |
| Full suite command | `cd src-tauri && cargo test --lib` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DET-01 | detect_foreground_app returns exe_name | manual-only | N/A -- requires running desktop with foreground window | N/A |
| DET-02 | Exe name is lowercase, bare filename | unit | `cd src-tauri && cargo test --lib -- foreground::tests::test_exe_name_normalization` | Wave 0 |
| DET-03 | Fallback returns None, not error | unit | `cd src-tauri && cargo test --lib -- foreground::tests::test_fallback_returns_none` | Wave 0 |
| OVR-04 | AppRule serde round-trip, HashMap persistence | unit | `cd src-tauri && cargo test --lib -- foreground::tests::test_app_rule_serde` | Wave 0 |

**Note:** DET-01 (actual foreground detection) is inherently manual -- it requires a running desktop environment with a foreground window. The Win32 API calls cannot be meaningfully unit-tested without mocking the entire Windows API surface, which would test mocks not behavior. Verification: run the app, click "Detect Active App" while another app is focused, confirm correct exe name appears.

### Sampling Rate
- **Per task commit:** `cd src-tauri && cargo test --lib -- foreground`
- **Per wave merge:** `cd src-tauri && cargo test --lib`
- **Phase gate:** Full suite green before verification

### Wave 0 Gaps
- [ ] `src-tauri/src/foreground.rs` -- needs `#[cfg(test)] mod tests` section with serde round-trip and normalization tests
- [ ] Framework install: None needed -- `cargo test` works out of the box

## Sources

### Primary (HIGH confidence)
- [GetForegroundWindow - windows crate docs](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/UI/WindowsAndMessaging/fn.GetForegroundWindow.html) - signature verified
- [QueryFullProcessImageNameW - windows crate docs](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/Threading/fn.QueryFullProcessImageNameW.html) - signature verified
- [OpenProcess - windows crate docs](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/Threading/fn.OpenProcess.html) - signature and access flags verified
- [EnumChildWindows - windows crate docs](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/UI/WindowsAndMessaging/fn.EnumChildWindows.html) - callback signature verified
- Existing codebase: `keyboard_hook.rs` (Win32 FFI pattern), `profiles.rs` (managed state), `lib.rs` (settings persistence, Tauri command registration)

### Secondary (MEDIUM confidence)
- [Tracking active process in Windows with Rust - Hello Code blog](https://hellocode.co/blog/post/tracking-active-process-windows-rust/) - confirms detection chain pattern
- [PROCESS_QUERY_LIMITED_INFORMATION - windows-sys docs](https://docs.rs/windows-sys/latest/windows_sys/Win32/System/Threading/constant.PROCESS_QUERY_LIMITED_INFORMATION.html)

### Tertiary (LOW confidence)
- [AutoHotkey forum on ApplicationFrameHost.exe](https://www.autohotkey.com/boards/viewtopic.php?style=7&t=112906) - confirms UWP child window enumeration approach, but different language

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - all libraries already in Cargo.toml, APIs verified against official docs
- Architecture: HIGH - follows established codebase patterns (keyboard_hook.rs, profiles.rs, settings infrastructure)
- Pitfalls: HIGH - Win32 API pitfalls are well-documented, HANDLE management is standard concern
- UWP resolution: MEDIUM - EnumChildWindows approach is established but the `windows` crate callback pattern in v0.58 specifically needs verification during implementation

**Research date:** 2026-03-07
**Valid until:** 2026-04-07 (stable domain -- Win32 APIs do not change)
