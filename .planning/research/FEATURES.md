# Feature Research: Per-App Settings with Foreground Window Detection

**Domain:** Desktop voice-to-text per-application rule management
**Researched:** 2026-03-07
**Confidence:** HIGH

---

## Context: What Is Already Built

This is a subsequent milestone (v1.4). The existing app has:

- Global ALL CAPS toggle on General page, stored in `ActiveProfile.all_caps`
- Settings persistence via Rust `SettingsState` Mutex + `settings.json`
- `store.get`/`store.set` frontend facade calling `get_setting`/`set_setting` IPC
- `AllCapsToggle` component as template for toggle UI
- Pipeline applies `all_caps` at injection time (pipeline.rs lines 396-404)
- Sidebar with 6 sections: General, Dictionary, Model, Appearance, System, History
- `inject_text()` already handles clipboard paste into the foreground app

**The new milestone (v1.4) adds:** Per-app settings overrides (starting with ALL CAPS), auto-detected by foreground window at injection time.

---

## How Comparable Desktop Tools Handle Per-App Rules

### Patterns Observed

**1. PowerToys Keyboard Manager -- "Target App" text field**
Shortcut remaps have a "Target App" column where you type the process name (e.g., `msedge`). Users find process names via `Get-Process` or Task Manager. No auto-detect, no browse dialog, no searchable dropdown. Simple but requires user to know the exe name.

**2. Windows Firewall -- Browse + installed app list**
"Add an app" shows installed applications in a scrollable list. "Browse" button opens a file picker for exe selection. No auto-detect of foreground app. Heavy, enterprise-focused UI.

**3. f.lux -- "Disable for current app" contextual action**
When an app is in the foreground, f.lux menu shows "Disable for current app" which auto-detects the foreground process. No centralized list management -- you configure each app while it is active. Users have requested a centralized list view.

**4. Windows Volume Mixer -- Auto-populated from running apps**
Shows sliders only for apps currently producing audio. No manual add. Apps appear/disappear dynamically. Not suitable for persistent rules.

**5. EarTrumpet -- Per-app sliders with icons**
Shows app icon + name + slider for each running process. Good visual identification. Still runtime-only, no persistent rules.

### What Works for VoiceType's Use Case

VoiceType needs **persistent rules** (not runtime-only) with **auto-detection at injection time** (not just at configuration time). The best UX combines:

- **f.lux's "detect current app" approach** for adding apps (user activates the target app, clicks "Detect", app gets added)
- **PowerToys' persistent rule list** for managing configured apps with overrides
- **EarTrumpet's app icon + name pattern** for visual identification in the list

---

## Feature Landscape

### Table Stakes (Users Expect These)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Per-app rule list with add/remove | Core feature. Without a visible list of configured apps, users cannot manage overrides. Every per-app settings system shows a list of configured items. | MEDIUM | New "App Rules" sidebar section. List of `{exe_name, display_name, overrides}` entries persisted to settings.json |
| "Detect Active App" button | Users should not need to manually type `BLUEBEAM.exe`. f.lux proves that "detect current app" is the intuitive way. Click button -> switch to target app -> detection captures it. | MEDIUM | Backend: `GetForegroundWindow` + `GetWindowThreadProcessId` + `QueryFullProcessImageNameW` -> extract exe name. Frontend: timer-based polling or single-shot after delay |
| Per-app ALL CAPS toggle | The stated v1.4 use case. Must override the global default from General page. Three states per app: inherit global (default) / force ON / force OFF. | LOW | Toggle per rule entry. Pipeline reads foreground app at injection time, looks up rule, applies override |
| Foreground detection at injection time | The whole point -- auto-apply the right settings based on which app receives the text. Must happen in `run_pipeline` before text formatting. | MEDIUM | Win32 API call in pipeline.rs, just before ALL CAPS application. Match exe name against rules list |
| Global default for unlisted apps | Users expect the global ALL CAPS toggle to still work as the fallback. Apps not in the rules list should use the global setting. | LOW | Existing `ActiveProfile.all_caps` becomes the default. Per-app rule overrides only when a match exists |
| Remove app from list | Must be able to delete rules. Every list-based settings UI has this. | LOW | Delete button per row, confirm not needed for single items |

