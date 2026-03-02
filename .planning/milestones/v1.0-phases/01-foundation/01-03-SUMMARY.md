---
phase: 01-foundation
plan: 03
subsystem: ui
tags: [tauri, react, typescript, tailwind, store, hotkey, theme, autostart, settings]

# Dependency graph
requires:
  - phase: 01-01
    provides: Tauri 2.0 app shell with plugin registrations
  - phase: 01-02
    provides: rebind_hotkey command and global-shortcut plugin
provides:
  - Settings UI: hotkey capture, theme toggle, autostart toggle
  - tauri-plugin-store wired to persist all settings to settings.json
  - Hotkey restored from disk on startup (read_saved_hotkey in setup())
  - Dark mode applied via .dark class on html element (Tailwind v4 class strategy)
affects: [02-audio-whisper, 04-overlay, 06-ux]

# Tech tracking
tech-stack:
  added:
    - "@tauri-apps/plugin-store (npm, already installed)"
    - "@tauri-apps/plugin-autostart (npm, already installed)"
  patterns:
    - Store singleton pattern: getStore() lazy-loads Store.load('settings.json') with defaults and autoSave:100
    - Dark mode: apply/remove .dark on document.documentElement — no prefers-color-scheme (broken in Tauri WebView per research #5802)
    - HotkeyCapture: keydown event listener on window with capture:true, normalizes e.code to tauri shortcut format
    - Hotkey restore on startup: read_saved_hotkey() reads settings.json directly with serde_json — synchronous, works in setup()
    - Tailwind v4 dark mode: @variant dark (&:where(.dark, .dark *)) in styles.css

key-files:
  created:
    - src/lib/store.ts
    - src/components/HotkeyCapture.tsx
    - src/components/ThemeToggle.tsx
    - src/components/AutostartToggle.tsx
  modified:
    - src/App.tsx
    - src/styles.css
    - src-tauri/src/lib.rs

key-decisions:
  - "Hotkey restore reads settings.json via std::fs + serde_json — no async Rust store API needed for one-time startup read"
  - "Tailwind v4 dark mode uses @variant dark directive in CSS, not darkMode config key — v4 has no tailwind.config.js"
  - "prefers-color-scheme intentionally avoided — broken in Tauri WebView (issue #5802), class-based toggle used instead"
  - "HotkeyCapture uses e.code for key normalization (not e.key) — layout-independent, maps directly to tauri shortcut strings"
  - "rebind_hotkey invoke parameter named new_key matches #[tauri::command] fn parameter new_key — Tauri snake_case to camelCase mapping"

# Metrics
duration: 4min
completed: 2026-02-27
---

# Phase 1 Plan 03: Settings UI Summary

**Settings window with hotkey rebinding (HotkeyCapture), dark mode toggle, and autostart toggle — all persisted via tauri-plugin-store with hotkey restored from disk on startup**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-02-27T15:57:25Z
- **Completed:** 2026-02-27T16:00:46Z
- **Tasks:** 2 completed (1 auto + 1 human-verify checkpoint — all 13 checks passed)
- **Files modified:** 7

## Accomplishments

- `src/lib/store.ts`: Store singleton wrapping tauri-plugin-store with typed AppSettings interface and auto-save
- `src/components/HotkeyCapture.tsx`: Click-to-capture hotkey box — enters listening mode, captures modifier+key, normalizes via e.code, invokes rebind_hotkey, writes to store
- `src/components/ThemeToggle.tsx`: Toggle switch applying/removing .dark class on html element, persists to store
- `src/components/AutostartToggle.tsx`: Toggle switch using plugin-autostart isEnabled/enable/disable JS API with loading state
- `src/App.tsx`: Full settings page with Hotkey/Appearance/Startup sections; loads saved settings on mount, applies theme to DOM
- `src/styles.css`: Tailwind v4 dark mode via `@variant dark` directive (class strategy)
- `src-tauri/src/lib.rs`: `read_saved_hotkey()` reads settings.json at startup via serde_json; registers saved hotkey (or default) in setup()
- `cargo tauri build --debug` succeeds, producing .exe + .msi + .nsis installer bundles

## Task Commits

Each task was committed atomically:

1. **Task 1: Settings store, hotkey restore on startup, and settings UI** - `ba52dee` (feat)
2. **Task 2: Verify full Phase 1 functionality** - Checkpoint approved — all 13 verification checks passed (no code commit)

## Files Created/Modified

- `src/lib/store.ts` - Store.load singleton with AppSettings type, DEFAULTS export, autoSave:100
- `src/components/HotkeyCapture.tsx` - Hotkey capture box with listening state, key normalization, rebind_hotkey invoke
- `src/components/ThemeToggle.tsx` - Dark/light toggle switch with .dark class manipulation
- `src/components/AutostartToggle.tsx` - Autostart toggle using plugin-autostart JS API
- `src/App.tsx` - Settings page: load settings on mount, render three sections with controls
- `src/styles.css` - Added @variant dark for Tailwind v4 class-based dark mode
- `src-tauri/src/lib.rs` - Added read_saved_hotkey(), hotkey variable in setup(), logs registered hotkey

## Decisions Made

- `read_saved_hotkey()` reads the store JSON file directly via `std::fs::read_to_string` + `serde_json` — the tauri-plugin-store Rust API requires async context which is not available in synchronous `setup()`; direct file read is the plan's recommended fallback and works correctly
- Tailwind v4 dark mode configured via `@variant dark (&:where(.dark, .dark *))` in CSS — v4 has no tailwind.config.js, the `@variant` directive is the correct equivalent of `darkMode: 'class'`
- `e.code` used for key normalization in HotkeyCapture (not `e.key`) — keyboard-layout independent and maps cleanly to tauri shortcut format (e.g., "KeyA" -> "a", "Space" -> "space")
- Store `autoSave: 100` (100ms debounce) — immediate writes not needed, 100ms is imperceptible and reduces disk I/O

## Deviations from Plan

None - plan executed exactly as written.

## Checkpoint Outcome

**Task 2 (human-verify):** All 13 verification checks passed. Phase 1 success criteria fully satisfied.

---

## Self-Check: PASSED

- src/lib/store.ts: FOUND
- src/components/HotkeyCapture.tsx: FOUND
- src/components/ThemeToggle.tsx: FOUND
- src/components/AutostartToggle.tsx: FOUND
- src/App.tsx: FOUND (modified)
- src/styles.css: FOUND (modified)
- src-tauri/src/lib.rs: FOUND (modified)
- .planning/phases/01-foundation/01-03-SUMMARY.md: FOUND
- Commit ba52dee: FOUND

---
*Phase: 01-foundation*
*Completed: 2026-02-27*
