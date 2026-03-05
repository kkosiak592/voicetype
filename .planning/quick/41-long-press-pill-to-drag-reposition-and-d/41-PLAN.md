---
phase: quick-41
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/Pill.tsx
  - src/pill.css
  - src-tauri/src/pill.rs
  - src-tauri/src/lib.rs
autonomous: true
requirements: [QUICK-41]

must_haves:
  truths:
    - "Long-pressing the pill for ~600ms enters drag mode (visual glow + slight scale)"
    - "While in drag mode, moving the mouse repositions the pill window in real time"
    - "Releasing the mouse exits drag mode and persists the new position to settings.json"
    - "Double-clicking the pill resets it to the default bottom-center home position"
    - "When show_pill is called, if a saved position exists it is used instead of recomputing bottom-center"
    - "Custom position survives app restart"
  artifacts:
    - path: "src/Pill.tsx"
      provides: "Long-press detection, drag mode state, double-click reset, IPC calls"
    - path: "src/pill.css"
      provides: "drag-mode glow animation keyframe"
    - path: "src-tauri/src/pill.rs"
      provides: "show_pill reads saved pill_position; set_pill_position command moves window + persists; reset_pill_position clears saved + recenters"
    - path: "src-tauri/src/lib.rs"
      provides: "Registers set_pill_position and reset_pill_position as Tauri invoke handlers"
  key_links:
    - from: "src/Pill.tsx"
      to: "src-tauri/src/lib.rs"
      via: "invoke('set_pill_position', {x, y}) and invoke('reset_pill_position')"
      pattern: "invoke.*pill_position"
    - from: "src-tauri/src/pill.rs show_pill()"
      to: "settings.json pill_position key"
      via: "read_settings -> json[\"pill_position\"]"
      pattern: "pill_position"
---

<objective>
Add iPhone-style long-press-to-drag repositioning to the pill overlay, with double-click to reset home position and persistence across restarts.

Purpose: The pill is fixed at bottom-center and cannot be moved out of the way. Users need to reposition it depending on what they're working on.
Output: Draggable pill window with persisted position and double-click home reset.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md

Relevant existing code:

From src/Pill.tsx:
- Uses `getCurrentWebviewWindow()` as `appWindow`
- Listens to `pill-show`, `pill-hide`, `pill-state`, `pill-level`, `pill-result` events
- `displayState`: "hidden" | "recording" | "processing" | "error"
- `animState`: "hidden" | "visible" | "exiting"
- Root div has `select-none` and `pointer-events-none` when hidden

From src-tauri/src/pill.rs:
- `show_pill(app)` — positions pill at bottom-center of cursor's monitor, then emits `pill-show`
- Position math uses `work_area`, pill dims 178x46, margin_bottom 14px

From src-tauri/src/lib.rs:
- Settings persistence via `read_settings(app)` / `write_settings(app, &json)` — uses `settings.json` key-value store
- Pattern for IPC commands: `#[tauri::command] async fn set_model(...) -> Result<(), String>`
- Pill window setup at startup at line ~1744: `set_focusable(false)`, `set_shadow(false)`

Pill window config (tauri.conf.json):
- 178x46 transparent, no decorations, alwaysOnTop, skipTaskbar
- `resizable: false` (does NOT prevent programmatic `set_position`)

Settings.json pattern (from lib.rs):
- `read_settings(&app)` returns `serde_json::Value`
- `json["pill_position"] = serde_json::json!({"x": x, "y": y})`
- `write_settings(&app, &json)` flushes to disk
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add set_pill_position and reset_pill_position IPC commands; update show_pill to respect saved position</name>
  <files>src-tauri/src/pill.rs, src-tauri/src/lib.rs</files>
  <action>
In `src-tauri/src/pill.rs`:

1. Modify `show_pill(app)` to check for a saved position before computing bottom-center:
   - Call `read_settings` (import from parent via `crate::read_settings` — it is `pub(crate)`)
   - If `json["pill_position"]["x"]` and `["y"]` exist as i64 values, use those instead of computing bottom-center
   - Log: `"Pill restored to saved position ({}, {})"` vs `"Pill positioned at bottom-center ({}, {})"`

2. Add `pub async fn set_pill_position(app: tauri::AppHandle, x: i32, y: i32) -> Result<(), String>`:
   - Get pill window: `app.get_webview_window("pill").ok_or("pill window not found")?`
   - Call `pill_window.set_position(tauri::PhysicalPosition::new(x, y)).map_err(|e| e.to_string())?`
   - Persist: `let mut json = crate::read_settings(&app)?; json["pill_position"] = serde_json::json!({"x": x, "y": y}); crate::write_settings(&app, &json)?`
   - Return Ok(())

3. Add `pub async fn reset_pill_position(app: tauri::AppHandle) -> Result<(), String>`:
   - Remove saved position: `let mut json = crate::read_settings(&app)?; if let Some(obj) = json.as_object_mut() { obj.remove("pill_position"); } crate::write_settings(&app, &json)?`
   - Recompute and apply bottom-center position (extract the positioning logic from `show_pill` into a helper or call `show_pill` without emitting `pill-show` — preferred: extract a `position_pill_at_home(app)` helper that just sets the position without emitting, call it from both `show_pill` default path and `reset_pill_position`)
   - Return Ok(())

In `src-tauri/src/lib.rs`:

4. Register both commands in the `.invoke_handler(tauri::generate_handler![...])` call — add `pill::set_pill_position, pill::reset_pill_position` to the existing handler list.

Note: `read_settings` and `write_settings` are currently `pub(crate) fn` — verify they are accessible from pill.rs (same crate, so yes). If they are `fn` (private), change to `pub(crate) fn` for both.
  </action>
  <verify>
    Build check: `cd /c/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text/src-tauri && cargo check 2>&1 | tail -20`
  </verify>
  <done>
    `cargo check` passes with no errors. `set_pill_position` and `reset_pill_position` are registered as Tauri commands. `show_pill` uses saved position when available.
  </done>
</task>

<task type="auto">
  <name>Task 2: Add long-press drag interaction and double-click reset to Pill.tsx; add drag-mode CSS</name>
  <files>src/Pill.tsx, src/pill.css</files>
  <action>
In `src/pill.css`, add:

```css
/* Drag mode: glow + scale cue */
@keyframes pill-drag-ready {
  0%, 100% { transform: scale(1.0); }
  50% { transform: scale(1.04); }
}

.pill-drag-ready {
  animation: pill-drag-ready 0.5s ease-in-out 1;
  border: 1px solid rgba(139, 92, 246, 0.7);
  box-shadow: 0 0 20px rgba(139, 92, 246, 0.35), 0 4px 16px rgba(0, 0, 0, 0.4);
  cursor: grab;
}

.pill-dragging {
  border: 1px solid rgba(139, 92, 246, 0.9);
  box-shadow: 0 0 28px rgba(139, 92, 246, 0.5), 0 8px 24px rgba(0, 0, 0, 0.5);
  cursor: grabbing;
}
```

In `src/Pill.tsx`, add drag logic:

1. New imports: `invoke` from `@tauri-apps/api/core`

2. New state/refs:
   ```typescript
   type DragState = "idle" | "ready" | "dragging";
   const [dragState, setDragState] = useState<DragState>("idle");
   const longPressTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
   const dragOriginRef = useRef<{ mouseX: number; mouseY: number; winX: number; winY: number } | null>(null);
   ```

3. Long-press: on `pointerdown`, start a 600ms timer. When it fires, set `dragState = "ready"` and capture pointer (`e.currentTarget.setPointerCapture(e.pointerId)`). On `pointerup` or `pointercancel` before timer fires, clear the timer (no drag).

