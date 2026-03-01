---
phase: quick-8
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/src/pill.rs
  - src-tauri/src/lib.rs
  - src/Pill.tsx
autonomous: false
requirements: [PILL-POS-01, PILL-POS-02]

must_haves:
  truths:
    - "Pill appears horizontally centered on the monitor where the cursor currently is"
    - "Pill sits just above the taskbar (bottom of monitor work area)"
    - "Pill cannot be dragged or repositioned by the user"
    - "On a multi-monitor setup, pressing the hotkey on monitor 2 shows the pill on monitor 2"
  artifacts:
    - path: "src-tauri/src/pill.rs"
      provides: "show_pill() function that positions pill on cursor monitor and emits pill-show"
      contains: "pub fn show_pill"
    - path: "src-tauri/src/lib.rs"
      provides: "All pill-show calls replaced with pill::show_pill, saved position restore removed"
    - path: "src/Pill.tsx"
      provides: "Pill component with no drag logic, no saved position, no cursor-grab styling"
  key_links:
    - from: "src-tauri/src/lib.rs"
      to: "src-tauri/src/pill.rs"
      via: "pill::show_pill(app) called at all 4 recording-start sites"
      pattern: "pill::show_pill"
    - from: "src-tauri/src/pill.rs"
      to: "tauri monitor API"
      via: "cursor_position + available_monitors + work_area"
      pattern: "cursor_position.*available_monitors.*work_area"
---

<objective>
Fix pill position to bottom-center above the taskbar and add multi-monitor support.

Purpose: The pill currently uses a saved/draggable position which is fragile across monitor changes. It should always appear centered at the bottom of whichever monitor the cursor is on, with no user repositioning.

Output: Pill always appears bottom-center above taskbar on the active cursor monitor. All drag logic removed.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@src-tauri/src/pill.rs
@src-tauri/src/lib.rs
@src/Pill.tsx
@src-tauri/tauri.conf.json

<interfaces>
<!-- Tauri 2 APIs used in this plan -->

From tauri::WebviewWindow (via app.get_webview_window("pill")):
- fn cursor_position(&self) -> Result<PhysicalPosition<f64>>
- fn available_monitors(&self) -> Result<Vec<Monitor>>
- fn set_position(&self, position: Position) -> Result<()>

From tauri::Monitor:
- fn position(&self) -> &PhysicalPosition<i32>
- fn size(&self) -> &PhysicalSize<u32>
- fn work_area(&self) -> &PhysicalRect<i32, u32>

PhysicalRect has .position (PhysicalPosition<i32>) and .size (PhysicalSize<u32>).

Pill window config (tauri.conf.json):
- label: "pill", width: 178, height: 46
- transparent, decorations: false, alwaysOnTop, skipTaskbar

Current pill-show emission sites in lib.rs (lines 176, 201, 1068, 1093):
  app.emit_to("pill", "pill-show", ()).ok();
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add show_pill() with multi-monitor positioning, replace all call sites</name>
  <files>src-tauri/src/pill.rs, src-tauri/src/lib.rs</files>
  <action>
**In src-tauri/src/pill.rs**, add a new public function `show_pill`:

