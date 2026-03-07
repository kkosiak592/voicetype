# Project Research Summary

**Project:** VoiceType v1.4 Per-App Settings
**Domain:** Win32 foreground window detection + per-application setting overrides for Tauri 2.0 desktop app
**Researched:** 2026-03-07
**Confidence:** HIGH

## Executive Summary

VoiceType v1.4 adds per-application settings overrides, starting with ALL CAPS. The core mechanism is straightforward: detect the foreground window's process name at text injection time using Win32 APIs (`GetForegroundWindow` -> `GetWindowThreadProcessId` -> `QueryFullProcessImageNameW`), look up per-app rules stored in `settings.json`, and apply overrides before pasting. The existing `windows` crate v0.58 already provides all necessary APIs except process enumeration for the searchable dropdown, which requires one additional feature flag (`Win32_System_Diagnostics_ToolHelp`). No new crate dependencies are needed on the backend. The frontend needs one new sidebar section and a custom searchable dropdown component, both buildable with existing React + Tailwind patterns.

The recommended approach is: build the detection module and data model first (they have no UI dependency and can be tested via settings.json manipulation), then integrate into the pipeline at injection time, then build the management UI. The architecture uses a two-level override resolution chain -- per-app override falls back to global default -- with `Option<bool>` fields providing clean three-state semantics (inherit / force ON / force OFF). The data model uses a `HashMap<String, AppOverride>` keyed by lowercase process executable name, stored under an `app_rules` key in the existing settings.json.

The primary risks are: (1) UWP apps reporting as `ApplicationFrameHost.exe` instead of their real process name, requiring child window enumeration as a workaround; (2) a race condition if foreground detection happens at the wrong pipeline stage -- detection must occur immediately before injection, not at recording time; (3) case-sensitive exe name matching silently breaking rule lookups. All three are solvable with known patterns and must be addressed in the first implementation phase, not retrofitted later.

## Key Findings

### Recommended Stack

No new crate dependencies. The `windows` crate v0.58 (already in Cargo.toml) covers all Win32 APIs for foreground detection and process name resolution. Process enumeration for the dropdown needs one new feature flag: `Win32_System_Diagnostics_ToolHelp`. The frontend uses existing React + Tailwind + lucide-react -- a custom `<SearchableDropdown>` component (60-80 lines) is preferred over adding a UI library for a single widget.

**Core technologies:**
- `windows` crate v0.58 (`Win32_System_Diagnostics_ToolHelp` flag): process enumeration via `CreateToolhelp32Snapshot` -- avoids adding `sysinfo` (1.2MB compiled) for a 30-line function
- `GetForegroundWindow` + `QueryFullProcessImageNameW` chain: foreground process detection at injection time -- ~0.1ms synchronous call, no async needed
- Custom `<SearchableDropdown>` React component: process selection UI -- matches existing zero-dependency component pattern, avoids `react-select` (CSS-in-JS conflict with Tailwind)

### Expected Features

**Must have (table stakes):**
- Per-app rule list with add/remove in a new "App Rules" sidebar section
- "Detect Active App" button with 3-second countdown (to avoid detecting VoiceType itself)
- Per-app ALL CAPS three-state toggle (inherit global / force ON / force OFF)
- Foreground detection at injection time with fallback to global settings
- Global ALL CAPS toggle on General page remains as the default for unlisted apps

**Should have (differentiators):**
- Searchable running-process dropdown for adding apps without the detect flow
- App icon extraction and display in the rules list
- Duplicate detection preventing the same exe from being added twice

**Defer (v2+):**
- Per-app engine/profile/corrections switching (massive scope increase)
- Window-title-based matching (fragile, locale-dependent)
- Real-time foreground monitoring (wasteful CPU, unnecessary)
- Regex/wildcard exe matching (over-engineering for 3-5 app rules)

### Architecture Approach

The system adds one new Rust module (`app_rules.rs`) containing types, managed state, foreground detection, and process enumeration. One new frontend component (`AppRulesSection.tsx`) handles the management UI. The pipeline modification is surgical: ~20 lines replacing the current direct `ActiveProfile.all_caps` read at pipeline.rs line 395 with an override resolution chain that queries foreground process -> looks up per-app rule -> falls back to global default. State is kept separate: `ActiveProfile` holds global defaults (unchanged), `AppRulesState` holds per-app overrides. No mutation of global state during override resolution.

