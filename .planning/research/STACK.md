# Stack Research: Per-App Foreground Window Detection

**Domain:** Win32 foreground window detection + process enumeration for per-app settings
**Researched:** 2026-03-07
**Confidence:** HIGH

## Verdict: No New Crates Required

Everything needed is already available through the existing `windows` crate v0.58 dependency. The required feature flags (`Win32_UI_WindowsAndMessaging`, `Win32_System_Threading`, `Win32_Foundation`) are already enabled in Cargo.toml. Process enumeration for the searchable dropdown uses the same Win32 APIs. No `sysinfo` crate needed.

The frontend searchable dropdown should be built as a custom component -- the app already uses zero UI libraries beyond React + Tailwind, and the interaction is simple enough to not justify adding one.

## Backend: Foreground Window Detection

### APIs Already Available (No Changes to Cargo.toml)

All three APIs live in modules whose feature flags are already enabled:

| API | Module | Feature Flag | Status |
|-----|--------|-------------|--------|
| `GetForegroundWindow()` | `Win32::UI::WindowsAndMessaging` | `Win32_UI_WindowsAndMessaging` | Already enabled |
| `GetWindowThreadProcessId()` | `Win32::UI::WindowsAndMessaging` | `Win32_UI_WindowsAndMessaging` | Already enabled |
| `OpenProcess()` | `Win32::System::Threading` | `Win32_System_Threading` | Already enabled |
| `QueryFullProcessImageNameW()` | `Win32::System::Threading` | `Win32_System_Threading` | Already enabled |
| `CloseHandle()` | `Win32::Foundation` | `Win32_Foundation` | Already enabled |
| `PROCESS_QUERY_LIMITED_INFORMATION` | `Win32::System::Threading` | `Win32_System_Threading` | Already enabled |
| `PROCESS_NAME_FORMAT` | `Win32::System::Threading` | `Win32_System_Threading` | Already enabled |

### Detection Pattern

The full chain to resolve foreground process name at injection time:

```rust
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW,
    PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowThreadProcessId,
};

/// Get the executable name (e.g., "Code.exe") of the foreground window.
/// Returns None if any step fails (no foreground window, access denied, etc.).
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

        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 260]; // MAX_PATH
        let mut size = buf.len() as u32;
        let result = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut size,
        );
        let _ = CloseHandle(handle);

        result.ok()?;
        let path = String::from_utf16_lossy(&buf[..size as usize]);
        // Extract filename from full path: "C:\...\Code.exe" -> "Code.exe"
        path.rsplit('\\').next().map(|s| s.to_string())
    }
}
```

**Why `PROCESS_QUERY_LIMITED_INFORMATION` over `PROCESS_QUERY_INFORMATION`:** Limited access rights succeed for processes running as the same user without requiring elevation. `PROCESS_QUERY_INFORMATION` fails for some system processes. For our use case (user-launched apps like VS Code, Outlook, Bluebeam), limited is sufficient and more reliable.

### Integration Point

The detection call goes in `pipeline.rs` at lines 395-404, replacing the current direct `ActiveProfile` read:

```
Current: Read ActiveProfile.all_caps -> apply uppercase
New:     Call get_foreground_process_name() -> look up per-app override -> fall back to ActiveProfile.all_caps
```

This is a synchronous call (~0.1ms) on the blocking inject thread. No async needed.

## Backend: Process Enumeration (For Searchable Dropdown)

### Recommended: Raw Win32 via Existing `windows` Crate

Use `CreateToolhelp32Snapshot` + `Process32FirstW` / `Process32NextW` to enumerate all processes. This avoids adding `sysinfo` as a dependency.

**Additional feature flag needed:** `Win32_System_Diagnostics_ToolHelp`

| API | Module | Feature Flag | Status |
|-----|--------|-------------|--------|
| `CreateToolhelp32Snapshot()` | `Win32::System::Diagnostics::ToolHelp` | `Win32_System_Diagnostics_ToolHelp` | **NEW -- must add** |
| `Process32FirstW()` | `Win32::System::Diagnostics::ToolHelp` | `Win32_System_Diagnostics_ToolHelp` | **NEW -- must add** |
| `Process32NextW()` | `Win32::System::Diagnostics::ToolHelp` | `Win32_System_Diagnostics_ToolHelp` | **NEW -- must add** |
| `PROCESSENTRY32W` | `Win32::System::Diagnostics::ToolHelp` | `Win32_System_Diagnostics_ToolHelp` | **NEW -- must add** |
| `TH32CS_SNAPPROCESS` | `Win32::System::Diagnostics::ToolHelp` | `Win32_System_Diagnostics_ToolHelp` | **NEW -- must add** |

### Cargo.toml Change

```toml
# Before:
windows = { version = "0.58", features = [
    "Win32_Graphics_Dxgi",
    "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging",
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_System_Threading",
] }

# After (one feature added):
windows = { version = "0.58", features = [
    "Win32_Graphics_Dxgi",
    "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging",
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_System_Threading",
    "Win32_System_Diagnostics_ToolHelp",
] }
```

### Enumeration Pattern

```rust
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW,
    PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};

/// List unique running process names (e.g., ["Code.exe", "OUTLOOK.EXE", ...]).
/// Deduplicates and sorts alphabetically. Filters out system processes with empty names.
pub fn list_running_processes() -> Vec<String> {
    let mut names = std::collections::HashSet::new();
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok();
        let Some(snapshot) = snapshot else { return vec![] };

        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let name_len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..name_len]);
                if !name.is_empty() {
                    names.insert(name);
                }
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
    }
    let mut sorted: Vec<String> = names.into_iter().collect();
    sorted.sort_unstable_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    sorted
}
```

