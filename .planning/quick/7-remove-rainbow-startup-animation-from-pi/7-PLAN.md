---
phase: quick
plan: 7
type: execute
wave: 1
depends_on: []
files_modified:
  - src/Pill.tsx
  - src/pill.css
autonomous: true
requirements: []
must_haves:
  truths:
    - "Pill appears during recording with no rainbow rotating border"
    - "Frequency bars (waveform) render immediately inside the pill on entrance"
    - "Processing state border animation still works unchanged"
  artifacts:
    - path: "src/Pill.tsx"
      provides: "Pill component without rainbow border class"
    - path: "src/pill.css"
      provides: "CSS without rainbow border rules"
  key_links: []
---

<objective>
Remove the rainbow rotating conic-gradient border from the pill during recording state. The pill should appear with just the dark glass background and frequency bars inside — no colorful spinning ring around it.

Purpose: The rainbow border is visual noise; the waveform bars are sufficient feedback during recording.
Output: Clean pill appearance during recording — dark glass + waveform bars only.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src/Pill.tsx
@src/pill.css
</context>

<tasks>

<task type="auto">
  <name>Task 1: Remove rainbow border from pill recording state</name>
  <files>src/Pill.tsx, src/pill.css</files>
  <action>
In src/Pill.tsx:
- Remove the conditional class on line 148: `${displayState === "recording" ? "pill-rainbow-border" : ""}` — delete the entire line.

In src/pill.css:
- Remove the entire "Rainbow border" section (lines 245-277):
  - The `@property --border-angle` declaration
  - The `@keyframes rainbow-rotate` keyframe
  - The `.pill-rainbow-border` rule
  - The `.pill-rainbow-border::after` rule
- Also remove the comment on line 13 referencing rainbow border: update "leaves room for rainbow border" to just "centers pill in window" or similar.

Do NOT touch:
- The processing state border (`.pill-processing::after` uses a different gradient: cyan/violet/pink, not rainbow — keep it)
- The FrequencyBars component bar colors (those are per-bar hues, not a border animation)
- The entrance/exit animations (`pill-entering`, `pill-exiting`) — those stay
- The `pill-content-fade-in` class — that stays
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && npx tsc --noEmit 2>&1 | head -20</automated>
  </verify>
  <done>
- pill-rainbow-border class no longer referenced in Pill.tsx
- All rainbow border CSS rules removed from pill.css
- Processing border animation untouched
- TypeScript compiles without errors
  </done>
</task>

</tasks>

<verification>
- `grep -r "rainbow" src/` returns only the FrequencyBars.tsx comment about rainbow hue (bar colors), nothing in Pill.tsx or pill.css
- `grep "pill-rainbow" src/` returns no results
- TypeScript compiles cleanly
</verification>

<success_criteria>
Pill appears during recording with dark glass background + frequency bars only, no rotating rainbow ring. Processing state gradient border still works.
</success_criteria>

<output>
After completion, create `.planning/quick/7-remove-rainbow-startup-animation-from-pi/7-SUMMARY.md`
</output>
