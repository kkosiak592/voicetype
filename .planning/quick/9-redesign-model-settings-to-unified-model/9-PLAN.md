---
phase: quick-9
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/components/sections/ModelSection.tsx
  - src/components/ModelSelector.tsx
autonomous: false
requirements: [QUICK-9]
must_haves:
  truths:
    - "All models (Large v3 Turbo, Parakeet TDT, Small English) appear as cards in a single flat list"
    - "No 'Transcription Engine' toggle buttons are visible anywhere in settings"
    - "Selecting Parakeet TDT calls set_engine('parakeet') behind the scenes"
    - "Selecting any Whisper model calls set_engine('whisper') behind the scenes"
    - "Undownloaded Parakeet shows a download button on its card with dashed border, 661 MB label, and progress bar"
    - "Vocabulary prompting disclaimer appears when Parakeet is the active engine"
  artifacts:
    - path: "src/components/sections/ModelSection.tsx"
      provides: "Unified model section without engine toggle"
    - path: "src/components/ModelSelector.tsx"
      provides: "Model cards for all engines with Parakeet download integration"
  key_links:
    - from: "ModelSelector card click"
      to: "invoke('set_engine')"
      via: "onSelect callback in ModelSection"
      pattern: "set_engine.*parakeet|set_engine.*whisper"
    - from: "Parakeet download button"
      to: "invoke('download_parakeet_model')"
      via: "handleDownload in ModelSelector"
      pattern: "download_parakeet_model"
---

<objective>
Redesign the Model settings section to show all transcription models (Whisper and Parakeet) in a single flat list of cards. Engine switching happens implicitly when the user picks a model. Remove the separate "Transcription Engine" toggle entirely.

Purpose: Simplify the UX by removing the two-level engine/model hierarchy. Users just pick a model; the app handles the engine routing.
Output: Updated ModelSection.tsx and ModelSelector.tsx with unified model list and implicit engine switching.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md

<interfaces>
<!-- Backend commands used by these components -->

From src-tauri/src/lib.rs:
- `list_models` returns Vec<ModelInfo> where ModelInfo has { id, name, description, recommended, downloaded }
  - Returns all models including parakeet-tdt-v2 in a single list
- `set_model(modelId: String)` — sets active Whisper model
- `set_engine(engine: String)` — sets engine to "whisper" or "parakeet"
- `get_engine()` returns String — current engine name
- `download_model(modelId, onEvent: Channel)` — downloads Whisper models
- `download_parakeet_model(onEvent: Channel)` — downloads Parakeet model files (661 MB, 5 files)

From src/components/ModelSelector.tsx:
```typescript
export interface ModelInfo {
  id: string;
  name: string;
  description: string;
  recommended: boolean;
  downloaded: boolean;
}
```

