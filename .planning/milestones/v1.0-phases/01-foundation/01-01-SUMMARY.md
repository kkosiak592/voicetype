---
phase: 01-foundation
plan: 01
subsystem: ui
tags: [tauri, rust, react, typescript, tailwind, tray, single-instance, autostart, store]

# Dependency graph
requires: []
provides:
  - Tauri 2.0 app shell that compiles and runs on Windows
  - System tray with Settings and Quit context menu
  - Settings window (hidden on startup, hides to tray on close)
  - Single-instance enforcement via tauri-plugin-single-instance
  - Plugin registrations: store, autostart, single-instance
  - React+TypeScript frontend with Tailwind CSS v4
affects: [01-02, 01-03, 02-audio, 03-whisper, 04-overlay, 05-vad, 06-ux, 07-distribution]

# Tech tracking
tech-stack:
  added:
    - tauri 2.10.2 (with tray-icon feature)
    - tauri-plugin-single-instance 2.4.0
    - tauri-plugin-store 2.4.2
    - tauri-plugin-autostart 2.5.1
    - react 18.3.x + react-dom
    - typescript 5.6.x
    - vite 6.x + @vitejs/plugin-react
    - tailwindcss 4.2.1 + @tailwindcss/vite (v4 plugin, no tailwind.config.js needed)
    - @tauri-apps/api 2.x
    - VS Build Tools 2022 (installed as dependency for MSVC toolchain)
    - Rust 1.93.1 stable-x86_64-pc-windows-msvc
  patterns:
    - Plugin registration order in tauri::Builder: single-instance FIRST, then store, autostart, then setup()
    - System tray via TrayIconBuilder in separate tray.rs module
    - Hide-to-tray via on_window_event CloseRequested + api.prevent_close() + window.hide()
    - use tauri::Manager required to call get_webview_window on AppHandle

key-files:
  created:
    - src-tauri/src/lib.rs
    - src-tauri/src/main.rs
    - src-tauri/src/tray.rs
    - src-tauri/tauri.conf.json
    - src-tauri/Cargo.toml
    - src-tauri/build.rs
    - src-tauri/capabilities/default.json
    - src-tauri/icons/32x32.png
    - src-tauri/icons/128x128.png
    - src-tauri/icons/128x128@2x.png
    - src-tauri/icons/icon.ico
    - src-tauri/icons/icon.icns
    - src-tauri/icons/tray-icon.png
    - src/main.tsx
    - src/App.tsx
    - src/styles.css
    - index.html
    - package.json
    - vite.config.ts
    - tsconfig.json
    - tsconfig.node.json
    - .gitignore
  modified: []

key-decisions:
  - "Use show_menu_on_left_click(false) not deprecated menu_on_left_click(false)"
  - "App identifier changed to com.voicetype.desktop (not .app — conflicts with macOS bundle extension)"
  - "use tauri::Manager must be imported explicitly to use get_webview_window on AppHandle"
  - "Rust installed automatically via rustup-init with --no-modify-path flag; MSVC toolchain requires VS Build Tools 2022"
  - "Tailwind CSS v4 uses @import tailwindcss in CSS and @tailwindcss/vite plugin — no tailwind.config.js needed"

patterns-established:
  - "Pattern: tray.rs module — all tray setup in its own module, called from lib.rs setup()"
  - "Pattern: Plugin chain order — single-instance > store > autostart > setup() > on_window_event"
  - "Pattern: Window hide-to-tray — CloseRequested event + prevent_close() + window.hide() for settings label"

requirements-completed: [UI-05]

# Metrics
duration: 22min
completed: 2026-02-27
---

# Phase 1 Plan 01: Tauri 2.0 App Shell Summary

**Tauri 2.0 Windows app with system tray (Settings/Quit), hide-to-tray settings window, and single-instance enforcement — compiles to .exe and .msi**

## Performance

- **Duration:** 22 min
- **Started:** 2026-02-27T15:24:10Z
- **Completed:** 2026-02-27T15:46:30Z
- **Tasks:** 1
- **Files modified:** 23

## Accomplishments

- Tauri 2.0 app scaffolded manually with React+TypeScript frontend and Tailwind CSS v4
- System tray built with TrayIconBuilder: right-click menu shows Settings and Quit, left-click does nothing
- Settings window defined in tauri.conf.json with visible: false — opens via tray, hides to tray on X
- Single-instance enforced: second launch shows and focuses existing settings window
- Three plugins registered in correct order: tauri-plugin-single-instance (first), store, autostart
- Debug build produces working .exe + .msi installer bundle

## Task Commits

Each task was committed atomically:

1. **Task 1: Scaffold Tauri 2.0 project with plugins and tray** - `0346a36` (feat)

## Files Created/Modified

