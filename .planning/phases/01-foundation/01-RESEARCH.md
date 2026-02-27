# Phase 1: Foundation - Research

**Researched:** 2026-02-27
**Domain:** Tauri 2.0 — system tray, global hotkey, settings persistence, window lifecycle
**Confidence:** HIGH (all major claims verified via Context7 official docs or official Tauri GitHub)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Default hotkey & rebinding**
- Default hotkey: Ctrl+Shift+Space
- Rebinding via key capture UI — click a box, press desired combo, it captures it
- Hotkey swap is immediate — old combo unregisters, new one registers instantly, no restart or Apply button needed
- Changed hotkey persists to settings store immediately on capture

**System tray**
- Right-click menu: Settings and Quit (two items only for Phase 1)
- Left-click on tray icon does nothing
- Tray icon: microphone silhouette

**Settings window**
- Theme toggle in settings: light and dark mode
- Default theme: light
- Theme preference persists across restarts

**App lifecycle**
- No auto-start with Windows by default; toggle available in settings
- Closing the settings window minimizes to tray (app keeps running)
- Single instance enforced — second launch focuses the existing instance
- Quit only via tray menu > Quit

### Claude's Discretion
- Settings window layout (single page vs sidebar tabs) — pick what fits the Phase 1 settings count
- Settings window size (fixed vs resizable)
- Tray icon state planning (whether to design for future state-aware icons now or defer)
- Hotkey conflict validation approach
- Startup notification behavior (silent vs brief toast)

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| CORE-01 | User can activate voice recording via a system-wide global hotkey from any application | tauri-plugin-global-shortcut — register/unregister API verified; no-focus-steal behavior is the OS default for global shortcuts |
| UI-05 | App runs in the system tray with a context menu (Settings, Quit, version info) | TrayIconBuilder + Menu::with_items verified in Context7 official docs; tray-icon feature flag required in Cargo.toml |
| SET-02 | User can configure the global hotkey binding (choose any key or key combo) | unregister old + register new pattern verified; key capture UI via keydown event in frontend |
| SET-05 | Settings persist across app restarts (tauri-plugin-store) | Store.load() / store.set() / store.get() verified; store:default capability permission required |
</phase_requirements>

---

## Summary

Phase 1 sets up the Tauri 2.0 app shell with no audio or transcription — just the container that all later phases build on. The three plans (scaffold, global hotkey, settings persistence) map cleanly to three independent plugin integrations that are well-documented and have HIGH confidence.

The **global hotkey plugin** (`tauri-plugin-global-shortcut`) is the Tauri-official solution and supports runtime registration and unregistration, which directly satisfies the live-rebind requirement. The **store plugin** (`tauri-plugin-store`) provides JSON-backed key-value persistence with auto-save and defaults. The **single-instance plugin** (`tauri-plugin-single-instance`) and **autostart plugin** (`tauri-plugin-autostart`) are both official and trivially integrated via `cargo tauri add`.

One **medium-confidence** area is window close interception for "hide to tray": the `CloseRequested` + `api.prevent_close()` + `window.hide()` pattern is widely used but has a reported macOS bug (closed as not-planned). On Windows — the only target platform — `prevent_close()` is the expected working approach. The alternative `ExitRequested` + `api.prevent_exit()` pattern also works but operates at the app level rather than per-window. Use `CloseRequested` / `prevent_close()` first; if it fails on Windows, fall back to `ExitRequested`.

**Primary recommendation:** Scaffold with `npm create tauri-app@latest`, add all four official plugins via `cargo tauri add`, wire everything in Rust `setup()`, expose hotkey rebind via Tauri commands.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tauri | 2.x | App framework | Official; ships WebView + IPC + tray |
| tauri-plugin-global-shortcut | 2.3.1 | System-wide hotkey registration | Official Tauri plugin; desktop-only guard built in |
| tauri-plugin-store | 2.x | Settings persistence to JSON on disk | Official Tauri plugin; JS + Rust interop; auto-save debounce |
| tauri-plugin-single-instance | 2.x | Prevent second launch; focus existing | Official Tauri plugin; one-liner integration |
| tauri-plugin-autostart | 2.x | Windows startup toggle | Official Tauri plugin; exposes enable/disable/isEnabled |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| React (or Svelte) | current | Settings window UI | Already chosen at project init via create-tauri-app |
| Tailwind CSS | 3.x | Utility styling for settings window | Recommended for simple single-page settings with few controls |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| tauri-plugin-store | serde_json + fs::write directly | Store plugin adds JS interop and change listeners; only use raw fs if you never need JS access |
| tauri-plugin-global-shortcut | global-hotkey crate directly | The plugin wraps global-hotkey and adds IPC bridge; use the plugin unless you need lower-level control |
| tauri-plugin-single-instance | Manual DBus/NSIS logic | Plugin handles Windows named-pipe detection automatically |

