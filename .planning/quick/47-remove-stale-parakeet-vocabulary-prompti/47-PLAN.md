---
phase: "47"
plan: 1
type: execute
wave: 1
depends_on: []
files_modified:
  - src/components/sections/ModelSection.tsx
autonomous: true
requirements:
  - QT-47

must_haves:
  truths:
    - "No vocabulary prompting warning appears on model page for any engine"
    - "No stale vocabulary/initial_prompt references remain anywhere in codebase"
    - "ModelSection still renders and functions correctly after removal"
  artifacts:
    - path: "src/components/sections/ModelSection.tsx"
      provides: "Model settings UI without stale warning"
  key_links: []
---

<objective>
Remove the stale vocabulary prompting warning from ModelSection.tsx. Vocabulary prompting was fully removed in quick task #38 (commit 6c3616b), but a conditional warning block remains on lines 198-203 telling users that Parakeet/Moonshine "doesn't support vocabulary prompting." This is now misleading since vocabulary prompting no longer exists for any engine.

Purpose: Eliminate confusing stale UI text that references a removed feature.
Output: Clean ModelSection.tsx with no vocabulary prompting references.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src/components/sections/ModelSection.tsx

Pre-analysis findings:
- The ONLY remaining vocabulary prompting reference is in ModelSection.tsx lines 198-203
- A codebase-wide grep for "vocabulary", "initial_prompt" across both src/ and src-tauri/ returned zero other matches
- The stale block is a conditional rendering: `{(currentEngine === 'parakeet' || currentEngine === 'moonshine') && (<p>...doesn't support vocabulary prompting...</p>)}`
- No state, props, or imports exist solely to serve this warning -- `currentEngine` state is used elsewhere in the component for engine-switching logic
</context>

<tasks>

<task type="auto">
  <name>Task 1: Remove stale vocabulary prompting warning and clean up</name>
  <files>src/components/sections/ModelSection.tsx</files>
  <action>
Remove lines 198-203 from ModelSection.tsx -- the entire conditional block:
```
{(currentEngine === 'parakeet' || currentEngine === 'moonshine') && (
  <p className="text-xs text-amber-600 ...">
    <span className="size-1.5 rounded-full bg-amber-500"></span>
    {currentEngine.charAt(0).toUpperCase() + currentEngine.slice(1)} doesn't support vocabulary prompting. Your corrections dictionary still applies.
  </p>
)}
```

Also remove the empty lines (196-197) between the ModelSelector card closing div and this block to keep clean formatting.

Do NOT remove `currentEngine` state or `loadEngine()` -- these are used by the engine-switching logic in `handleModelSelect`.

After removal, verify no other vocabulary/initial_prompt references exist with a codebase-wide search.
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && npx tsc --noEmit 2>&1 | head -20</automated>
  </verify>
  <done>ModelSection.tsx compiles without errors. No vocabulary prompting warning text exists in the file. Grep for "vocabulary" across src/ and src-tauri/ returns zero matches.</done>
</task>

</tasks>

<verification>
- `npx tsc --noEmit` passes (no type errors)
- `grep -r "vocabulary" src/ src-tauri/` returns no matches
- ModelSection.tsx no longer contains any text about "vocabulary prompting"
</verification>

<success_criteria>
- Stale warning block fully removed from ModelSection.tsx
- No compilation errors
- No residual vocabulary prompting references in codebase
</success_criteria>

<output>
After completion, create `.planning/quick/47-remove-stale-parakeet-vocabulary-prompti/47-SUMMARY.md`
</output>