**Major components:**
1. `app_rules.rs` (NEW) -- types (`AppOverride`, `AppRules`, `AppRulesState`), `get_foreground_process_name()`, `list_running_process_names()`, 5 Tauri commands
2. `pipeline.rs` (MODIFY) -- override resolution at line 395, replacing direct `ActiveProfile.all_caps` read with per-app lookup chain
3. `AppRulesSection.tsx` (NEW) -- rules list, detect button with countdown, three-state toggles, searchable dropdown
4. `Sidebar.tsx` + `App.tsx` (MODIFY) -- add `'app-rules'` section ID and routing

### Critical Pitfalls

1. **UWP `ApplicationFrameHost.exe` masking** -- All UWP/Store apps report as the same host process. Fix: enumerate child windows via `EnumChildWindows` to find the real app PID. Must be solved in Phase 1 before any matching logic exists.
2. **Race condition: detection timing** -- Detecting foreground app at recording start (not injection time) means alt-tabbing during transcription applies wrong formatting. Fix: detect immediately before injection. This is an architectural decision, not a bug fix -- must be correct from the start. Recovery cost is HIGH if built wrong.
3. **Case-sensitive exe name matching** -- Win32 APIs return inconsistent casing across different code paths. Fix: normalize to lowercase via a single `normalize_exe_name()` function used at every boundary (storage, detection, display).
4. **Elevated process `OpenProcess` failure** -- `PROCESS_QUERY_INFORMATION` fails on admin processes. Fix: use `PROCESS_QUERY_LIMITED_INFORMATION` from day one -- one constant change that prevents an entire failure class.
5. **Settings migration from v1.3** -- Missing `app_rules` key in existing settings files must not crash the app. Fix: null-safe reads at every consumption point, test with real v1.3 settings.json.

## Implications for Roadmap

Based on research, suggested phase structure:

### Phase 1: Foreground Detection Backend
**Rationale:** Foundation dependency -- both the "add app" UI flow and pipeline injection lookup require this module. No UI can be built until detection works. Contains the highest-stakes architectural decisions (detection timing, UWP handling, case normalization).
**Delivers:** `app_rules.rs` with types, `get_foreground_process_name()`, `AppRulesState` managed state, settings persistence, Tauri commands (`detect_foreground_app`, `list_running_apps`)
**Addresses:** Win32 foreground detection module (P1), per-app rules data model (P1), rules persistence
**Avoids:** UWP ApplicationFrameHost pitfall (#1), elevated process Access Denied (#2), case-sensitive matching (#4), NULL/self foreground window (#6), race condition timing (#3)

### Phase 2: Pipeline Integration
**Rationale:** With detection working and rules storable, the pipeline can resolve per-app overrides. This is the phase where per-app settings actually take effect. Can be tested by manually adding rules to settings.json without any UI.
**Delivers:** Modified `pipeline.rs` with override resolution chain, effective ALL CAPS based on foreground app, global fallback for unlisted apps
**Addresses:** Pipeline injection-time override (P1), global default for unlisted apps (P1)
**Avoids:** Race condition timing (#3 -- detection at injection time, not recording time), settings migration (#5 -- null-safe reads)

### Phase 3: App Rules UI
**Rationale:** Backend is complete and testable. UI can now be built against working Tauri commands. This is the user-facing phase -- sidebar section, detect button with countdown, rules list, three-state toggles.
**Delivers:** `AppRulesSection.tsx`, sidebar integration, "Detect Active App" with 3s countdown, per-app ALL CAPS toggle, remove button
**Addresses:** App Rules sidebar section (P1), Detect Active App with countdown (P1), per-app ALL CAPS toggle (P1), remove rule button (P1)
**Avoids:** Self-detection (countdown prevents detecting VoiceType's own window)

### Phase 4: Polish and Differentiators
**Rationale:** Core functionality is complete. This phase adds convenience features that improve the UX but are not required for the feature to work.
**Delivers:** Searchable process dropdown, app icon display, duplicate detection, windowed-process filtering
**Addresses:** Searchable process dropdown (P2), app icon display (P2), duplicate detection (P2)

### Phase Ordering Rationale

- Phases 1-2 form the functional backend. Phase 2 depends on Phase 1's detection module and data model. Together they deliver a working per-app override system testable without any UI.
- Phase 3 depends on Phases 1-2 for working Tauri commands. The UI is pure consumption of backend capabilities.
- Phase 4 is independent of Phases 1-3 in terms of correctness -- the feature works without it. It depends on Phase 3 for the UI surface to enhance.
- All critical pitfalls (UWP, timing, case sensitivity, elevation) are addressed in Phase 1 because they are architectural decisions with HIGH recovery cost if deferred.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 1:** UWP child window enumeration via `EnumChildWindows` -- the pattern is documented but the `windows` crate API surface for callback-based enumeration needs verification. Also verify whether `PROCESS_NAME_WIN32` constant requires additional feature flags.
- **Phase 3:** Three-state toggle UX -- confirm the cycling interaction (inherit -> ON -> OFF -> inherit) is intuitive. Consider segmented control vs. dropdown vs. cycling button.

Phases with standard patterns (skip research):
- **Phase 2:** Override resolution chain is a straightforward `Option::unwrap_or` pattern. Pipeline modification is ~20 lines of well-understood Rust.
- **Phase 4:** Searchable dropdown and icon extraction are standard UI/Win32 patterns.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All APIs verified available in existing `windows` crate v0.58. Feature flags confirmed against current Cargo.toml. No new dependencies needed. |
| Features | HIGH | Feature landscape mapped against PowerToys, f.lux, EarTrumpet. Clear table stakes vs. differentiators. Scope is well-bounded. |
| Architecture | HIGH | Integration points identified at specific line numbers in existing code. Data flow traced end-to-end. Build order is dependency-ordered. |
| Pitfalls | HIGH | UWP and elevation issues are well-documented across AutoHotkey, PowerToys, and Microsoft Learn. Race condition identified through pipeline code analysis. |

**Overall confidence:** HIGH

### Gaps to Address

- **UWP `EnumChildWindows` callback pattern in `windows` crate:** The research identifies the need but the exact Rust callback API needs verification during Phase 1 implementation. The `windows` crate uses a different callback pattern than raw C -- may need a closure wrapper.
- **`PROCESS_NAME_WIN32` vs `PROCESS_NAME_FORMAT(0)`:** STACK.md uses `PROCESS_NAME_FORMAT(0)`, ARCHITECTURE.md uses `PROCESS_NAME_WIN32`. These may be the same constant -- verify during implementation.
- **Pill window focusability:** Research flags that VoiceType's pill overlay may receive focus during detection. Need to verify `tauri.conf.json` window attributes (`skipTaskbar`, `decorations`, focusability) and add self-detection filter if needed.
- **Three-state toggle component design:** No existing pattern in the codebase. The `AllCapsToggle` is a binary toggle. The three-state interaction (inherit/ON/OFF) needs UX validation.
- **Conflicting recommendation on detection timing:** ARCHITECTURE.md recommends detection at pipeline.rs line 395 (before ALL CAPS application). PITFALLS.md recommends detection at injection time (inside inject.rs, moving ALL CAPS logic to the injection layer). Both agree detection must happen as close to injection as possible. **Recommendation: Follow ARCHITECTURE.md's approach -- detect at pipeline.rs line 395 immediately before the formatting step.** This keeps inject.rs as a pure injection function and avoids rearchitecting the pipeline. The time between line 395 and the inject call is microseconds (string formatting only), making the race condition window negligible.

## Sources

### Primary (HIGH confidence)
- [Microsoft Learn: GetForegroundWindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getforegroundwindow)
- [Microsoft Learn: Process Security and Access Rights](https://learn.microsoft.com/en-us/windows/win32/procthread/process-security-and-access-rights)
- [Microsoft Learn: OpenProcess](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-openprocess)
- [windows-docs-rs: GetForegroundWindow](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/UI/WindowsAndMessaging/fn.GetForegroundWindow.html)
- [windows-docs-rs: QueryFullProcessImageNameW](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/Threading/fn.QueryFullProcessImageNameW.html)
- [PowerToys Keyboard Manager docs](https://learn.microsoft.com/en-us/windows/powertoys/keyboard-manager)
- [Raymond Chen: OpenProcess access denied with debug privilege](https://devblogs.microsoft.com/oldnewthing/20151210-00/?p=92451)
- Existing codebase: `pipeline.rs`, `profiles.rs`, `inject.rs`, `keyboard_hook.rs`, `lib.rs`, `Sidebar.tsx`, `Cargo.toml`

### Secondary (MEDIUM confidence)
- [Tracking active process in Windows with Rust (hellocode.co)](https://hellocode.co/blog/post/tracking-active-process-windows-rust/)
- [AutoHotkey Community: ApplicationFrameHost.exe UWP detection](https://www.autohotkey.com/boards/viewtopic.php?style=7&t=112906)
- [PowerToys Issue #1766: Window Walker UWP process names](https://github.com/microsoft/PowerToys/issues/1766)
- [Enumerating Windows processes with Rust (bazizi.github.io)](https://bazizi.github.io/2022/12/29/enumerating-windows-processes-using-Rust.html)
- [f.lux forum: disable for games](https://forum.justgetflux.com/topic/2450/how-to-disable-for-current-app-for-games-full-screen)

---
*Research completed: 2026-03-07*
*Ready for roadmap: yes*