**Installation:**
```bash
# Create project
npm create tauri-app@latest

# Add plugins (run from project root)
cargo tauri add global-shortcut
cargo tauri add store
cargo tauri add single-instance
cargo tauri add autostart
```

---

## Architecture Patterns

### Recommended Project Structure
```
src-tauri/
├── src/
│   ├── lib.rs          # tauri::Builder::default() setup — plugins, tray, window events, commands
│   └── main.rs         # entry point (calls lib.rs run())
├── icons/              # app icon + tray icon (PNG/ICO)
├── capabilities/
│   └── default.json    # plugin permissions
└── tauri.conf.json     # window definitions, app metadata

src/
├── main.tsx            # React entry; route "/" = settings page
├── Settings.tsx        # hotkey capture + theme toggle + autostart toggle
└── styles/             # Tailwind or plain CSS
```

### Pattern 1: Plugin Setup in `setup()` Closure

All plugin initialization happens in one `setup()` closure wired to `tauri::Builder`. This keeps the `main.rs` entry point minimal.

```rust
// Source: https://github.com/tauri-apps/tauri-plugin-global-shortcut (README, verified Context7)
use tauri::Emitter;
use tauri_plugin_global_shortcut::{Builder as ShortcutBuilder, Code, Modifiers, ShortcutState};

tauri::Builder::default()
    .plugin(tauri_plugin_store::Builder::default().build())
    .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        // Second instance launched — focus existing settings window
        let _ = app.get_webview_window("settings")
                   .map(|w| { let _ = w.show(); let _ = w.set_focus(); });
    }))
    .plugin(tauri_plugin_autostart::Builder::new().build())
    .setup(|app| {
        // Global shortcut
        #[cfg(desktop)]
        app.handle().plugin(
            ShortcutBuilder::new()
                .with_shortcuts(["ctrl+shift+space"])?
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        let _ = app.emit("hotkey-triggered", ());
                    }
                })
                .build(),
        )?;

        // System tray
        build_tray(app)?;
        Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error running tauri application");
```

### Pattern 2: System Tray with Two-Item Menu

```rust
// Source: https://github.com/tauri-apps/tauri-docs (v2, system-tray.mdx, verified Context7)
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let settings_i = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit_i     = MenuItem::with_id(app, "quit",     "Quit",     true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings_i, &quit_i])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .menu_on_left_click(false)   // left-click does nothing per spec
        .on_menu_event(|app, event| match event.id.as_ref() {
            "settings" => {
                if let Some(w) = app.get_webview_window("settings") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
```

### Pattern 3: Hide Settings Window on Close (hide to tray)

```rust
// Source: tauri-apps/tauri GitHub (on_window_event CloseRequested pattern, verified working on Windows)
// Note: prevent_close() has a reported macOS bug (issue #12334, closed not-planned).
// Windows target only — this is safe.
.on_window_event(|window, event| {
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        if window.label() == "settings" {
            window.hide().unwrap();
            api.prevent_close();
        }
    }
})
```

### Pattern 4: Live Hotkey Rebinding via Tauri Command

```rust
// Source: tauri-plugin-global-shortcut docs (unregister + re-register pattern)
use tauri_plugin_global_shortcut::GlobalShortcutExt;

#[tauri::command]
fn rebind_hotkey(app: tauri::AppHandle, old: String, new: String) -> Result<(), String> {
    let gs = app.global_shortcut();
    if !old.is_empty() {
        gs.unregister(old.as_str()).map_err(|e| e.to_string())?;
    }
    gs.register(new.as_str()).map_err(|e| e.to_string())?;
    Ok(())
}
```

Frontend calls `invoke('rebind_hotkey', { old: currentHotkey, new: capturedHotkey })` immediately after key capture and then writes the new value to the store.

### Pattern 5: Settings Store — Read/Write

