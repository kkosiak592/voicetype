# Phase 26: Process Dropdown - Context

**Gathered:** 2026-03-07
**Status:** Ready for planning

<domain>
## Phase Boundary

Searchable dropdown of running processes for adding apps to the rules list without using the detect flow. This is an alternative "add" mechanism alongside the existing Detect Active App button. No changes to detection, override resolution, or existing rules UI behavior.

</domain>

<decisions>
## Implementation Decisions

### Dropdown trigger & placement
- Separate "Browse Running Apps" button next to "Detect Active App"
- Browse button uses secondary/outline styling (Detect remains primary emerald)
- Clicking Browse opens a dropdown panel below the button with a search input and process list
- Dropdown closes when an item is selected or user clicks outside

### Process filtering
- Show only processes that have a visible window — filters out background services (svchost, RuntimeBroker, csrss) automatically
- Deduplicate by exe name — show each exe once, pick the window title from the first instance found
- Sort alphabetically by exe name (matches existing rules list sort order)
- Uses CreateToolhelp32Snapshot for process enumeration (Win32_System_Diagnostics_ToolHelp feature flag, noted in Phase 23)
- Case-normalize exe names at every boundary (carried from Phase 23)

### Search UX
- Search input at top of dropdown, auto-focused when dropdown opens
- Matches against both exe name and window title (typing "auto" matches acad.exe via "AutoCAD 2025")
- Each item displays: exe name bold + window title subtitle (matches existing rule row style)
- Clicking an item immediately adds it to rules list with Inherit default — no confirmation step
- Dropdown closes after adding an app

### Already-added handling
- Processes that already have rules appear dimmed/grayed out with "already added" label
- Dimmed items are non-clickable

### Claude's Discretion
- Process list fetch strategy (once on open vs refresh on keystroke)
- Dropdown max height and scroll behavior
- Search debounce timing
- Empty state when no processes match the search query
- Whether to show a loading state while enumerating processes
- Exact lucide icon for the Browse button

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `AppRulesSection.tsx`: Already has rule management (add, set, remove), detect flow, dropdown pattern for three-state toggle — process dropdown integrates here
- `foreground.rs`: Has `get_process_exe_name(pid)` for resolving PID to exe name, `get_window_title(hwnd)` for window titles — reusable for process enumeration
- `AppRulesState` and `AppRule` types in `foreground.rs` — new process list command returns data compatible with these types
- `invoke()` from `@tauri-apps/api/core` — established IPC pattern for the new `list_running_processes` command

### Established Patterns
- Tauri commands as `#[tauri::command]` async functions in `lib.rs`
- Managed state via `Mutex<T>` registered on Builder
- Dark mode via Tailwind `dark:` prefix throughout
- `windows` crate v0.58 already in Cargo.toml — needs `Win32_System_Diagnostics_ToolHelp` feature flag added

### Integration Points
- `foreground.rs`: Add `list_running_processes()` function using CreateToolhelp32Snapshot + window enumeration
- `lib.rs`: Register new `list_running_processes` Tauri command
- `Cargo.toml`: Add `Win32_System_Diagnostics_ToolHelp` to windows crate features
- `AppRulesSection.tsx`: Add Browse button, dropdown component, search state, process list fetching

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches matching existing UI patterns.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 26-process-dropdown*
*Context gathered: 2026-03-07*
