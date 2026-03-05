---
phase: quick-46
plan: 01
type: execute
wave: 1
depends_on: []
files_modified: [src-tauri/src/keyboard_hook.rs]
autonomous: true
requirements: [QUICK-46]

must_haves:
  truths:
    - "Generic VK_CONTROL keydown sets STATE.ctrl_held to true"
    - "Generic VK_CONTROL keyup sets STATE.ctrl_held to false and fires Released if combo was active"
    - "Ctrl+Win hotkey fires in Outlook and Office apps that send generic VK_CONTROL"
  artifacts:
    - path: "src-tauri/src/keyboard_hook.rs"
      provides: "VK_CONTROL handling in keyboard hook"
      contains: "VK_CONTROL"
  key_links:
    - from: "keyboard_hook.rs ctrl keydown block"
      to: "STATE.ctrl_held"
      via: "VK_CONTROL added to condition"
      pattern: "vk == VK_CONTROL"
---

<objective>
Add VK_CONTROL (generic) tracking to the keyboard hook's Ctrl keydown and keyup conditions, matching the pattern already used for Shift (VK_SHIFT) and Alt (VK_MENU).

Purpose: Outlook and Office apps send the generic VK_CONTROL virtual key code instead of the left/right variants (VK_LCONTROL/VK_RCONTROL). Without tracking VK_CONTROL, STATE.ctrl_held is never set when these apps are focused, breaking Ctrl+Win combo detection.

Output: Updated keyboard_hook.rs with VK_CONTROL in both Ctrl keydown and Ctrl keyup conditions.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src-tauri/src/keyboard_hook.rs
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add VK_CONTROL to Ctrl keydown and keyup conditions</name>
  <files>src-tauri/src/keyboard_hook.rs</files>
  <action>
In keyboard_hook.rs, make two edits:

1. Line ~272 — Ctrl keydown condition: Change
   `if is_down && (vk == VK_LCONTROL || vk == VK_RCONTROL)`
   to
   `if is_down && (vk == VK_LCONTROL || vk == VK_RCONTROL || vk == VK_CONTROL)`

2. Line ~299 — Ctrl keyup condition: Change
   `if is_up && (vk == VK_LCONTROL || vk == VK_RCONTROL)`
   to
   `if is_up && (vk == VK_LCONTROL || vk == VK_RCONTROL || vk == VK_CONTROL)`

This matches the existing pattern used for Shift (line ~260: `VK_LSHIFT || VK_RSHIFT || VK_SHIFT`) and Alt (line ~266: `VK_LMENU || VK_RMENU || VK_MENU`).

Do NOT change any other logic. The two conditions are the only edits.
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && cargo check 2>&1 | tail -5</automated>
  </verify>
  <done>VK_CONTROL appears in both Ctrl keydown and Ctrl keyup conditions. cargo check passes. The three modifier families (Shift, Alt, Ctrl) now all track their generic variant consistently.</done>
</task>

</tasks>

<verification>
- `cargo check` compiles without errors
- grep confirms VK_CONTROL appears in keyboard_hook.rs Ctrl tracking blocks
- Pattern consistency: all three modifier families (Shift/Alt/Ctrl) handle generic + left + right variants
</verification>

<success_criteria>
- keyboard_hook.rs Ctrl keydown condition includes VK_CONTROL
- keyboard_hook.rs Ctrl keyup condition includes VK_CONTROL
- No other changes to file
- Builds successfully
</success_criteria>

<output>
After completion, create `.planning/quick/46-add-vk-control-tracking-to-keyboard-hook/46-SUMMARY.md`
</output>
