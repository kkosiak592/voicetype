# Pitfalls Research

**Domain:** Per-app settings with foreground window detection on Windows (Tauri 2.0 / Rust)
**Researched:** 2026-03-07
**Confidence:** HIGH

---

## Critical Pitfalls

### Pitfall 1: UWP Apps Return ApplicationFrameHost.exe Instead of Real Process

**What goes wrong:**
`GetForegroundWindow()` + `GetWindowThreadProcessId()` + `OpenProcess()` returns `ApplicationFrameHost.exe` for all UWP/Store apps (Calculator, Windows Terminal from Store, Mail, Calendar, Settings, Photos). The user adds a rule for "Calculator" but the detected process is `ApplicationFrameHost.exe`, which matches ALL UWP apps simultaneously. One rule applies to every UWP app on the system.

**Why it happens:**
UWP apps run inside an `ApplicationFrameHost.exe` container process that owns the top-level HWND. The actual app process is a child, but the window handle belongs to the host. This is fundamental Windows architecture since Windows 8, not a bug. PowerToys, AutoHotkey, and every window-management tool has hit this exact issue.

**How to avoid:**
After detecting `ApplicationFrameHost.exe`, enumerate child windows of the foreground HWND using `EnumChildWindows`. The first child window owned by a different PID is the actual UWP app. Call `GetWindowThreadProcessId` on that child to get the real PID, then resolve the exe name via `QueryFullProcessImageNameW`. Fallback: use `GetWindowText` on the parent HWND as the identifier when child enumeration fails (minimized UWP apps sometimes have no reliable child process link).

```rust
let hwnd = GetForegroundWindow();
let pid = GetWindowThreadProcessId(hwnd);
let exe = get_process_name(pid);
if exe == "applicationframehost.exe" {
    if let Some(real_pid) = find_uwp_child_process(hwnd) {
        exe = get_process_name(real_pid);
    } else {
        // Fallback: window title as identifier
        exe = format!("[UWP] {}", get_window_title(hwnd));
    }
}
```

**Warning signs:**
- All UWP apps matching the same rule
- Test with Calculator, Settings, Mail -- if they all report the same process name, this pitfall is active

**Phase to address:**
Phase 1 (foreground detection backend). Must be solved before any per-app matching logic is built.

---

### Pitfall 2: Elevated/Admin Processes Cause Access Denied on OpenProcess

**What goes wrong:**
When the foreground window belongs to an elevated process (Task Manager, Device Manager, Registry Editor, or any app run as Admin), `OpenProcess(PROCESS_QUERY_INFORMATION, ...)` returns `ERROR_ACCESS_DENIED`. Process name resolution fails, and the app either panics on an unwrap, or silently falls back to global settings without telling the user why their rule didn't apply.

**Why it happens:**
VoiceType runs as a standard user. Windows DACL restrictions prevent non-elevated processes from opening elevated process handles with `PROCESS_QUERY_INFORMATION` access rights. This is UAC working as designed.

**How to avoid:**
Use `PROCESS_QUERY_LIMITED_INFORMATION` instead of `PROCESS_QUERY_INFORMATION` when calling `OpenProcess`. This access right was introduced in Vista specifically to allow querying process image names without elevation. It permits `QueryFullProcessImageNameW` to succeed on elevated processes.

```rust
let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid);
```

If even `PROCESS_QUERY_LIMITED_INFORMATION` fails (System process, CSRSS, some protected processes like antimalware), treat as "unknown app" and apply global settings. Never attempt `SeDebugPrivilege` escalation -- it defeats UAC and is unnecessary for this use case.

**Warning signs:**
- "Access Denied" errors in logs when Task Manager or other admin apps are focused
- Per-app rules silently not applying for some apps
- Works on developer machine (if running IDE as admin) but fails for normal users

**Phase to address:**
Phase 1. Use `PROCESS_QUERY_LIMITED_INFORMATION` from day one. This is a one-line difference that prevents an entire class of failures.

---

