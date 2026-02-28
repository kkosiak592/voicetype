---
phase: quick-2
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/capabilities/default.json
  - src-tauri/src/lib.rs
autonomous: false
requirements: [QUICK-2]
must_haves:
  truths:
    - "Pill overlay has no DWM shadow haze around rounded corners on Windows 10"
    - "Pill window still renders with transparent background and border-radius intact"
  artifacts:
    - path: "src-tauri/capabilities/default.json"
      provides: "set-shadow permission grant"
      contains: "core:window:allow-set-shadow"
    - path: "src-tauri/src/lib.rs"
      provides: "set_shadow(false) call on pill window"
      contains: "set_shadow(false)"
  key_links:
    - from: "src-tauri/capabilities/default.json"
      to: "src-tauri/src/lib.rs"
      via: "permission enables runtime API call"
      pattern: "allow-set-shadow"
---

<objective>
Fix the visible haziness around the pill overlay's rounded corners on Windows 10.

Purpose: DWM applies a rectangular window shadow to transparent undecorated windows that doesn't respect CSS border-radius, causing a ghostly rectangular haze around the pill shape. Disabling the shadow via Tauri's set_shadow(false) API eliminates this.

Output: Two small edits — one permission grant, one API call — that remove the DWM shadow artifact.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@src-tauri/capabilities/default.json
@src-tauri/src/lib.rs (lines 336-341 — pill window setup block)

<interfaces>
<!-- Existing pill window setup pattern in lib.rs (around line 336-341): -->

```rust
// Configure pill overlay: no focus steal + restore saved position
if let Some(pill_window) = app.get_webview_window("pill") {
    log::info!("Pill window found — applying configuration");

    // focusable(false) sets WS_EX_NOACTIVATE — pill never steals focus
    let _ = pill_window.set_focusable(false);

    // ... position restore logic follows ...
}
```

<!-- Existing permissions array in default.json: -->
```json
"permissions": [
    "core:default",
    "core:window:allow-show",
    "core:window:allow-hide",
    "core:window:allow-set-focusable",
    "core:window:allow-set-position",
    "core:window:allow-start-dragging",
    "store:default",
    "autostart:allow-enable",
    "autostart:allow-disable",
    "autostart:allow-is-enabled"
]
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add set-shadow permission and disable DWM shadow on pill window</name>
  <files>src-tauri/capabilities/default.json, src-tauri/src/lib.rs</files>
  <action>
Two edits:

1. **src-tauri/capabilities/default.json** — Add `"core:window:allow-set-shadow"` to the permissions array, after the existing `"core:window:allow-set-focusable"` entry (line 10). This follows the same pattern used for allow-set-focusable, allow-show, allow-hide, etc.

2. **src-tauri/src/lib.rs** — Add `let _ = pill_window.set_shadow(false);` immediately after the existing `let _ = pill_window.set_focusable(false);` call (line 340). Add a comment: `// Disable DWM shadow — rectangular shadow doesn't respect CSS border-radius (tauri#11321)`. Same `let _ =` pattern as the focusable call (ignore Result since shadow removal is non-critical).

Do NOT change anything else in either file.
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && grep -q "allow-set-shadow" src-tauri/capabilities/default.json && grep -q "set_shadow(false)" src-tauri/src/lib.rs && echo "PASS" || echo "FAIL"</automated>
  </verify>
  <done>default.json contains "core:window:allow-set-shadow" permission. lib.rs calls pill_window.set_shadow(false) after set_focusable(false). No other changes.</done>
</task>

<task type="checkpoint:human-verify" gate="blocking">
  <name>Task 2: Verify pill corners are clean</name>
  <what-built>Disabled DWM window shadow on the pill overlay to eliminate rounded corner haziness.</what-built>
  <how-to-verify>
    1. Build and run the app: `npx tauri dev` (or however you normally launch)
    2. Trigger the pill overlay (use the hotkey to start recording)
    3. Look at the pill's rounded corners — the rectangular shadow haze should be gone
    4. Confirm the pill still has its transparent background and rounded shape
    5. Confirm drag still works (click and drag the pill)
  </how-to-verify>
  <resume-signal>Type "approved" or describe any remaining visual issues</resume-signal>
</task>

</tasks>

<verification>
- `grep "allow-set-shadow" src-tauri/capabilities/default.json` returns a match
- `grep "set_shadow(false)" src-tauri/src/lib.rs` returns a match
- App builds without errors
- Pill overlay renders without rectangular shadow haze at corners
</verification>

<success_criteria>
Pill overlay on Windows 10 shows clean rounded corners with no DWM shadow artifact. The fix is two lines total across two files.
</success_criteria>

<output>
After completion, create `.planning/quick/2-fix-pill-rounded-corner-haziness-via-set/2-SUMMARY.md`
</output>
