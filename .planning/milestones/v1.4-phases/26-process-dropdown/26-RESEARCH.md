# Phase 26: Process Dropdown - Research

**Researched:** 2026-03-07
**Domain:** Win32 process enumeration + React searchable dropdown UI
**Confidence:** HIGH

## Summary

This phase adds a "Browse Running Apps" button that opens a searchable dropdown of running processes with visible windows. The backend requires a new `list_running_processes` Tauri command using `CreateToolhelp32Snapshot` + `EnumWindows` to enumerate processes with visible windows, deduplicated by exe name. The frontend adds a dropdown panel with search input to `AppRulesSection.tsx`, reusing the existing `set_app_rule` IPC to add selected processes.

The implementation is straightforward because: (1) all Win32 APIs needed are already available via the `windows` crate v0.58 with one additional feature flag, (2) the existing `foreground.rs` already has the helper functions `get_process_exe_name` and `get_window_title` that can be reused, and (3) the `AppRulesSection.tsx` already has the add-rule pattern via `set_app_rule` IPC.

**Primary recommendation:** Fetch process list once when dropdown opens (not on every keystroke). Filter client-side. Use `Process32FirstW`/`Process32NextW` (wide-char variants) combined with `EnumWindows` + `IsWindowVisible` to build a deduplicated list.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Separate "Browse Running Apps" button next to "Detect Active App"
- Browse button uses secondary/outline styling (Detect remains primary emerald)
- Clicking Browse opens a dropdown panel below the button with a search input and process list
- Dropdown closes when an item is selected or user clicks outside
- Show only processes that have a visible window (filters out background services automatically)
- Deduplicate by exe name -- show each exe once, pick the window title from the first instance found
- Sort alphabetically by exe name (matches existing rules list sort order)
- Uses CreateToolhelp32Snapshot for process enumeration (Win32_System_Diagnostics_ToolHelp feature flag)
- Case-normalize exe names at every boundary
- Search input at top of dropdown, auto-focused when dropdown opens
- Matches against both exe name and window title
- Each item displays: exe name bold + window title subtitle (matches existing rule row style)
- Clicking an item immediately adds it to rules list with Inherit default -- no confirmation step
- Dropdown closes after adding an app
- Processes that already have rules appear dimmed/grayed out with "already added" label
- Dimmed items are non-clickable

### Claude's Discretion
- Process list fetch strategy (once on open vs refresh on keystroke)
- Dropdown max height and scroll behavior
- Search debounce timing
- Empty state when no processes match the search query
- Whether to show a loading state while enumerating processes
- Exact lucide icon for the Browse button

### Deferred Ideas (OUT OF SCOPE)
None.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| UI-04 | User can add an app via searchable dropdown of currently running processes | Backend: new `list_running_processes` command using ToolHelp32 + EnumWindows. Frontend: dropdown panel with search in AppRulesSection.tsx, reusing existing `set_app_rule` IPC. |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| windows (crate) | 0.58 | Win32 API bindings for process enumeration | Already in Cargo.toml, just needs one feature flag added |
| React + Tauri IPC | existing | Frontend dropdown UI + invoke backend command | Established project pattern |
| Tailwind CSS | existing | Dropdown styling with dark mode | All existing UI uses Tailwind dark: prefix |
| lucide-react | existing | Icon for Browse button | All existing buttons use lucide icons |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| serde/serde_json | 1.x | Serialize process list for IPC | Return Vec of process info structs to frontend |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| CreateToolhelp32Snapshot | EnumProcesses (psapi) | ToolHelp32 is already decided, gives exe name directly in PROCESSENTRY32W |
| Custom dropdown | Headless UI library | Overkill for single dropdown; project already has custom dropdown pattern in three-state toggle |

**Installation:**
```bash
# No new dependencies -- just add feature flag to existing windows crate in Cargo.toml:
# "Win32_System_Diagnostics_ToolHelp" added to windows features list
```

## Architecture Patterns

### Recommended Project Structure
```
src-tauri/src/
  foreground.rs          # Add list_running_processes() function
  lib.rs                 # Register new Tauri command, add #[tauri::command] wrapper

src/components/sections/
  AppRulesSection.tsx     # Add Browse button + dropdown panel + search state
```