### Pitfall 3: Race Condition Between Foreground Detection and Text Injection

**What goes wrong:**
Detection happens at the START of pipeline processing, but `inject_text()` fires Ctrl+V 500-2000ms later (transcription time). The user alt-tabs during transcription. ALL CAPS setting from app A is applied, but text is injected into app B. Wrong formatting for the destination window.

**Why it happens:**
Looking at `pipeline.rs`, the current flow is: hotkey release -> stop recording -> VAD gate -> transcribe (500-2000ms) -> corrections -> ALL CAPS (line 396-404) -> inject. If foreground detection happens at step 1 but injection at step 6, the user has had seconds to switch windows. This is the same class of race condition that the existing keyboard hook already handles carefully.

**How to avoid:**
Detect the foreground window at INJECTION TIME, not at recording time. The correct detection point is immediately before clipboard write in `inject_text()`. The window receiving the Ctrl+V is the one that matters for per-app settings.

This means the ALL CAPS transformation (currently in `pipeline.rs` lines 396-404, reading from `ActiveProfile`) must move to the injection layer. The pipeline should pass the raw corrected text, and the injection function resolves the foreground app, looks up per-app overrides, applies formatting, then injects.

```
Current flow:  transcribe -> correct -> ALL CAPS -> inject
Required flow: transcribe -> correct -> inject(raw_text) -> detect_app -> apply_per_app_caps -> clipboard+paste
```

**Warning signs:**
- Wrong capitalization after alt-tabbing during transcription
- ALL CAPS text appearing in non-CAPS-configured apps
- Testing only in single-window scenarios masks this entirely -- must test with deliberate alt-tab during dictation

**Phase to address:**
Phase 1 (architecture). The detection-at-injection-time pattern must be the design from the start. Retrofitting it after building detection-at-recording-time requires rearchitecting the pipeline formatting flow.

---

### Pitfall 4: Case-Sensitive Exe Name Matching Causes Silent Rule Misses

**What goes wrong:**
Process names returned by Windows APIs have inconsistent casing. `QueryFullProcessImageNameW` returns `C:\Program Files\Autodesk\AutoCAD\acad.exe` while `CreateToolhelp32Snapshot` + `Process32Next` returns `ACAD.EXE`. The user adds a rule via "Detect Active App" (which got `acad.exe`), but the injection-time detection resolves `ACAD.EXE`. Case-sensitive string comparison fails to match.

**Why it happens:**
Windows filesystem is case-insensitive but case-preserving. Different Win32 APIs return different casings depending on how the process was launched (file association vs. shortcut vs. command line). The exe name in the PE header may differ from the filesystem name.

**How to avoid:**
Normalize ALL process names to lowercase at every boundary:
1. When storing a rule from "Detect Active App": `.to_lowercase()`
2. When detecting foreground app at injection time: `.to_lowercase()`
3. When populating the searchable dropdown: `.to_lowercase()`

Also: strip the full path -- store and match on filename only (`acad.exe`), not the full path. Paths differ between installations, user profiles, and Program Files vs. Program Files (x86).

```rust
let exe_name = Path::new(&full_path)
    .file_name()
    .map(|f| f.to_string_lossy().to_lowercase())
    .unwrap_or_default();
```

**Warning signs:**
- Rules working for some apps but not others, no clear pattern
- Rules working when added via "Detect" but failing after app update changed installation path
- Developer machine matches because the same API is used for both add and detect, but a different code path uses a different API

**Phase to address:**
Phase 1 (data model). Normalize early, normalize consistently. Establish a single `normalize_exe_name()` function used everywhere.

---

### Pitfall 5: Settings Migration Breaks Existing Users on Upgrade from v1.3

**What goes wrong:**
Adding `appRules` to `settings.json` causes issues when existing v1.3 users upgrade. If any code path does `settings["appRules"].as_array().unwrap()` on a v1.3 settings file that has no `appRules` key, it panics. User's app crashes on startup after update. All settings lost if the error handling resets to defaults.