### Differentiators (Competitive Advantage)

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Searchable running-process dropdown | Faster than "detect active app" when you already know the app name. Shows currently running processes with display names. PowerToys forces users to look up process names manually. | MEDIUM | `EnumProcesses` + `OpenProcess` + `QueryFullProcessImageNameW` for running processes. Combobox with filter-as-you-type. Show display name + exe name |
| Three-state toggle (inherit / ON / OFF) | More expressive than binary. "Inherit" means the global setting controls this app. ON/OFF explicitly override. Users can set global=ON but force OFF for VS Code (where lowercase is preferred for code). | LOW | Segmented control or dropdown with three options. Default: inherit |
| App icon display in rules list | Visual identification makes the list scannable. Users recognize app icons faster than reading exe names. | LOW | Extract icon from exe path using `ExtractIconExW` or `SHGetFileInfo`. Display as 16x16 or 24x24 in list |
| Detection countdown UX | After clicking "Detect Active App", show "Switch to target app... detecting in 3s" countdown. Gives user time to alt-tab. Better than immediate detection (which captures the settings window itself). | LOW | 3-second countdown timer in frontend. After countdown, call backend `detect_foreground_app` command |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Auto-populate rules for all running apps | "Show me everything" | Creates a huge list of irrelevant processes (svchost, RuntimeBroker, etc.). Volume Mixer can do this because audio is the filter. VoiceType has no natural filter for which apps matter. | Start with empty list. Users add only apps they actually dictate into |
| Window-title-based matching | "Different behavior for different Chrome tabs" | Window titles change constantly, are locale-dependent, and break on updates. Process exe name is stable and unambiguous. | Match on exe name only. If a user needs different behavior per Chrome tab, that is a different product |
| Real-time foreground monitoring with live indicator | "Show me which app is active right now in settings" | Continuous polling wastes CPU. Desktop tools that need this (window managers) use shell hooks which add complexity. VoiceType only needs detection at two points: when adding a rule, and at injection time. | Detect only when needed: on "Detect" button click and at injection time |
| Per-app profile switching (engine, corrections, vocabulary) | "Different whisper model per app" | Massive scope increase. Engine switching has startup cost (model loading). Profile switching affects corrections state. Creates a combinatorial explosion of settings. | Start with ALL CAPS only. The architecture should support adding more per-app overrides later, but v1.4 ships only ALL CAPS |
| Regex or wildcard matching for exe names | "Match all Chrome variants" | Over-engineering. Users have 3-5 apps they dictate into. Exact exe match is sufficient. | Exact exe name match. If `chrome.exe` vs `msedge.exe` matters, add both |
| Browse for exe file picker | "Let me find the exe on disk" | Users rarely know where executables live. `C:\Program Files\...` browsing is painful. The "Detect Active App" flow and searchable process dropdown cover all use cases. | Detect Active App button + searchable running process dropdown |

---

## Feature Dependencies

```
[Foreground window detection (Rust/Win32)]
    |
    |--used-by--> [Detect Active App button (add-flow)]
    |--used-by--> [Pipeline injection-time lookup]
    |
    +--requires--> [GetForegroundWindow + GetWindowThreadProcessId + QueryFullProcessImageNameW]

[Per-app rules data model]
    |
    |--stored-in--> [settings.json via SettingsState]
    |--read-by---> [Pipeline at injection time]
    |--managed-by-> [App Rules sidebar section]

[App Rules sidebar section]
    |--requires--> [Per-app rules data model]
    |--requires--> [Foreground window detection]
    |--uses------> [Searchable process dropdown (optional, differentiator)]

[Pipeline per-app override]
    |--requires--> [Per-app rules data model]
    |--requires--> [Foreground window detection]
    |--modifies--> [ALL CAPS application logic in pipeline.rs]
    |--falls-back-> [Global ActiveProfile.all_caps]

[Searchable process dropdown] --enhances--> [App Rules sidebar section]
[App icon display] --enhances--> [App Rules sidebar section]
[Detection countdown UX] --enhances--> [Detect Active App button]
```

### Dependency Notes

- **Foreground detection is the shared foundation.** Both the "add app" flow and the injection-time lookup need the same Win32 API calls. Build this Rust module first.
- **Data model must exist before UI or pipeline changes.** The rules structure in settings.json determines what the UI displays and what the pipeline reads.
- **Pipeline override is independent of the UI.** Even with no UI, if rules exist in settings.json, the pipeline should respect them. This enables testing the pipeline logic before the UI is complete.
- **Searchable dropdown requires process enumeration**, which is a superset of foreground detection (enumerate all vs detect one). Can be deferred if needed.

---

## Scope Definition for v1.4

### Must Do (v1.4)