### Pattern 1: Two-Phase Process Enumeration (Backend)
**What:** First enumerate all PIDs via CreateToolhelp32Snapshot, then use EnumWindows to find which PIDs have visible windows, cross-reference to build the filtered list.
**When to use:** Always -- this is the only reliable way to get "processes with visible windows."
**Example:**
```rust
// Step 1: Build PID -> exe_name map via ToolHelp32
// Step 2: EnumWindows callback collects (pid, window_title) for visible windows
// Step 3: Cross-reference: for each visible window's PID, look up exe_name
// Step 4: Deduplicate by exe_name (first window title wins)
// Step 5: Return sorted Vec<RunningProcess>

use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW,
    PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, IsWindowVisible, GetWindowTextW, GetWindowThreadProcessId,
};

#[derive(Clone, Serialize)]
pub struct RunningProcess {
    pub exe_name: String,       // lowercased
    pub window_title: String,   // from first visible window
}

pub fn list_running_processes() -> Vec<RunningProcess> {
    // Implementation uses unsafe Win32 calls
    // Returns deduplicated, alphabetically sorted list
}
```

### Pattern 2: Fetch-Once Dropdown (Frontend)
**What:** Fetch process list once when dropdown opens, filter client-side as user types.
**When to use:** Default strategy -- process enumeration takes ~10-50ms, no need to re-fetch on keystroke.
**Example:**
```typescript
const [browseOpen, setBrowseOpen] = useState(false);
const [processes, setProcesses] = useState<RunningProcess[]>([]);
const [search, setSearch] = useState('');
const searchRef = useRef<HTMLInputElement>(null);

async function handleBrowseClick() {
  setBrowseOpen(true);
  const list = await invoke<RunningProcess[]>('list_running_processes');
  setProcesses(list);
  // Auto-focus search input after render
}

const filtered = processes.filter(p =>
  p.exe_name.includes(search.toLowerCase()) ||
  p.window_title.toLowerCase().includes(search.toLowerCase())
);
```

### Pattern 3: Outside-Click Dismiss (Existing Pattern)
**What:** Close dropdown when clicking outside, using mousedown event listener on document.
**When to use:** Already implemented for three-state toggle dropdown in AppRulesSection.tsx.
**Example:**
```typescript
// Already in codebase -- reuse same pattern for browse dropdown
useEffect(() => {
  if (!browseOpen) return;
  function handleClick(e: MouseEvent) {
    if (browseRef.current && !browseRef.current.contains(e.target as Node)) {
      setBrowseOpen(false);
      setSearch('');
    }
  }
  document.addEventListener('mousedown', handleClick);
  return () => document.removeEventListener('mousedown', handleClick);
}, [browseOpen]);
```

### Anti-Patterns to Avoid
- **Re-fetching on every keystroke:** Process enumeration involves Win32 snapshot + window enumeration. Fetch once, filter in JS.
- **Using PROCESSENTRY32 (ANSI) instead of PROCESSENTRY32W (Wide):** The project uses wide-string APIs throughout (GetWindowTextW, QueryFullProcessImageNameW). Use the W variants.
- **Not zeroing szExeFile between iterations:** Process32NextW does NOT zero-initialize the buffer. The exe name from a previous iteration can leak into shorter names. Always read up to the null terminator, or zero the buffer before each call.
- **Forgetting CloseHandle on snapshot:** CreateToolhelp32Snapshot returns a HANDLE that must be closed.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Process enumeration | Manual NtQuerySystemInformation | CreateToolhelp32Snapshot + Process32FirstW/NextW | Documented, safe, already decided |
| Visible window detection | Checking window styles manually | IsWindowVisible from Win32 | Handles all edge cases (minimized still counts as visible, which is correct for our use case) |
| Exe name resolution | Parse PROCESSENTRY32W szExeFile | Reuse existing `get_process_exe_name(pid)` or read szExeFile directly | szExeFile gives bare filename, but get_process_exe_name gives full-path resolution. For dropdown, szExeFile is sufficient since we only need the bare name |

