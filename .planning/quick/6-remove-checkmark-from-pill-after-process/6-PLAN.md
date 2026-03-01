---
phase: quick
plan: 6
type: execute
wave: 1
depends_on: []
files_modified:
  - src/Pill.tsx
  - src/pill.css
  - src/components/CheckmarkIcon.tsx
autonomous: true
requirements: [QUICK-6]

must_haves:
  truths:
    - "After processing completes successfully, the pill exits immediately without showing a checkmark"
    - "The success path behaves identically to the error path — immediate exit animation"
    - "No dead code remains for the removed checkmark feature"
  artifacts:
    - path: "src/Pill.tsx"
      provides: "Pill component without checkmark state"
    - path: "src/pill.css"
      provides: "Styles without checkmark animation"
  key_links:
    - from: "pill-result success handler"
      to: "exit animation"
      via: "direct setAnimState('exiting') call, no intermediate success display state"
---

<objective>
Remove the checkmark icon and animation that appears on the pill after successful processing. The pill should transition directly to exit animation on success, identical to the error path.

Purpose: User does not want the checkmark feedback — pill should just dismiss.
Output: Pill component without checkmark state, CSS without checkmark animation, CheckmarkIcon component deleted.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src/Pill.tsx
@src/pill.css
@src/components/CheckmarkIcon.tsx
</context>

<tasks>

<task type="auto">
  <name>Task 1: Remove checkmark from pill success path and clean up</name>
  <files>src/Pill.tsx, src/pill.css, src/components/CheckmarkIcon.tsx</files>
  <action>
In src/Pill.tsx:
1. Remove the `import { CheckmarkIcon } from "./components/CheckmarkIcon"` line (line 7).
2. Remove `"success"` from the `PillDisplayState` type union — it becomes `"hidden" | "recording" | "processing" | "error"`.
3. Remove the `successTimerRef` ref declaration (line 22) and its cleanup in `clearAllTimers()` (lines 34-37).
4. In the `pill-result` listener, change the success branch (lines 108-120) to match the error branch — immediately trigger exit animation without setting displayState to "success":
   ```typescript
   if (result === "success") {
     setAnimState("exiting");
     exitTimerRef.current = setTimeout(() => {
       appWindow.hide();
       setAnimState("hidden");
       setDisplayState("hidden");
       exitTimerRef.current = null;
     }, 200);
   }
   ```
   Since both branches are now identical, simplify the entire `pill-result` handler to remove the if/else — just always do the exit animation regardless of result payload.
5. Remove the JSX block for success state (lines 189-194): the `{displayState === "success" && ...}` conditional rendering block with `<CheckmarkIcon />`.
6. Update the comment on line 104 from "success shows checkmark then exits; error silently exits" to "trigger exit animation on result".

In src/pill.css:
7. Remove the entire checkmark section (lines 281-295): the comment header, `.pill-checkmark-draw` rule, and `@keyframes draw-check`.

Delete src/components/CheckmarkIcon.tsx entirely.
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && npx tsc --noEmit 2>&1 | head -20</automated>
  </verify>
  <done>Pill exits immediately on success without showing checkmark. No "success" display state, no CheckmarkIcon component, no checkmark CSS animation. TypeScript compiles cleanly.</done>
</task>

</tasks>

<verification>
- `npx tsc --noEmit` passes with no errors
- No references to CheckmarkIcon or pill-checkmark-draw remain in the codebase
- The pill-result handler triggers immediate exit for both success and error
</verification>

<success_criteria>
- Pill dismisses immediately after successful processing (no checkmark flash)
- CheckmarkIcon.tsx deleted
- No dead code remains (no "success" display state, no successTimerRef, no checkmark CSS)
- App compiles without errors
</success_criteria>

<output>
After completion, create `.planning/quick/6-remove-checkmark-from-pill-after-process/6-SUMMARY.md`
</output>