**Why it happens:**
The current settings system uses `serde_json::Value` via a Mutex, with individual keys read/written through `get_setting`/`set_setting` IPC (see `store.ts`). New top-level keys are naturally absent in old settings files, and `get_setting("appRules")` returns `null`. The risk is in code that consumes the value without handling null.

**How to avoid:**
Since the store uses untyped `serde_json::Value`, new keys are naturally absent and `get_setting` returns null. Enforce null-safety at every consumption point:

- Backend: `settings.get("appRules").and_then(|v| v.as_array()).unwrap_or(&Vec::new())`
- Frontend: `const rules = (await store.get<AppRule[]>('appRules')) ?? []`
- Never write a migration script that modifies existing settings files
- Test by running the app with a real v1.3 settings.json (no `appRules` key) and verifying clean startup

The existing `serde_json::Value` approach is migration-friendly by design -- leverage it, don't fight it by switching to a typed struct with `#[serde(default)]` unless you also handle existing files.

**Warning signs:**
- App crashes on startup after update
- All user settings reset to defaults after update
- Works on clean install but breaks on upgrade from v1.3

**Phase to address:**
Phase 2 (settings/UI integration). Must be tested with a real v1.3 settings file before release.

---

### Pitfall 6: GetForegroundWindow Returns NULL or VoiceType's Own Window

**What goes wrong:**
`GetForegroundWindow()` returns `NULL` during window transitions (alt-tab animation, UAC consent dialog, lock screen). It can also return VoiceType's own pill overlay HWND or settings window HWND, causing the app to look up rules for itself.

**Why it happens:**
During activation changes, there is a brief period where no window is foreground. The pill overlay is a visible Tauri window during recording/processing -- if it has focus (even briefly), it becomes the foreground window. The existing pill window likely has `WS_EX_NOACTIVATE` / `WS_EX_TOOLWINDOW` via Tauri config, but this needs verification.

**How to avoid:**
1. **NULL guard:** If `GetForegroundWindow()` returns null, fall back to global settings. Do not retry in a loop -- just use defaults.
2. **Self-detection filter:** Get VoiceType's own HWNDs (pill window, settings window) via `tauri::WebviewWindow::hwnd()` and skip them. If detected HWND is VoiceType's own, use the previously cached foreground app or fall back to global settings.
3. **Verify pill window attributes:** Confirm the pill window config in `tauri.conf.json` has `decorations: false`, `skipTaskbar: true`, and ideally is non-focusable. If the pill can receive focus, it will be the foreground window during the exact moment detection runs.

**Warning signs:**
- Per-app rules randomly not applying (NULL case)
- VoiceType applying its own app rules to itself
- Rules failing specifically during rapid alt-tab sequences