**Key insight:** The `szExeFile` field in PROCESSENTRY32W already contains the bare exe filename (not full path), so for the dropdown list we can read it directly without needing `OpenProcess` + `QueryFullProcessImageNameW`. This is faster and works even for elevated processes we can't open. However, the name must still be lowercased for case normalization.

## Common Pitfalls

### Pitfall 1: szExeFile Buffer Not Zeroed Between Iterations
**What goes wrong:** Process32NextW writes the new exe name into szExeFile but doesn't zero-fill the rest. If the new name is shorter than the previous one, leftover characters from the previous name remain.
**Why it happens:** Win32 API behavior -- it writes up to the null terminator only.
**How to avoid:** Either zero the buffer before each Process32NextW call, or find the null terminator when converting to String. The simplest approach: `let name_len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(260);` then convert `&entry.szExeFile[..name_len]`.
**Warning signs:** Process names appearing with garbage suffix characters.

### Pitfall 2: Snapshot Handle Leak
**What goes wrong:** CreateToolhelp32Snapshot returns a HANDLE. If not closed, it leaks.
**Why it happens:** Rust's ownership doesn't auto-close Win32 HANDLEs.
**How to avoid:** Call `CloseHandle(snapshot)` in a finally-like pattern. Consider using a defer/drop guard, or simply close after the loop.
**Warning signs:** Resource leak under repeated dropdown opens.

### Pitfall 3: EnumWindows Callback Safety
**What goes wrong:** The callback receives an LPARAM that is cast to a mutable pointer. If the callback panics, it unwinds across FFI boundary (UB).
**Why it happens:** Rust panics + extern "system" callbacks = undefined behavior.
**How to avoid:** Wrap callback body in `std::panic::catch_unwind` or use simple logic that cannot panic. The existing `enum_child_proc` in foreground.rs sets a good pattern.
**Warning signs:** Crash on dropdown open with certain window configurations.

### Pitfall 4: Case Sensitivity Mismatch
**What goes wrong:** Process from dropdown added as "Notepad.exe" but rules keyed by "notepad.exe" -- comparison fails.
**Why it happens:** szExeFile preserves original case from the OS.
**How to avoid:** Lowercase exe names at the Rust boundary before returning to frontend (matching existing `get_process_exe_name` pattern). The `set_app_rule` command already lowercases via `exe_name.to_lowercase()`.
**Warning signs:** Duplicate entries appearing in rules list.

### Pitfall 5: Including Own Process
**What goes wrong:** The VoiceType app itself appears in the process list.
**Why it happens:** The app has a visible window, so it passes the visibility filter.
**How to avoid:** Optionally exclude the current process by comparing against `std::process::id()`. Not strictly required but improves UX.
**Warning signs:** Users see their own app in the browse list.

## Code Examples