```rust
/// Position the pill at bottom-center of the monitor the cursor is on,
/// then emit pill-show to make it visible.
pub fn show_pill(app: &tauri::AppHandle) {
    if let Some(pill_window) = app.get_webview_window("pill") {
        // 1. Get cursor position
        let cursor = match pill_window.cursor_position() {
            Ok(pos) => pos,
            Err(e) => {
                log::warn!("Failed to get cursor position: {}, using primary monitor", e);
                // Fall back to primary monitor center
                tauri::PhysicalPosition { x: 0.0, y: 0.0 }
            }
        };

        // 2. Find which monitor the cursor is on
        let monitors = pill_window.available_monitors().unwrap_or_default();
        let target_monitor = monitors.iter().find(|m| {
            let pos = m.position();
            let size = m.size();
            let (mx, my) = (pos.x as f64, pos.y as f64);
            let (mw, mh) = (size.width as f64, size.height as f64);
            cursor.x >= mx && cursor.x < mx + mw && cursor.y >= my && cursor.y < my + mh
        });

        // 3. Get the work area (excludes taskbar) of that monitor
        // Fall back to primary monitor if cursor isn't found on any
        //
        // NOTE: work_area() may not exist on all Tauri 2 patch versions.
        // If it fails to compile, fall back to computing work area from
        // monitor.size() minus a fixed taskbar offset (e.g., 48px):
        //   let wa_pos = mon.position().clone();
        //   let wa_size_w = mon.size().width;
        //   let wa_size_h = mon.size().height - 48;
        let work_area = if let Some(mon) = target_monitor {
            mon.work_area().clone()
        } else if let Some(primary) = monitors.first() {
            primary.work_area().clone()
        } else {
            log::warn!("No monitors detected, emitting pill-show without positioning");
            app.emit_to("pill", "pill-show", ()).ok();
            return;
        };

        // 4. Calculate bottom-center position within work area
        // Pill dimensions: 178 x 46 (from tauri.conf.json)
        let pill_width = 178;
        let pill_height = 46;
        let margin_bottom = 14; // pixels above bottom of work area
        let wa_x = work_area.position.x;
        let wa_y = work_area.position.y;
        let wa_w = work_area.size.width as i32;
        let wa_h = work_area.size.height as i32;

        let x = wa_x + (wa_w - pill_width) / 2;
        let y = wa_y + wa_h - pill_height - margin_bottom;

        let _ = pill_window.set_position(tauri::PhysicalPosition::new(x, y));
        log::debug!("Pill positioned at ({}, {}) on monitor work area", x, y);

        // Emit pill-show AFTER positioning, inside the pill_window guard
        app.emit_to("pill", "pill-show", ()).ok();
    } else {
        log::warn!("Pill window not found, cannot position or show pill");
    }
}
```

Note: Use `app.get_webview_window("pill")` which requires `use tauri::Manager;` — this is already imported in lib.rs but pill.rs will need it too. Add `use tauri::Manager;` at the top of pill.rs. Also add `use tauri::Emitter;` for the emit_to call.

The `work_area()` method returns a reference to `PhysicalRect` which has `position: PhysicalPosition<i32>` and `size: PhysicalSize<u32>`. Clone it to own the data.

**IMPORTANT: `work_area()` availability.** If `work_area()` does not compile on your Tauri 2 version, replace the `work_area` variable computation with this fallback that uses `monitor.size()` minus a fixed 48px taskbar offset:
```rust
        // Fallback if work_area() is unavailable:
        let (wa_pos, wa_w, wa_h) = if let Some(mon) = target_monitor.or(monitors.first()) {
            let p = mon.position();
            let s = mon.size();
            (*p, s.width as i32, s.height as i32 - 48)
        } else {
            log::warn!("No monitors detected, emitting pill-show without positioning");
            app.emit_to("pill", "pill-show", ()).ok();
            return;
        };
        let wa_x = wa_pos.x;
        let wa_y = wa_pos.y;
```

**In src-tauri/src/lib.rs**, make these changes:

1. **Replace all 4 `pill-show` emission sites** (lines 176, 201, 1068, 1093):
   - Change `app.emit_to("pill", "pill-show", ()).ok();` to `pill::show_pill(&app);`
   - The app variable is `&tauri::AppHandle` at all 4 sites (already verified from context).

2. **Remove the saved-position restoration block in setup()** (lines 969-986):
   Delete the entire block that reads `pill-position` from `settings.json` and calls `set_position`. This is the section starting with the comment `// Restore saved pill position from settings.json` and ending just before `log::info!("Pill overlay window configured")`.

   Update the log message on line 988 to: `"Pill overlay window configured (focusable=false, no-shadow)"`

Do NOT remove the `set_focusable(false)` or `set_shadow(false)` calls in setup — those are still needed.
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && cargo build --manifest-path src-tauri/Cargo.toml --features whisper 2>&1 | tail -5</automated>
  </verify>
  <done>
    - pill.rs has show_pill() that detects cursor monitor, computes bottom-center of work area, positions pill, emits pill-show
    - All 4 pill-show emission sites in lib.rs call pill::show_pill(&app) instead of raw emit
    - Saved position restore block removed from setup()
    - Project compiles without errors
  </done>
</task>

<task type="auto">
  <name>Task 2: Remove drag logic and fixed-position init from Pill.tsx frontend</name>
  <files>src/Pill.tsx</files>
  <action>
Remove all drag/position code from Pill.tsx:

1. **Remove imports that are only used for drag/position:**
   - Delete: `import { PhysicalPosition } from "@tauri-apps/api/dpi";`
   - Delete: `import { load } from "@tauri-apps/plugin-store";`