**Phase to address:**
Phase 1. The NULL check and self-detection filter are part of the core detection function.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Matching on window title instead of exe name for all apps | No OpenProcess needed, simpler code | Titles change with document names, localization, tab changes | Only as fallback for UWP apps when child window enum fails |
| Storing full exe path instead of filename only | More precise matching | Breaks on reinstall, different user profiles, different drive letters | Never -- always strip to filename |
| Hardcoding `"applicationframehost.exe"` check | Works for current Windows versions | Microsoft could rename the host process | Acceptable -- isolate to one function for future-proofing |
| Keeping ALL CAPS logic in pipeline.rs and adding per-app override as a separate layer | Minimal pipeline changes | Two places where caps logic lives, confusing ownership | Only for initial prototype; refactor before shipping |
| Using `EnumProcesses` for process dropdown | Simple API | Returns only PIDs, requires opening each process for name -- O(n) OpenProcess calls | Never -- use `CreateToolhelp32Snapshot` which gives exe names directly |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| `windows` crate process access | Using `PROCESS_QUERY_INFORMATION` (fails on elevated) | Use `PROCESS_QUERY_LIMITED_INFORMATION` with `QueryFullProcessImageNameW` |
| Tauri window handles | Assuming Tauri exposes raw HWND easily | Use `webview_window.hwnd()` -- available in Tauri 2.0 for Windows |
| Process handle cleanup | Opening process handle, getting name, forgetting `CloseHandle` | Use `windows` crate RAII wrappers or explicit `CloseHandle` in a Drop guard |
| serde_json settings | Adding typed struct for `AppRule` with strict deserialization | Keep using `serde_json::Value` for the settings file; deserialize `appRules` value into `Vec<AppRule>` with fallback to empty vec |
| Process enumeration dropdown | `EnumProcesses` returns PIDs only, must open each for name | Use `CreateToolhelp32Snapshot` + `Process32First/Next` -- gives exe name directly without opening each process |
| Current pipeline ALL CAPS | Adding per-app check alongside existing profile-based caps | Move ALL CAPS out of pipeline.rs entirely; detection + formatting happens at injection time |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Calling `CreateToolhelp32Snapshot` on every injection to enumerate all processes | 10-50ms added to injection latency | Only call `GetForegroundWindow` + resolve single PID at injection time; full enumeration only when dropdown opens | Immediately noticeable -- injection latency jumps for every dictation |
| Enumerating child windows on every UWP app detection | 5-20ms per `EnumChildWindows` call | Cache last detection result with HWND as key; only re-resolve if HWND changed | High-frequency dictation into UWP apps |
| Refreshing process list on every keystroke in searchable dropdown | UI jank with 200+ running processes | Snapshot process list once when dropdown opens, filter in-memory on keystrokes | Any machine with many background processes |
| Resolving process name via full path then stripping | `QueryFullProcessImageNameW` resolves the full path including network drives | Use `GetProcessImageFileNameW` or just `Process32Next.szExeFile` which returns filename only | When target app is on a network share -- path resolution can block on network I/O |

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Enabling `SeDebugPrivilege` to read elevated process names | Defeats UAC security model, unnecessary privilege escalation | Use `PROCESS_QUERY_LIMITED_INFORMATION` -- works without elevation for image name queries |
| Storing exe paths that reveal user directory structure | Minor privacy leak in settings file (username in path) | Store filename only, not full path |

---

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Showing raw exe names like `ApplicationFrameHost.exe` or `svchost.exe` in UI | Confusing, user doesn't know what app this is | Show resolved app name (window title for UWP, product name from version info for Win32) alongside exe name |
| No visual indicator of which app rule is active during injection | User can't verify per-app detection is working | Show the detected app name briefly in tray tooltip: "VoiceType -- injected to Code.exe (ALL CAPS)" |
| Showing ALL processes in the dropdown including system services | Overwhelming list of `svchost.exe`, `csrss.exe`, `dwm.exe` | Filter to windowed processes only -- processes that have at least one visible top-level window |
| No way to test if a rule matches before dictating | User adds rule, not sure if it will work | The "Detect Active App" flow naturally validates detection -- show the detected exe name in the UI when the user presses the detect button |
| Global ALL CAPS toggle disappearing or being confusing alongside per-app rules | User unsure whether global or per-app setting applies | Label clearly: "Default for all apps" on the global toggle; per-app rules say "Override: ALL CAPS ON/OFF" |

---

## "Looks Done But Isn't" Checklist