From src/components/sections/ModelSection.tsx:
```typescript
interface ModelSectionProps {
  selectedModel: string;
  onSelectedModelChange: (id: string) => void;
}
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Unify ModelSelector to handle all models including Parakeet download</name>
  <files>src/components/ModelSelector.tsx</files>
  <action>
Modify ModelSelector to support Parakeet TDT as a first-class model card alongside Whisper models:

1. Add an optional `parakeetDownloadHandler` prop: `onParakeetDownload?: () => void` and related state props:
   - `parakeetDownloading?: boolean`
   - `parakeetPercent?: number`
   - `parakeetError?: string | null`

2. In the model card rendering loop, when a model is NOT downloaded:
   - If model.id === 'parakeet-tdt-v2' AND `onParakeetDownload` is provided: render the card with a dashed border (`border-dashed`) instead of solid, show "661 MB" in the description (already in model.description from backend), and wire the Download button to call `onParakeetDownload()` instead of the normal `handleDownload`.
   - Show the Parakeet progress bar using `parakeetPercent` when `parakeetDownloading` is true, same style as existing download progress.
   - Show `parakeetError` if set, with the same error styling as existing whisper download errors.

3. For all OTHER undownloaded models, keep existing download behavior (calls `download_model` via `handleDownload`).

4. Keep all existing ModelSelector behavior intact: selection, loading states, download progress for Whisper models.

The card styling for an undownloaded Parakeet model should use `border-dashed border-gray-300 dark:border-gray-600` to visually indicate it needs downloading, matching the current dashed-border pattern from ModelSection.
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && npx tsc --noEmit 2>&1 | head -30</automated>
  </verify>
  <done>ModelSelector renders Parakeet TDT as a card with dashed border when undownloaded, wires to parakeet download handler, shows progress/error inline. All other models unchanged.</done>
</task>

<task type="auto">
  <name>Task 2: Rewrite ModelSection to flat unified model list with implicit engine switching</name>
  <files>src/components/sections/ModelSection.tsx</files>
  <action>
Rewrite ModelSection to remove the engine toggle and present all models in one flat list:

1. **Remove entirely:**
   - The `hasGpu` engine selector block (lines 130-167 — the "Transcription Engine" heading with Whisper/Parakeet toggle buttons)
   - The separate Parakeet download prompt block (lines 169-197 — dashed border download card)
   - The `whisperModels` filter — stop filtering out parakeet-tdt-v2 from the model list

2. **Pass ALL models to ModelSelector** (not just `whisperModels`). The full `models` array from `list_models` already includes parakeet-tdt-v2.

3. **Modify `handleModelSelect`** to include implicit engine switching:
   ```typescript
   async function handleModelSelect(modelId: string) {
     try {
       // Determine the correct engine based on the selected model
       const engine = modelId === 'parakeet-tdt-v2' ? 'parakeet' : 'whisper';

       // Switch engine if needed
       if (engine !== currentEngine) {
         await invoke('set_engine', { engine });
         setCurrentEngine(engine);
       }

       // For Whisper models, also call set_model
       if (engine === 'whisper') {
         await invoke('set_model', { modelId });
       }

       const store = await getStore();
       await store.set('selectedModel', modelId);
       onSelectedModelChange(modelId);
     } catch (err) {
       console.error('Failed to set model:', err);
     }
   }
   ```

4. **Wire Parakeet download to ModelSelector** via the new props:
   - Pass `parakeetDownloading`, `parakeetPercent`, `parakeetError` state
   - Pass `onParakeetDownload={handleParakeetDownload}` prop
   - On `handleParakeetDownload` finished event, call `loadModels()` to refresh, then auto-select parakeet: call `handleModelSelect('parakeet-tdt-v2')` which will implicitly set the engine

5. **Update `handleDownloadComplete`** (for Whisper models) to also call the engine-aware `handleModelSelect` instead of the old version.

6. **Keep the vocabulary prompting disclaimer** but move it below the ModelSelector, conditioned on `currentEngine === 'parakeet'`:
   ```tsx
   {currentEngine === 'parakeet' && (
     <p className="mt-3 text-xs text-gray-400 dark:text-gray-500">
       Parakeet doesn't support vocabulary prompting. Your corrections dictionary still applies.
     </p>
   )}
   ```

7. **Remove `handleEngineChange`** function entirely — no longer needed.

8. **Keep** the `loadEngine` call in useEffect — still need to know current engine for the disclaimer and for `handleModelSelect` logic.

9. **Track selected model for Parakeet**: When `currentEngine === 'parakeet'`, the selectedModel indicator should highlight the parakeet-tdt-v2 card. Ensure the `selectedModel` prop passed to ModelSelector can be 'parakeet-tdt-v2' (it should already work since it comes from the store).
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && npx tsc --noEmit 2>&1 | head -30</automated>
  </verify>
  <done>ModelSection shows a single flat list of all models. No engine toggle visible. Selecting Parakeet auto-switches engine to "parakeet". Selecting Whisper models auto-switches to "whisper". Parakeet download card has dashed border with progress bar. Disclaimer shows when Parakeet is active.</done>
</task>

<task type="checkpoint:human-verify" gate="blocking">
  <name>Task 3: Verify unified model settings UI</name>
  <files>src/components/sections/ModelSection.tsx, src/components/ModelSelector.tsx</files>
  <action>
Human verifies the redesigned Model settings section:
1. Open Settings > Model section
2. Verify NO "Transcription Engine" toggle buttons appear
3. Verify all three models appear in a single list: Large v3 Turbo, Small (English), Parakeet TDT
4. If Parakeet is not downloaded: verify its card has a dashed border with a Download button and "661 MB" label
5. Click a Whisper model — verify it selects normally
6. If Parakeet is downloaded: click Parakeet TDT — verify the disclaimer about vocabulary prompting appears below the list
7. Switch back to a Whisper model — verify disclaimer disappears
  </action>
  <verify>User types "approved" or describes issues</verify>
  <done>All visual and functional checks pass. Unified model list works correctly with implicit engine switching.</done>
</task>

</tasks>

<verification>
- TypeScript compiles without errors: `npx tsc --noEmit`
- No "Transcription Engine" text in ModelSection.tsx
- No `whisperModels` filter in ModelSection.tsx
- `set_engine` is called inside `handleModelSelect` based on model ID
- Parakeet download still functional via ModelSelector card
</verification>

<success_criteria>
- Single flat list of all models in settings (no engine toggle)
- Clicking Parakeet TDT implicitly sets engine to "parakeet"
- Clicking any Whisper model implicitly sets engine to "whisper"
- Undownloaded Parakeet shows dashed-border card with download button and progress bar
- Vocabulary prompting disclaimer shows when Parakeet is active engine
- TypeScript compiles cleanly
</success_criteria>

<output>
After completion, create `.planning/quick/9-redesign-model-settings-to-unified-model/9-SUMMARY.md`
</output>