```typescript
// Source: tauri-apps/plugins-workspace (store README, verified Context7)
import { Store } from '@tauri-apps/plugin-store';

const store = await Store.load('settings.json', {
  defaults: { hotkey: 'ctrl+shift+space', theme: 'light', autostart: false },
  autoSave: 100 // debounce ms
});

// Read on startup
const hotkey = await store.get<string>('hotkey') ?? 'ctrl+shift+space';

// Write immediately after change
await store.set('hotkey', newHotkey);
await store.set('theme', 'dark');
```

### Pattern 6: Key Capture UI

No Tauri-specific library needed. Use browser `keydown` event on a focused input-like element:

```typescript
function onKeyDown(e: KeyboardEvent) {
  e.preventDefault();
  const mods: string[] = [];
  if (e.ctrlKey)  mods.push('ctrl');
  if (e.shiftKey) mods.push('shift');
  if (e.altKey)   mods.push('alt');
  // Exclude modifier-only keystrokes
  if (!['Control','Shift','Alt','Meta'].includes(e.key)) {
    const key = e.code.replace('Key','').replace('Digit','').toLowerCase();
    const combo = [...mods, key].join('+');
    setCapturing(combo);
    invoke('rebind_hotkey', { old: currentHotkey, new: combo });
  }
}
```

Key format must match what `tauri-plugin-global-shortcut` expects: `ctrl+shift+space`, `alt+f4` (lowercase, `+`-separated). Verify against the plugin's accepted key names before committing a format.

### Pattern 7: Theme Toggle (CSS class approach)

```typescript
// Apply theme via CSS class on documentElement; store preference
async function setTheme(theme: 'light' | 'dark') {
  document.documentElement.classList.toggle('dark', theme === 'dark');
  await store.set('theme', theme);
}

// On app load, restore
const saved = await store.get<string>('theme') ?? 'light';
setTheme(saved as 'light' | 'dark');
```

**Critical:** Do NOT rely on `prefers-color-scheme` CSS media query to toggle themes — Tauri's WebView propagation of window theme to `prefers-color-scheme` is broken (issue #5802 open). Always drive theme via an explicit CSS class.

### Anti-Patterns to Avoid

- **Using `window.close()` after `prevent_close()`:** After intercepting `CloseRequested`, calling `window.close()` again will not work. Use `window.destroy()` if you need to truly close programmatically.
- **Relying on `tauri.conf.json` `theme` field:** The `theme` field on the window config does not propagate `prefers-color-scheme` into the WebView reliably on Windows. Drive theme via CSS class from JS.
- **Registering shortcuts outside `#[cfg(desktop)]`:** The global shortcut plugin is desktop-only. Always guard with `#[cfg(desktop)]` or `#[cfg(not(any(target_os = "android", target_os = "ios")))]`.
- **Calling `window.hide()` without `prevent_close()`:** On Windows, hiding without preventing close may still destroy the window. Always pair them.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Settings persistence | Custom file I/O with serde_json | `tauri-plugin-store` | Store handles write-on-crash safety, JS/Rust interop, change listeners, schema defaults |
| Global hotkey | Win32 RegisterHotKey via raw Rust FFI | `tauri-plugin-global-shortcut` | Plugin wraps `global-hotkey` crate with Tauri IPC; handles key parsing, conflict detection, unregister |
| Single instance | Named pipe or registry check | `tauri-plugin-single-instance` | Plugin handles Windows named pipe + focus callback correctly |
| Windows autostart | Registry key writes | `tauri-plugin-autostart` | Plugin abstracts HKCU\Software\Microsoft\Windows\CurrentVersion\Run correctly across Windows versions |

**Key insight:** All four problems have official Tauri plugins with tested Windows implementations. The time cost of hand-rolling any of them (especially single-instance on Windows with race conditions) far exceeds the time to integrate the plugins.

---

## Common Pitfalls

### Pitfall 1: `tray-icon` Feature Flag Missing

**What goes wrong:** Tray icon API silently unavailable; compiler error referencing `TrayIconBuilder` not found.
**Why it happens:** Tray support is an opt-in Cargo feature in Tauri 2.0.
**How to avoid:** Ensure `Cargo.toml` contains:
```toml
tauri = { version = "2.0.0", features = ["tray-icon"] }
```
**Warning signs:** `cargo tauri add` does not add this automatically — verify after scaffolding.

### Pitfall 2: Capabilities Not Configured for Plugins