- [ ] **UWP detection:** Works with Calculator, Settings, Windows Terminal (Store version) -- not just Win32 apps like Notepad
- [ ] **Elevated process handling:** Dictate while Task Manager (run as admin) is focused -- no crash, global settings apply
- [ ] **Self-detection filter:** Pill overlay is visible during processing -- detected app is NOT VoiceType itself
- [ ] **NULL foreground:** Alt-tab rapidly during transcription -- no crash, global settings apply
- [ ] **Case-insensitive matching:** Rule added for `Code.exe`, detected as `code.exe` at injection -- rule matches
- [ ] **Path-independent matching:** Rule still works after app reinstall to a different directory
- [ ] **Settings migration:** App starts cleanly with v1.3 `settings.json` that has no `appRules` key
- [ ] **Global fallback:** Global ALL CAPS toggle works as default when no per-app rule matches the foreground app
- [ ] **Injection-time detection:** ALL CAPS override uses foreground window at injection time, not at recording start time
- [ ] **Multi-monitor:** Detection returns correct window when pill is on monitor 2 and target app is on monitor 1
- [ ] **Target apps:** Tested with Bluebeam, AutoCAD, Revit, VS Code, Chrome, Excel, Outlook, Teams (all listed in PROJECT.md target apps)
- [ ] **Searchable dropdown:** Only shows windowed processes, not system services; shows friendly names

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| UWP ApplicationFrameHost not handled | LOW | Add `EnumChildWindows` fallback in detection function -- isolated change |
| Access Denied on elevated processes | LOW | Change `PROCESS_QUERY_INFORMATION` to `PROCESS_QUERY_LIMITED_INFORMATION` -- one constant change |
| Race condition (detection at wrong time) | HIGH | Must move ALL CAPS logic from pipeline.rs to injection layer -- rearchitects the formatting flow. This is why it must be designed correctly in Phase 1. |
| Case-sensitive matching shipped | MEDIUM | Add `.to_lowercase()` normalization everywhere + one-time lowercase conversion of existing stored rules |
| Settings migration breaks upgrades | HIGH | Users lose all settings on crash, must reconfigure. No recovery for lost config. Prevention is the only option. |
| NULL foreground window not handled | LOW | Add null check with global-settings fallback -- single if-statement |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| UWP ApplicationFrameHost (#1) | Phase 1 (detection backend) | Detect Calculator, Settings, Mail -- all show correct distinct app names |
| Elevated process Access Denied (#2) | Phase 1 (detection backend) | Open Task Manager as admin, dictate -- no crash, falls back to global settings |
| Race condition timing (#3) | Phase 1 (architecture) | Alt-tab during transcription -- correct CAPS applied for destination window |
| Case-sensitive matching (#4) | Phase 1 (data model) | Add rule for "Code.exe", detect as "code.exe" at runtime -- rule matches |
| Settings migration (#5) | Phase 2 (settings integration) | Upgrade from v1.3 settings file -- app starts, global CAPS works, no rules present |
| NULL/self foreground window (#6) | Phase 1 (detection backend) | Alt-tab rapidly during dictation -- no crash, global settings applied |

---

## Sources

- [GetForegroundWindow function - Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getforegroundwindow) -- HIGH confidence
- [Process Security and Access Rights - Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/procthread/process-security-and-access-rights) -- HIGH confidence
- [OpenProcess function - Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-openprocess) -- HIGH confidence
- [Why does OpenProcess return access denied, even if I enable debug privilege? - Raymond Chen](https://devblogs.microsoft.com/oldnewthing/20151210-00/?p=92451) -- HIGH confidence
- [Tracking the current active process in Windows with Rust - Hello Code](https://hellocode.co/blog/post/tracking-active-process-windows-rust/) -- MEDIUM confidence
- [ApplicationFrameHost.exe and UWP process detection - AutoHotkey Community](https://www.autohotkey.com/boards/viewtopic.php?style=7&t=112906) -- MEDIUM confidence
- [Window Walker UWP process names - PowerToys Issue #1766](https://github.com/microsoft/PowerToys/issues/1766) -- MEDIUM confidence
- [Enumerating All Processes - Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/psapi/enumerating-all-processes) -- HIGH confidence
- [GetForegroundWindow in windows crate - Rust docs](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/UI/WindowsAndMessaging/fn.GetForegroundWindow.html) -- HIGH confidence
- Current codebase: `src-tauri/src/pipeline.rs` lines 396-404 (ALL CAPS logic), `src-tauri/src/inject.rs` (injection flow), `src-tauri/src/profiles.rs` (profile struct), `src/lib/store.ts` (settings facade) -- direct analysis

---
*Pitfalls research for: Per-app settings with foreground window detection (VoiceType v1.4)*
*Researched: 2026-03-07*
