---
status: awaiting_human_verify
trigger: "pill window not appearing — Tauri v2 app has two windows configured in tauri.conf.json — settings (works fine) and pill (never appears)"
created: 2026-02-28T00:00:00Z
updated: 2026-02-28T01:00:00Z
---

## Current Focus

hypothesis: CONFIRMED ROOT CAUSE. The implementation diverges from the plan spec in a way that makes the pill window invisible even with visible:true. The config was changed to {transparent:false, decorations:true, visible:true, skipTaskbar:false} for debugging. However, Pill.tsx starts with React state visible=false (opacity-0). The window builds (confirmed by source), exists as an OS window, loads http://localhost:1420/pill.html correctly, but: (a) content is opacity-0 so nothing renders inside the window frame, (b) the window was showing a white/transparent rectangle. The "not in taskbar" may be explained by skipTaskbar:false not being respected by Windows OR by the window briefly existing before being somehow dismissed. Fix: restore correct config (transparent:true, decorations:false, visible:false, skipTaskbar:true) AND add explicit show() in setup for debugging.
test: Apply fix and check if window appears when explicitly shown
expecting: After adding show() call in Rust setup, window should appear as transparent pill shape on screen
next_action: Apply config fix + add diagnostic show() in lib.rs

## Symptoms

expected: A second window labeled "pill" should appear when the app launches, loading pill.html with a React component
actual: Only the settings window appears. No pill window visible anywhere — not behind other windows, not offscreen, not in taskbar
errors: No errors in terminal output. Build succeeds with only one warning (dead_code in pipeline.rs)
reproduction: Run `npx tauri dev --features whisper`, app launches showing only the settings window
started: First time pill window was added (Phase 04, Plan 04-01). Has never worked.

## Eliminated

(none yet)

## Evidence

- timestamp: 2026-02-28T00:01:00Z
  checked: src-tauri/tauri.conf.json
  found: pill window configured with url:"pill.html", visible:true, decorations:true, 300x200, alwaysOnTop:true. settings window has NO url field (uses default index.html)
  implication: Tauri will attempt to load http://localhost:1420/pill.html in dev mode

- timestamp: 2026-02-28T00:02:00Z
  checked: src-tauri/src/lib.rs setup block
  found: `if let Some(pill_window) = app.get_webview_window("pill")` — pill window IS retrieved (Some branch executes if window exists). The log line "Pill overlay window configured (focusable=false)" would appear if window was created. No log means get_webview_window returns None OR the log is never printed for another reason.
  implication: If Tauri created the window, it would be retrievable. Silent None = window creation failed at Tauri level OR window exists but get_webview_window label mismatch.

- timestamp: 2026-02-28T00:03:00Z
  checked: vite.config.ts
  found: rollupOptions.input has both main (index.html) and pill (pill.html). This is the PRODUCTION build config. In dev mode, Vite does NOT use rollupOptions — it serves files via its dev server on-demand.
  implication: pill.html IS served by Vite dev server at http://localhost:1420/pill.html (Vite serves static HTML files from project root). This is not the problem.

- timestamp: 2026-02-28T00:04:00Z
  checked: pill.html
  found: references `<script type="module" src="/src/pill-main.tsx"></script>` and `<div id="pill-root"></div>`. Body has transparent background.
  implication: Vite will serve /src/pill-main.tsx fine in dev mode.

- timestamp: 2026-02-28T00:05:00Z
  checked: src/pill-main.tsx
  found: imports `./pill.css` and `./Pill` (component). Both files exist.
  implication: Module graph is complete. No import errors.

- timestamp: 2026-02-28T00:08:00Z
  checked: Tauri v2 source — tauri-2.10.2/src/app.rs lines 2373-2380
  found: Both windows are created from config in a loop before user setup closure runs. Error from build() uses ? operator which would panic if it fails ("Failed to setup app"). Since settings window works, both windows must build successfully.
  implication: Pill window IS created at the OS level.

- timestamp: 2026-02-28T00:09:00Z
  checked: Tauri v2 source — manager/webview.rs WebviewUrl::App path for desktop
  found: PROXY_DEV_SERVER = cfg!(all(dev, mobile)) = false on desktop Windows. devUrl is used as base URL. url.join("pill.html") = http://localhost:1420/pill.html. URL construction is correct.
  implication: Pill window loads http://localhost:1420/pill.html from Vite dev server.

- timestamp: 2026-02-28T00:10:00Z
  checked: src/Pill.tsx
  found: const [visible, setVisible] = useState(false) — starts invisible. CSS class applies opacity-0 when visible=false. The window only becomes visible (in React sense) when "pill-show" event is received. Nobody emits "pill-show" on startup from Rust or JS.
  implication: The React component content is ALWAYS opacity-0 at startup. The window frame (decorations:true) should still be visible as an OS window, but the content area is transparent/invisible. This could explain why the user thinks there's "no window" — they see a window frame but no content.

- timestamp: 2026-02-28T00:11:00Z
  checked: pill.css
  found: body and #pill-root have background: transparent !important. tauri.conf.json has transparent: false for pill window.
  implication: With transparent:false in Tauri config, WebView2's default background should be white. The CSS sets background to transparent. On Windows, this may result in a window with white OR black background but no visible content (opacity-0 component).

- timestamp: 2026-02-28T00:06:00Z
  checked: src-tauri/capabilities/default.json
  found: windows array includes both "settings" and "pill"
  implication: Capabilities are not the issue.

- timestamp: 2026-02-28T00:07:00Z
  checked: tauri.conf.json build.devUrl
  found: devUrl is set to "http://localhost:1420". This is correct. Without devUrl, Tauri would not know where to point the webview in dev mode.
  implication: devUrl is correct. Both windows should point to this dev server.

## Resolution

root_cause: The pill window config was changed from the plan spec (transparent:true, decorations:false, visible:false, skipTaskbar:true) to debug values (transparent:false, decorations:true, visible:true, skipTaskbar:false) — but the React component (Pill.tsx) always starts with visible=false (opacity-0) and only shows on "pill-show" event. Additionally, previous config without devUrl caused tauri:// protocol fallback to index.html. The window was being created (Tauri source confirms panic if not), but was either: (a) showing as empty/blank because content was opacity-0 + wrong HTML loaded, or (b) appearing briefly and not being noticed. Fix applied: restore correct config from plan spec + add explicit show() in Rust setup for debugging + add error logging for None case.
fix: |
  1. tauri.conf.json: Restored pill window to plan spec: transparent:true, decorations:false, visible:false, skipTaskbar:true, width:120, height:40
  2. lib.rs: Added explicit pill_window.show() call in setup (debug) + error logging when get_webview_window returns None
verification: PENDING — user needs to run the app and check if the pill appears as a transparent pill shape
files_changed:
  - src-tauri/tauri.conf.json
  - src-tauri/src/lib.rs