**What goes wrong:** Plugin JS calls fail at runtime with "not allowed" errors; no compile-time warning.
**Why it happens:** Tauri 2.0 requires explicit permission grants in `src-tauri/capabilities/default.json`.
**How to avoid:** For this phase, `default.json` must include:
```json
{
  "permissions": [
    "global-shortcut:allow-is-registered",
    "global-shortcut:allow-register",
    "global-shortcut:allow-unregister",
    "store:default",
    "autostart:allow-enable",
    "autostart:allow-disable",
    "autostart:allow-is-enabled"
  ]
}
```
**Warning signs:** JS-side `invoke` or plugin calls return permission errors in the browser devtools console.

### Pitfall 3: Hotkey String Format Mismatch

**What goes wrong:** `register('Ctrl+Shift+Space')` fails silently or throws a parse error; captured key combo from the frontend doesn't match the plugin's expected format.
**Why it happens:** The plugin expects lowercase modifiers joined by `+` (e.g., `ctrl+shift+space`), but `KeyboardEvent.code` returns `Space`, `KeyA`, `Digit1`.
**How to avoid:** Normalize the key string in the capture handler before passing to `register()`. Test the exact string format against the plugin's accepted values on first integration.
**Warning signs:** `register()` returns an error or the shortcut fires on wrong keys.

### Pitfall 4: Settings Window Not Hidden — App Exits

**What goes wrong:** User closes settings window, app exits entirely instead of minimizing to tray.
**Why it happens:** Default Tauri behavior exits the app when the last window closes.
**How to avoid:** Wire `on_window_event` with `CloseRequested` + `api.prevent_close()` + `window.hide()` for the settings window label. The `ExitRequested` + `api.prevent_exit()` pattern is a backup if `CloseRequested` misbehaves.
**Warning signs:** App disappears from taskbar and tray simultaneously when settings closed.

### Pitfall 5: Single-Instance Plugin Order

**What goes wrong:** Single-instance plugin doesn't focus existing window; second launch opens second settings window.
**Why it happens:** Plugin must be registered BEFORE `setup()` in the builder chain.
**How to avoid:** Call `.plugin(tauri_plugin_single_instance::init(...))` before `.setup(|app| { ... })`.
**Warning signs:** Multiple instances can be launched from the taskbar.

### Pitfall 6: Dark Mode via `prefers-color-scheme` Broken

**What goes wrong:** Theme toggle in settings appears to work but the stored preference has no effect on next app launch; or OS dark mode overrides the user's choice.
**Why it happens:** Tauri's WebView on Windows does not correctly propagate the window `theme` config to the CSS `prefers-color-scheme` media query (open issue #5802).
**How to avoid:** Implement theme entirely through a CSS class (e.g., `.dark` on `<html>`), driven from JS reading the store on startup. Never rely on the media query.

---

## Code Examples

Verified patterns from official sources:

### Full Plugin Registration Order (`lib.rs`)

```rust
// Source: Context7 tauri-plugin-global-shortcut README + plugins-workspace store README
pub fn run() {
    tauri::Builder::default()
        // Order matters: single-instance must be first
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("settings") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .setup(|app| {
            #[cfg(desktop)]
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_shortcuts(["ctrl+shift+space"])?
                    .with_handler(|app, _shortcut, event| {
                        use tauri_plugin_global_shortcut::ShortcutState;
                        if event.state == ShortcutState::Pressed {
                            let _ = app.emit("hotkey-triggered", ());
                        }
                    })
                    .build(),
            )?;
            build_tray(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "settings" {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Store Read on Startup + Apply Hotkey

```typescript
// Source: Context7 plugins-workspace store docs
import { Store } from '@tauri-apps/plugin-store';
import { register } from '@tauri-apps/plugin-global-shortcut';

const store = await Store.load('settings.json', {
  defaults: { hotkey: 'ctrl+shift+space', theme: 'light', autostart: false },
  autoSave: 100,
});

