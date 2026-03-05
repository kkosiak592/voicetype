---
phase: quick-45
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/components/sections/GeneralSection.tsx
autonomous: true
requirements: [QUICK-45]

must_haves:
  truths:
    - "DictionaryEditor is visible in General settings under a Corrections Dictionary card"
    - "User can add, edit, and delete correction entries that persist via Tauri backend"
  artifacts:
    - path: "src/components/sections/GeneralSection.tsx"
      provides: "DictionaryEditor integration with load/save via invoke"
  key_links:
    - from: "src/components/sections/GeneralSection.tsx"
      to: "src/components/DictionaryEditor.tsx"
      via: "import { DictionaryEditor }"
      pattern: "DictionaryEditor"
    - from: "src/components/sections/GeneralSection.tsx"
      to: "src-tauri/src/lib.rs"
      via: "invoke('get_corrections') and invoke('save_corrections')"
      pattern: "invoke.*corrections"
---

<objective>
Wire the orphaned DictionaryEditor component into the General settings section.

Purpose: The DictionaryEditor (two-column From/To correction table) exists at src/components/DictionaryEditor.tsx but is not rendered anywhere after profile simplification removed its parent. The Tauri backend already has get_corrections and save_corrections IPC commands registered. This plan adds the component to GeneralSection as a third card.

Output: DictionaryEditor visible and functional in General > Corrections Dictionary card.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src/components/DictionaryEditor.tsx
@src/components/sections/GeneralSection.tsx
@src/App.tsx

<interfaces>
From src/components/DictionaryEditor.tsx:
```typescript
interface DictionaryEditorProps {
  corrections: Record<string, string>;
  onChange: (corrections: Record<string, string>) => void;
}
export function DictionaryEditor({ corrections, onChange }: DictionaryEditorProps): JSX.Element;
```

From src-tauri/src/lib.rs (Tauri IPC commands, already registered):
```rust
#[tauri::command]
fn get_corrections(app: tauri::AppHandle) -> Result<HashMap<String, String>, String>;

#[tauri::command]
fn save_corrections(app: tauri::AppHandle, corrections: HashMap<String, String>) -> Result<(), String>;
```

GeneralSection currently has two cards:
- Card 1: Activation (hotkey, recording mode, always listen)
- Card 2: Output (ALL CAPS, remove fillers)
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add DictionaryEditor card to GeneralSection with IPC load/save</name>
  <files>src/components/sections/GeneralSection.tsx</files>
  <action>
Modify GeneralSection.tsx to:

1. Add imports at top:
   - `import { useState, useEffect } from 'react';`
   - `import { invoke } from '@tauri-apps/api/core';`
   - `import { DictionaryEditor } from '../DictionaryEditor';`

2. Inside the GeneralSection component, add state and effects:
   - `const [corrections, setCorrections] = useState<Record<string, string>>({});`
   - useEffect that calls `invoke<Record<string, string>>('get_corrections')` on mount and sets state. Catch errors silently (log to console).

3. Create a handler function:
   ```typescript
   async function handleCorrectionsChange(updated: Record<string, string>) {
     setCorrections(updated);
     try {
       await invoke('save_corrections', { corrections: updated });
     } catch (err) {
       console.error('Failed to save corrections:', err);
     }
   }
   ```

4. After the Card 2 (Output) div, add a Card 3 with the same styling pattern:
   ```tsx
   {/* Card 3: Corrections Dictionary */}
   <div className="bg-white dark:bg-gray-900 ring-1 ring-gray-200 dark:ring-gray-800 rounded-2xl p-4 shadow-sm">
     <section>
       <h2 className="text-sm font-semibold text-gray-900 dark:text-gray-100">
         Corrections Dictionary
       </h2>
       <p className="mb-4 mt-1 text-sm text-gray-500 dark:text-gray-400">
         Fix recurring transcription mistakes. Matched words are automatically replaced.
       </p>
       <DictionaryEditor corrections={corrections} onChange={handleCorrectionsChange} />
     </section>
   </div>
   ```

Keep the existing GeneralSectionProps interface and component signature unchanged -- corrections state is self-contained within GeneralSection (loaded from IPC, not lifted to App.tsx).
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && npx tsc --noEmit 2>&1 | head -20</automated>
  </verify>
  <done>GeneralSection renders DictionaryEditor in a third card. Corrections load from get_corrections on mount and save via save_corrections on change. TypeScript compiles without errors.</done>
</task>

</tasks>

<verification>
- `npx tsc --noEmit` passes with no errors
- DictionaryEditor import is used (no unused import warning)
- GeneralSection renders three cards: Activation, Output, Corrections Dictionary
</verification>

<success_criteria>
- DictionaryEditor component is imported and rendered in GeneralSection
- Corrections load from Tauri backend on mount via invoke('get_corrections')
- Corrections save to Tauri backend on change via invoke('save_corrections')
- Existing GeneralSection functionality (hotkey, recording mode, toggles) unchanged
- TypeScript compiles cleanly
</success_criteria>

<output>
After completion, create `.planning/quick/45-wire-orphaned-dictionaryeditor-back-into/45-SUMMARY.md`
</output>