2. **Delete the entire `initPosition` useEffect** (lines 35-52):
   Remove the useEffect that reads `pill-position` from the store and sets initial position. Positioning is now handled entirely by Rust before the pill-show event.

3. **Delete the `handleMouseDown` callback** (lines 116-120):
   This enables focus + starts dragging. Remove entirely.

4. **Delete the `handleMouseUp` callback** (lines 122-132):
   This saves position to store and restores non-focusable state. Remove entirely.

5. **Remove drag event handlers and cursor classes from the root div** (line 123-137):
   - Remove `onMouseDown={handleMouseDown}`
   - Remove `onMouseUp={handleMouseUp}`
   - Remove `cursor-grab active:cursor-grabbing` from the className
   - **Do NOT change any animation classes or conditionals** — preserve the existing className logic exactly as-is, minus only the drag-related attributes listed above.
   - **Do NOT re-add `pill-rainbow-border`** — it was removed in quick-7 and must stay removed.

   **IMPORTANT:** Before editing, read the current file state. The root div in the current codebase looks like this (the source of truth). Only remove the three items listed above. The result should be:
```tsx
<div
  className={`
    pill-glass
    w-[170px] h-[38px] rounded-full
    flex items-center justify-center
    select-none
    ${animState === "exiting" ? "pill-exiting" : ""}
    ${animState === "hidden" ? "opacity-0 pointer-events-none" : ""}
    ${displayState === "processing" ? "pill-processing" : ""}
    ${displayState === "recording" ? "pill-rainbow-border" : ""}
  `}
>
```

Wait — `pill-rainbow-border` is present in the current file but was supposed to be removed in quick-7. If it is still present, leave it as-is for now (that is a separate issue, not in scope for this task). Do NOT add it if it is missing. Do NOT remove it if it is present — that is out of scope. Only remove drag-related attributes.

After these removals, the only imports should be:
```tsx
import { useEffect, useState, useRef } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { FrequencyBars } from "./components/FrequencyBars";
import { ProcessingDots } from "./components/ProcessingDots";
```

Remove `useCallback` from the react import — it is no longer used after removing the drag handlers.
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && npx tsc --noEmit 2>&1 | tail -10</automated>
  </verify>
  <done>
    - No drag-related code remains in Pill.tsx (no startDragging, no position save/restore, no cursor-grab)
    - No unused imports (PhysicalPosition, plugin-store, useCallback all removed)
    - Pill div has no mouse event handlers, no grab cursor
    - TypeScript compiles without errors
  </done>
</task>

<task type="checkpoint:human-verify" gate="blocking">
  <name>Task 3: Verify pill positioning and multi-monitor behavior</name>
  <files>n/a</files>
  <action>
Pill now auto-positions to bottom-center above taskbar on the monitor where the cursor is. Dragging is disabled. Multi-monitor support added.

Verify the following:
1. Build and run: `cargo tauri dev --features whisper` (or however you normally run)
2. Press the hotkey to start recording — pill should appear centered at the bottom of your primary monitor, just above the taskbar
3. Try to drag the pill — it should NOT be draggable (no grab cursor, no movement)
4. If you have a second monitor: move your cursor to the second monitor, press the hotkey — pill should appear on that second monitor, bottom-center above its taskbar
5. Release/tap again to stop — pill should animate out normally
6. Re-trigger on original monitor — pill should appear back on original monitor
  </action>
  <verify>Human visual/functional verification</verify>
  <done>User confirms pill positions correctly on all monitors, no dragging possible</done>
</task>

</tasks>

<verification>
- `cargo build --manifest-path src-tauri/Cargo.toml --features whisper` succeeds
- `npx tsc --noEmit` succeeds
- No references to `pill-position` remain in codebase (grep for `pill-position` should return 0 hits)
- No references to `startDragging` remain in Pill.tsx
- `pill::show_pill` is called at all 4 hotkey-triggered recording start sites
</verification>

<success_criteria>
- Pill appears bottom-center above taskbar on cursor's monitor every time
- Pill is not draggable
- Multi-monitor: pill follows cursor to whichever monitor is active
- No regressions in pill show/hide animations or state transitions
</success_criteria>

<output>
After completion, create `.planning/quick/8-fix-pill-position-to-bottom-center-above/8-SUMMARY.md`
</output>