- [ ] Win32 foreground window detection module (Rust) -- `GetForegroundWindow` + process name extraction
- [ ] Per-app rules data model -- `Vec<AppRule>` with `{exe_name, display_name, all_caps_override: Option<bool>}`
- [ ] Rules persistence in settings.json via SettingsState
- [ ] New "App Rules" sidebar section with rules list
- [ ] "Detect Active App" button with countdown UX (3s delay for alt-tab)
- [ ] Per-app ALL CAPS three-state toggle (inherit / ON / OFF)
- [ ] Pipeline injection-time lookup -- detect foreground app, match against rules, override ALL CAPS
- [ ] Remove button per rule entry
- [ ] Global ALL CAPS on General page remains as default for unlisted apps

### Should Do (v1.4 if time permits)

- [ ] Searchable running-process dropdown for adding apps without the detect flow
- [ ] App icon extraction and display in rules list
- [ ] Duplicate detection -- prevent adding the same exe twice

### Do Not Do (v1.4)

- [ ] Per-app engine/profile/corrections switching -- future milestone
- [ ] Window-title matching -- fragile, unnecessary
- [ ] Real-time foreground monitoring -- wasteful
- [ ] Auto-populated process list -- noisy
- [ ] Regex/wildcard exe matching -- over-engineering

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Win32 foreground detection module | HIGH (foundation) | MEDIUM | P1 |
| Per-app rules data model + persistence | HIGH (foundation) | LOW | P1 |
| App Rules sidebar section | HIGH (user-facing) | MEDIUM | P1 |
| Detect Active App with countdown | HIGH (primary add flow) | LOW | P1 |
| Per-app ALL CAPS toggle | HIGH (core feature) | LOW | P1 |
| Pipeline injection-time override | HIGH (core behavior) | MEDIUM | P1 |
| Remove rule button | MEDIUM | LOW | P1 |
| Searchable process dropdown | MEDIUM (convenience) | MEDIUM | P2 |
| App icon display | LOW (polish) | LOW | P2 |
| Duplicate detection | LOW (edge case) | LOW | P2 |

**Priority key:**
- P1: Must have for v1.4
- P2: Should have, add if time permits
- P3: Future consideration

---

## Competitor Feature Analysis

| Behavior | PowerToys KBM | f.lux | Windows Volume Mixer | VoiceType v1.4 (planned) |
|----------|--------------|-------|---------------------|--------------------------|
| Per-app rules | Yes (shortcuts only) | Yes (disable per app) | Runtime only | Yes (persistent) |
| How apps are added | Manual exe name typing | "Disable for current app" auto-detect | Auto from running apps | Detect Active App button + process dropdown |
| Rule persistence | Yes, in JSON config | Yes | No (runtime only) | Yes, in settings.json |
| Rule management UI | Inline table rows | No centralized list (per f.lux forum complaints) | N/A | Dedicated sidebar section with list |
| Override granularity | Key/shortcut remapping | Binary (enabled/disabled) | Volume level | Three-state per setting (inherit/ON/OFF) |
| Foreground detection | At keypress time | At menu open time | Continuous monitoring | At injection time + on detect button |
| App identification | Process name only | Process name + display name | Process name + icon | Exe name + display name + icon (P2) |

---

## Sources

- [PowerToys Keyboard Manager docs (Microsoft Learn)](https://learn.microsoft.com/en-us/windows/powertoys/keyboard-manager) -- Target App field uses exe process name, no auto-detect (HIGH confidence)
- [PowerToys KBM per-app remap issue #6756](https://github.com/microsoft/PowerToys/issues/6756) -- App-specific remapping feature requests and limitations (HIGH confidence)
- [f.lux FAQ](https://justgetflux.com/faq.html) -- "Disable for current app" auto-detects foreground process (HIGH confidence)
- [f.lux forum: disable for games](https://forum.justgetflux.com/topic/2450/how-to-disable-for-current-app-for-games-full-screen) -- Users want centralized per-app list (MEDIUM confidence)
- [GetForegroundWindow Win32 API (Microsoft Learn)](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getforegroundwindow) -- Official API for foreground window handle (HIGH confidence)
- [windows-docs-rs: GetForegroundWindow](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/UI/WindowsAndMessaging/fn.GetForegroundWindow.html) -- Rust bindings via windows crate (HIGH confidence)
- [Tracking active process in Windows with Rust (hellocode.co)](https://hellocode.co/blog/post/tracking-active-process-windows-rust/) -- Complete Rust pattern for GetForegroundWindow + PID + process name (HIGH confidence)
- [Tauri issue #4827: access system's active window](https://github.com/tauri-apps/tauri/issues/4827) -- Confirms no built-in Tauri API, use windows-rs directly (HIGH confidence)

---
*Feature research for: VoiceType v1.4 Per-App Settings*
*Researched: 2026-03-07*