**Why raw Win32 over `sysinfo`:** `sysinfo` v0.37 is a 83M-download crate that pulls in significant platform abstraction overhead. We only need a flat list of process names on Windows. The ToolHelp32 snapshot API does exactly this in ~30 lines. Adding `sysinfo` for this would be like importing lodash for `_.capitalize()`. The `windows` crate is already a dependency -- one additional feature flag costs zero compile time increase beyond the generated bindings for that module.

## Frontend: Searchable Process Dropdown

### Recommended: Custom Component (No New Dependencies)

Build a simple `<SearchableDropdown>` component using:
- `<input>` with `onChange` for filtering
- Filtered `<ul>/<li>` list rendered below
- Keyboard navigation (ArrowUp/ArrowDown/Enter/Escape)
- `lucide-react` icons (already a dependency) for search/chevron icons

**Why custom over a library:**

| Option | Bundle Size | Dependencies | Fit |
|--------|-------------|-------------|-----|
| Custom component | ~0 KB (already have React + Tailwind) | None | Matches existing app patterns |
| `react-select` | ~27 KB min | emotion (CSS-in-JS) | Conflicts with Tailwind-only approach |
| `@headlessui/react` | ~12 KB min | None | Good library, but overkill for one dropdown |
| `react-select-search` | ~5 KB | None | Decent, but still unnecessary |

The app has 20 components, all custom-built with React + Tailwind + clsx/tailwind-merge. Adding a UI library for a single dropdown breaks the consistency. The interaction is straightforward: filter a list of ~50-200 process names, select one. This is a 60-80 line component.

### Tauri Command for Process List

Expose `list_running_processes()` as a Tauri command:

```rust
#[tauri::command]
fn get_running_processes() -> Vec<String> {
    list_running_processes()
}
```

Frontend calls it when the "Add App" flow is triggered. No polling -- one-shot fetch when the dropdown opens.

## What NOT to Add

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `sysinfo` crate | Heavyweight (~1.2MB compiled), pulls platform abstraction we don't need | `CreateToolhelp32Snapshot` via existing `windows` crate |
| `react-select` or similar | Adds CSS-in-JS dependency, conflicts with Tailwind approach | Custom `<SearchableDropdown>` component |
| `winapi` crate | Legacy crate, `windows` is the Microsoft-maintained replacement | Already using `windows` v0.58 |
| Continuous foreground monitoring | Polling or hooks to track active window changes | One-shot detection at injection time only |
| `GetWindowTextW` for window titles | Window titles change constantly, unreliable for app identification | Process executable name is stable |
| `Win32_System_ProcessStatus` feature (EnumProcesses) | Older API, returns only PIDs requiring separate OpenProcess for each | ToolHelp32 returns names directly in PROCESSENTRY32W |

## Version Compatibility

| Package | Version | Compatible With | Notes |
|---------|---------|-----------------|-------|
| `windows` | 0.58 | All new APIs | ToolHelp32 APIs available since 0.48+; no version bump needed |
| React | 18.3.1 | Custom dropdown component | Standard controlled input pattern |
| `lucide-react` | 0.577.0 | Search/ChevronDown icons | Already available |

## Alternatives Considered

| Recommended | Alternative | Why Not |
|-------------|-------------|---------|
| `windows` crate ToolHelp32 | `sysinfo` crate | 1.2MB compiled weight for a 30-line function; overkill |
| `windows` crate ToolHelp32 | `EnumProcesses` (psapi) | Returns only PIDs, requires per-process OpenProcess+QueryFullProcessImageName loop |
| `PROCESS_QUERY_LIMITED_INFORMATION` | `PROCESS_QUERY_INFORMATION` | Limited access works for user processes and doesn't fail on elevated processes |
| Process exe name matching | Window title matching | Exe names are stable ("Code.exe"); titles change with open files |
| One-shot detection at injection | Polling/event-based foreground tracking | No need for continuous tracking; we only care at paste time |
| Custom dropdown | Headless UI Combobox | Good library but adds dependency for one component |

## Sources

- [windows crate GetForegroundWindow docs](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/UI/WindowsAndMessaging/fn.GetForegroundWindow.html) -- module path and signature (HIGH confidence)
- [windows crate QueryFullProcessImageNameW docs](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/Threading/fn.QueryFullProcessImageNameW.html) -- signature and feature flag (HIGH confidence)
- [Tracking active process in Windows with Rust](https://hellocode.co/blog/post/tracking-active-process-windows-rust/) -- pattern for GetForegroundWindow -> PID -> process name chain (MEDIUM confidence, older windows crate version but pattern is identical)
- [sysinfo crate](https://crates.io/crates/sysinfo) -- v0.37.2 latest, evaluated and rejected for this use case (HIGH confidence)
- [Enumerating Windows processes with Rust](https://bazizi.github.io/2022/12/29/enumerating-windows-processes-using-Rust.html) -- ToolHelp32 pattern reference (MEDIUM confidence)
- Existing Cargo.toml and keyboard_hook.rs -- direct code review confirming current feature flags (HIGH confidence)

---
*Stack research for: v1.4 Per-App Settings - Foreground Window Detection*
*Researched: 2026-03-07*