4. Drag move: on `pointermove`, if `dragState === "ready"` and this is the first move after ready, transition to `"dragging"` and record `dragOriginRef = { mouseX: e.screenX, mouseY: e.screenY, winX: currentWinX, winY: currentWinY }`. But we don't have current window position from frontend — simpler approach: use `e.screenX` and `e.screenY` directly as the target window position minus half pill width/height to center under cursor:
   - `const x = Math.round(e.screenX) - 89` (89 = 178/2)
   - `const y = Math.round(e.screenY) - 23` (23 = 46/2)
   - Call `invoke('set_pill_position', { x, y })` — throttle to once per ~16ms using a `lastInvokeRef`

5. On `pointerup` when `dragState === "dragging"`: set `dragState = "idle"`, release pointer capture. The last `set_pill_position` already persisted.

6. On `pointerup` when `dragState === "ready"` (long-press held but no move): set `dragState = "idle"` — treat as cancelled.

7. Double-click: add `onDoubleClick` handler. Call `invoke('reset_pill_position')`. No visual feedback needed beyond the pill snapping back (which `show_pill` handles next time it shows — but for immediate snap during recording, also emit a JS event or just call `reset_pill_position` which repositions the window directly).

8. Apply CSS classes to root div:
   ```tsx
   ${dragState === "ready" ? "pill-drag-ready" : ""}
   ${dragState === "dragging" ? "pill-dragging" : ""}
   ```

9. Remove `select-none` when in drag mode is not needed — `select-none` is fine for drag UX.

10. In `clearAllTimers()`, also clear `longPressTimerRef`.

Important: The `pointer-events-none` class is only applied when `animState === "hidden"`. When pill is visible and recording/processing, pointer events are active — long-press will work correctly.

The `set_focusable(false)` on the pill window means clicks do NOT steal focus from the user's active app — drag events still fire because they don't require focus.
  </action>
  <verify>
    TypeScript check: `cd /c/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && npx tsc --noEmit 2>&1 | tail -20`
  </verify>
  <done>
    `tsc --noEmit` passes. Pill.tsx compiles with no type errors. Drag state machine, long-press timer, pointer event handlers, and double-click reset are all wired.
  </done>
</task>

<task type="checkpoint:human-verify" gate="blocking">
  <what-built>
    Long-press drag repositioning and double-click reset for the pill overlay.
    - Long-press ~600ms while pill is visible enters drag mode (purple glow)
    - Dragging moves the pill window in real time
    - Release persists position to settings.json
    - Double-click resets to bottom-center home position
    - Position survives app restart (show_pill uses saved position)
  </what-built>
  <how-to-verify>
    1. Run `npm run tauri dev` to start the app
    2. Trigger recording (hotkey) so the pill appears
    3. Long-press the pill for ~600ms — verify it glows purple
    4. Drag it to a different screen position — verify it moves smoothly
    5. Release — verify it stays at the new position
    6. Trigger recording again — verify pill appears at the saved position (not bottom-center)
    7. Restart the app, trigger recording — verify pill still appears at the saved position
    8. Double-click the pill — verify it snaps back to bottom-center
    9. Trigger recording again — verify pill now shows at bottom-center
  </how-to-verify>
  <resume-signal>Type "approved" or describe any issues</resume-signal>
</task>

</tasks>

<verification>
- `cargo check` passes (Task 1 verify)
- `tsc --noEmit` passes (Task 2 verify)
- Manual verification confirms drag, persist, and reset behavior (Task 3)
</verification>

<success_criteria>
- Long-press 600ms enters drag mode with visible purple glow
- Mouse drag repositions pill window in real time via IPC
- Position persists to settings.json on release
- App restart shows pill at saved position
- Double-click resets to bottom-center and clears saved position
</success_criteria>

<output>
After completion, create `.planning/quick/41-long-press-pill-to-drag-reposition-and-d/41-SUMMARY.md`
</output>