- `src-tauri/src/lib.rs` - Tauri builder with plugin chain, setup() calling build_tray(), on_window_event hide-to-tray
- `src-tauri/src/tray.rs` - TrayIconBuilder with two-item menu, on_menu_event for Settings/Quit
- `src-tauri/src/main.rs` - Entry point calling voice_to_text_lib::run()
- `src-tauri/tauri.conf.json` - Window config: settings window 480x400, visible:false, identifier com.voicetype.desktop
- `src-tauri/Cargo.toml` - tauri with tray-icon feature, three plugin deps
- `src-tauri/capabilities/default.json` - core:default, store:default, autostart permissions
- `src-tauri/icons/` - Placeholder PNG icons (32x32, 128x128) and ICO/ICNS for bundling
- `src/App.tsx` - Placeholder "VoiceType Settings" heading (real UI in Plan 01-03)
- `src/styles.css` - @import "tailwindcss" (Tailwind v4 syntax)
- `vite.config.ts` - Vite with react() and tailwindcss() plugins, port 1420

## Decisions Made

- Identifier set to `com.voicetype.desktop` instead of `com.voicetype.app` — the `.app` suffix conflicts with macOS bundle extension format (Tauri warns about this)
- `show_menu_on_left_click(false)` used instead of deprecated `menu_on_left_click(false)` — API changed in Tauri 2.10.x
- `use tauri::Manager` must be explicitly imported to call `get_webview_window` on an `AppHandle` — not re-exported from tauri prelude
- Tailwind v4 confirmed: uses `@import "tailwindcss"` in CSS + `@tailwindcss/vite` Vite plugin, no `tailwind.config.js` required

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed missing Manager trait import**
- **Found during:** Task 1 (compile)
- **Issue:** `get_webview_window` not found on AppHandle — requires `use tauri::Manager` to be in scope
- **Fix:** Added `use tauri::Manager;` to both `lib.rs` and `tray.rs`
- **Files modified:** src-tauri/src/lib.rs, src-tauri/src/tray.rs
- **Verification:** Compiler error resolved; build succeeded
- **Committed in:** 0346a36 (Task 1 commit)

**2. [Rule 1 - Bug] Replaced deprecated menu_on_left_click with show_menu_on_left_click**
- **Found during:** Task 1 (compile warning)
- **Issue:** `menu_on_left_click` is deprecated in tauri-tray-icon 0.21.x; replacement is `show_menu_on_left_click`
- **Fix:** Updated method name in tray.rs
- **Files modified:** src-tauri/src/tray.rs
- **Verification:** Warning eliminated; behavior unchanged
- **Committed in:** 0346a36 (Task 1 commit)

**3. [Rule 1 - Bug] Changed app identifier from com.voicetype.app to com.voicetype.desktop**
- **Found during:** Task 1 (build warning)
- **Issue:** Tauri warns that identifiers ending in `.app` conflict with macOS bundle extension
- **Fix:** Changed identifier in tauri.conf.json
- **Files modified:** src-tauri/tauri.conf.json
- **Verification:** Warning eliminated; build succeeded
- **Committed in:** 0346a36 (Task 1 commit)

**4. [Rule 3 - Blocking] Installed Rust via rustup and VS Build Tools 2022 via winget**
- **Found during:** Task 1 (pre-build check)
- **Issue:** Rust not installed; cargo not in PATH; MSVC linker not available
- **Fix:** Downloaded rustup-init.exe, ran with `-y --no-modify-path`; installed VS Build Tools via `winget install Microsoft.VisualStudio.2022.BuildTools --override "--quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"`
- **Files modified:** System PATH (MSVC bin dir prepended for build session)
- **Verification:** `cargo tauri build --debug` completed successfully
- **Committed in:** System-level change, not tracked in repo

**5. [Rule 3 - Blocking] Fixed autostart plugin API: Builder::new().build() not init()**
- **Found during:** Task 1 (implementation review)
- **Issue:** Plan research showed `tauri_plugin_autostart::Builder::new().build()` but initial code used the older `init()` API
- **Fix:** Corrected to `tauri_plugin_autostart::Builder::new().build()` per plugin README
- **Files modified:** src-tauri/src/lib.rs
- **Verification:** Compiles without error
- **Committed in:** 0346a36 (Task 1 commit)

---

**Total deviations:** 5 auto-fixed (2 bugs, 3 blocking)
**Impact on plan:** All auto-fixes were compilation blockers or deprecation warnings. No scope creep.

## Issues Encountered

- Rust not pre-installed — automated install via rustup-init (downloaded and ran silently)
- VS Build Tools not installed — automated install via winget (MSVC linker required for x86_64-pc-windows-msvc toolchain)
- GNU toolchain attempted as alternative but failed due to missing dlltool.exe; MSVC toolchain used instead
- Git's link.exe was shadowing MSVC link.exe — resolved by prepending MSVC bin to PATH during build

## Next Phase Readiness

- App shell is complete and verified compiling
- Plan 01-02 (global hotkey) can proceed: tauri-plugin-global-shortcut ready to be added
- Plan 01-03 (settings UI) can proceed: placeholder App.tsx in place for replacement
- Note: For future builds, run with MSVC bin in PATH first or set up a `.cargo/config.toml` linker override

---
*Phase: 01-foundation*
*Completed: 2026-02-27*