### Backend: Complete list_running_processes Implementation Pattern
```rust
// Source: windows crate docs + project foreground.rs patterns
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW,
    PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, IsWindowVisible, GetWindowTextW, GetWindowThreadProcessId,
};
use windows::Win32::Foundation::{BOOL, CloseHandle, HWND, LPARAM};
use std::collections::HashMap;

#[derive(Clone, Serialize)]
pub struct RunningProcess {
    pub exe_name: String,
    pub window_title: String,
}

pub fn list_running_processes() -> Vec<RunningProcess> {
    unsafe {
        // Step 1: Enumerate visible windows -> collect (pid, title) pairs
        let mut window_info: Vec<(u32, String)> = Vec::new();
        let _ = EnumWindows(
            Some(enum_visible_windows),
            LPARAM(&mut window_info as *mut Vec<(u32, String)> as isize),
        );

        // Step 2: Snapshot processes -> build pid-to-exe map
        let mut pid_to_exe: HashMap<u32, String> = HashMap::new();
        if let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            let mut entry = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };
            if Process32FirstW(snapshot, &mut entry).is_ok() {
                loop {
                    let name_len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(260);
                    let exe_name = String::from_utf16_lossy(&entry.szExeFile[..name_len])
                        .to_lowercase();
                    pid_to_exe.insert(entry.th32ProcessID, exe_name);
                    entry.szExeFile = [0u16; 260]; // Zero buffer before next
                    if Process32NextW(snapshot, &mut entry).is_err() {
                        break;
                    }
                }
            }
            let _ = CloseHandle(snapshot);
        }

        // Step 3: Cross-reference: visible window PIDs -> exe names, deduplicate
        let mut seen: HashMap<String, String> = HashMap::new(); // exe -> title
        for (pid, title) in &window_info {
            if let Some(exe) = pid_to_exe.get(pid) {
                seen.entry(exe.clone()).or_insert_with(|| title.clone());
            }
        }

        // Step 4: Sort and return
        let mut result: Vec<RunningProcess> = seen
            .into_iter()
            .map(|(exe_name, window_title)| RunningProcess { exe_name, window_title })
            .collect();
        result.sort_by(|a, b| a.exe_name.cmp(&b.exe_name));
        result
    }
}

unsafe extern "system" fn enum_visible_windows(hwnd: HWND, lparam: LPARAM) -> BOOL {
    if IsWindowVisible(hwnd).as_bool() {
        let mut title_buf = [0u16; 512];
        let len = GetWindowTextW(hwnd, &mut title_buf);
        if len > 0 {
            let title = String::from_utf16_lossy(&title_buf[..len as usize]);
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid != 0 {
                let vec = &mut *(lparam.0 as *mut Vec<(u32, String)>);
                vec.push((pid, title));
            }
        }
    }
    BOOL(1) // Continue enumeration
}
```

### Frontend: Browse Button + Dropdown Pattern
```typescript
// Secondary/outline button styling (next to primary emerald Detect button)
<button
  onClick={handleBrowseClick}
  className="inline-flex items-center gap-2 rounded-xl px-4 py-2 text-sm font-medium
    ring-1 ring-gray-300 dark:ring-gray-600
    text-gray-700 dark:text-gray-300
    hover:bg-gray-100 dark:hover:bg-gray-800
    transition-colors"
>
  <List className="size-4" />
  Browse Running Apps
</button>
```