const hotkey = await store.get<string>('hotkey') ?? 'ctrl+shift+space';
await register(hotkey, (event) => {
  if (event.state === 'Pressed') {
    console.log('Hotkey triggered');
  }
});
```

### Tauri Window Config for Settings Window Only

```json
// Source: Context7 tauri-docs (Configure Multiple Windows)
// tauri.conf.json — define settings window; pill window deferred to Phase 4
{
  "app": {
    "windows": [
      {
        "label": "settings",
        "title": "VoiceType Settings",
        "width": 480,
        "height": 400,
        "resizable": false,
        "visible": false
      }
    ]
  }
}
```

Note: `visible: false` on startup — tray icon is the entry point. Settings opens via tray menu.

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Tauri 1.x `allowlist` in tauri.conf.json | Tauri 2.x capabilities JSON in `src-tauri/capabilities/` | Tauri 2.0 stable (2024) | All plugin permissions must be explicitly granted; no implicit allowlist |
| Tauri 1.x `systemTray` config key | Tauri 2.x `TrayIconBuilder` in Rust / `TrayIcon.new()` in JS | Tauri 2.0 | Tray is now fully programmatic; no static config |
| `tauri::GlobalShortcutManager` (Tauri 1.x) | `tauri_plugin_global_shortcut` (Tauri 2.x) | Tauri 2.0 | Plugin-based; requires separate dependency |
| `tauri::WindowBuilder` (Tauri 1.x) | `tauri::WebviewWindowBuilder` (Tauri 2.x) | Tauri 2.0 | API renamed; same conceptual model |

**Deprecated/outdated:**
- Tauri 1.x `on_system_tray_event`: replaced by `TrayIconBuilder::on_menu_event` + `on_tray_icon_event`
- Any blog post or Stack Overflow answer referencing `systemTray` in `tauri.conf.json`: that's Tauri 1.x; ignore

---

## Open Questions

1. **Exact key string format for `tauri-plugin-global-shortcut`**
   - What we know: examples show `"ctrl+shift+space"` and `"CommandOrControl+Shift+C"` in docs
   - What's unclear: full list of accepted key names (is it `space` or `Space`? `ctrl` or `Control`?); the plugin's key parser is in the `keyboard-types` crate
   - Recommendation: On first integration, log the `shortcut.into_string()` of a manually registered shortcut to learn the canonical format before wiring the rebind UI

2. **`prevent_close()` reliability on Windows for dynamically created windows**
   - What we know: issue #8435 reports `CloseRequested` cannot prevent close on dynamically created windows; settings window here is defined statically in `tauri.conf.json`
   - What's unclear: whether static declaration avoids the dynamic-window bug
   - Recommendation: Test close interception as the first thing in Plan 01-01 before building other features on top of it; have the `ExitRequested` fallback ready

3. **Tray icon asset format for microphone silhouette**
   - What we know: Tauri uses `app.default_window_icon()` to reuse the app icon for tray; custom icons require a PNG or ICO in `src-tauri/icons/`
   - What's unclear: whether a 32x32 PNG is sufficient on Windows high-DPI displays, or if a multi-resolution ICO is needed
   - Recommendation: Use a 32x32 transparent PNG for Phase 1; defer multi-DPI tray icon polish

---

## Sources

### Primary (HIGH confidence)
- `/tauri-apps/tauri-docs` (Context7) — system tray, multiple windows, window close events, plugin permissions, single-instance
- `/tauri-apps/tauri-plugin-global-shortcut` (Context7) — register, unregister, Builder pattern, Cargo.toml setup
- `/tauri-apps/plugins-workspace` (Context7) — store plugin, autostart plugin API
- https://v2.tauri.app/plugin/global-shortcut/ — plugin setup steps, Rust version requirements
- https://v2.tauri.app/learn/system-tray/ — TrayIconBuilder API, icon loading

### Secondary (MEDIUM confidence)
- https://github.com/tauri-apps/tauri/discussions/11489 — ExitRequested + prevent_exit() pattern for hide-to-tray; confirmed working on the target platform
- https://github.com/tauri-apps/tauri/discussions/13472 — CSS class approach for theme toggle; confirmed as working workaround for broken prefers-color-scheme

### Tertiary (LOW confidence)
- https://github.com/tauri-apps/tauri/issues/12334 — prevent_close() bug; macOS-specific, closed not-planned; Windows behavior inferred as working from community usage

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all four plugins are official Tauri plugins with Context7-verified docs
- Architecture: HIGH — patterns sourced directly from Context7 official Tauri docs
- Pitfalls: MEDIUM-HIGH — capability pitfall and feature flag pitfall verified; close interception pitfall is MEDIUM (Windows-only verified, macOS known broken but irrelevant)
- Theme approach: MEDIUM — CSS class workaround verified via community discussion; official API broken per open GitHub issue

**Research date:** 2026-02-27
**Valid until:** 2026-03-27 (stable plugins; Tauri 2.x moves slowly on these APIs)