### Frontend: Searchable Process List
```typescript
{browseOpen && (
  <div ref={browseRef} className="absolute z-20 mt-1 w-80 max-h-72 rounded-xl
    bg-white dark:bg-gray-800 shadow-lg ring-1 ring-gray-200 dark:ring-gray-700
    flex flex-col overflow-hidden">
    <input
      ref={searchRef}
      value={search}
      onChange={e => setSearch(e.target.value)}
      placeholder="Search processes..."
      className="px-3 py-2 text-sm border-b border-gray-200 dark:border-gray-700
        bg-transparent text-gray-900 dark:text-gray-100
        placeholder-gray-400 dark:placeholder-gray-500
        outline-none"
    />
    <div className="overflow-y-auto">
      {filtered.map(proc => {
        const alreadyAdded = proc.exe_name in rules;
        return (
          <button
            key={proc.exe_name}
            disabled={alreadyAdded}
            onClick={() => handleAddFromBrowse(proc.exe_name, proc.window_title)}
            className={`w-full text-left px-3 py-2 text-sm transition-colors ${
              alreadyAdded
                ? 'opacity-50 cursor-default'
                : 'hover:bg-gray-100 dark:hover:bg-gray-700 cursor-pointer'
            }`}
          >
            <span className="font-semibold text-gray-900 dark:text-gray-100">
              {proc.exe_name}
            </span>
            {alreadyAdded && (
              <span className="ml-2 text-xs text-gray-400">already added</span>
            )}
            <p className="text-xs text-gray-500 dark:text-gray-400 truncate">
              {proc.window_title}
            </p>
          </button>
        );
      })}
    </div>
  </div>
)}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| PROCESSENTRY32 (ANSI) | PROCESSENTRY32W (Wide) | windows crate convention | Use W variant for proper Unicode support |
| EnumProcesses + OpenProcess for each | CreateToolhelp32Snapshot | Already decided | Single snapshot is faster than N OpenProcess calls |
| Process32First/Next (ANSI) | Process32FirstW/NextW | windows crate convention | Matches project's wide-string pattern |

**Deprecated/outdated:**
- PROCESSENTRY32 (non-W): Still available but ANSI variant; the project uses wide APIs throughout

## Open Questions

None -- all technical questions resolved through code examination and API docs.

## Discretion Recommendations

For the areas marked as Claude's Discretion:

1. **Process list fetch strategy:** Fetch once on dropdown open. Process enumeration is fast (~10-50ms). No need to refresh on keystroke. Client-side filtering is instant.

2. **Dropdown max height and scroll behavior:** `max-h-72` (18rem / ~288px) with `overflow-y-auto`. Shows ~6-7 items before scrolling, reasonable for a dropdown.

3. **Search debounce timing:** No debounce needed. Filtering is client-side against an already-fetched list. React state updates on every keystroke are fine for <100 items.

4. **Empty state:** Show "No matching processes" centered text in gray when filter returns zero results.

5. **Loading state:** Show a brief loading spinner or "Loading..." text while the invoke call is in flight. Process enumeration is fast but the IPC round-trip adds latency on first open.

6. **Lucide icon:** `List` or `AppWindow` from lucide-react. `List` is more universally recognizable for a "browse list" action.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust unit tests (cargo test) |
| Config file | Cargo.toml (existing) |
| Quick run command | `cargo test --lib -p voice-to-text` |
| Full suite command | `cargo test -p voice-to-text` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| UI-04 | list_running_processes returns Vec of RunningProcess | manual-only | N/A -- requires live Win32 environment with running processes | N/A |
| UI-04 | Dropdown search filters correctly | manual-only | N/A -- React component behavior, visual verification | N/A |
| UI-04 | Adding from browse calls set_app_rule | manual-only | N/A -- IPC integration test requires running Tauri app | N/A |

**Justification for manual-only:** The `list_running_processes` function calls live Win32 APIs (CreateToolhelp32Snapshot, EnumWindows) that require a real Windows desktop session with running GUI processes. Unit testing would require mocking the entire Win32 API surface, which adds complexity without meaningful coverage. The frontend dropdown is a React component that requires visual verification. The existing set_app_rule IPC is already tested via the detect flow.

### Sampling Rate
- **Per task commit:** `cargo test --lib -p voice-to-text` (ensures no regressions)
- **Per wave merge:** `cargo test -p voice-to-text`
- **Phase gate:** Cargo build succeeds + manual verification of dropdown behavior

### Wave 0 Gaps
None -- existing test infrastructure covers all unit-testable code. New code is Win32 API integration that requires manual verification.

## Sources

### Primary (HIGH confidence)
- [windows crate ToolHelp module docs](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/Diagnostics/ToolHelp/index.html) - PROCESSENTRY32W struct, CreateToolhelp32Snapshot, Process32FirstW/NextW signatures
- [CreateToolhelp32Snapshot docs](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/Diagnostics/ToolHelp/fn.CreateToolhelp32Snapshot.html) - Function signature and parameters
- [EnumWindows docs](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/UI/WindowsAndMessaging/fn.EnumWindows.html) - Callback pattern for window enumeration
- [IsWindowVisible docs](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/UI/WindowsAndMessaging/fn.IsWindowVisible.html) - Visibility check function
- Project codebase: `foreground.rs`, `lib.rs`, `AppRulesSection.tsx` - Existing patterns and integration points

### Secondary (MEDIUM confidence)
- [Rustware Process Enumeration](https://securethinklab.com/blog/2023-11-06-rustware-part-2-process-enumeration-development/) - Working Rust + windows crate examples
- [Enumerating Windows Processes in Rust](https://bazizi.github.io/2022/12/29/enumerating-windows-processes-using-Rust.html) - szExeFile buffer zeroing issue documented
- [windows-rs issue #2879](https://github.com/microsoft/windows-rs/issues/2879) - Process32Next szExeFile not zeroed between calls

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - all libraries already in project, just adding one feature flag
- Architecture: HIGH - pattern directly follows existing foreground.rs + AppRulesSection.tsx code
- Pitfalls: HIGH - szExeFile zeroing issue is well-documented; other pitfalls from existing codebase patterns

**Research date:** 2026-03-07
**Valid until:** 2026-04-07 (stable Win32 APIs, unlikely to change)
