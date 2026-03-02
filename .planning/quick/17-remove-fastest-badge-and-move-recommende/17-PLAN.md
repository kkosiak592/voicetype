---
phase: quick-17
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/src/lib.rs
  - src/components/FirstRun.tsx
autonomous: true
requirements: [QUICK-17]
must_haves:
  truths:
    - "No 'Fastest' badge appears on any model card in FirstRun or settings"
    - "Parakeet TDT (fp32) shows 'Recommended' badge in FirstRun when GPU detected"
    - "Parakeet TDT (fp32) shows 'Recommended' pill in ModelSelector settings when GPU detected"
    - "Large v3 Turbo no longer shows Recommended badge anywhere"
  artifacts:
    - path: "src-tauri/src/lib.rs"
      provides: "Updated recommended_model and recommended flags"
      contains: "recommended: gpu_mode"
    - path: "src/components/FirstRun.tsx"
      provides: "Fastest badge removed, Recommended now targets fp32"
  key_links:
    - from: "src-tauri/src/lib.rs check_first_run"
      to: "src/components/FirstRun.tsx recommendedModel prop"
      via: "recommended_model field in FirstRunStatus"
      pattern: "parakeet-tdt-v2-fp32"
    - from: "src-tauri/src/lib.rs list_models"
      to: "src/components/ModelSelector.tsx recommended badge"
      via: "recommended bool on ModelInfo"
      pattern: "recommended.*gpu_mode"
---

<objective>
Remove the "Fastest" badge from the FirstRun model cards entirely, and move the "Recommended" badge from Large v3 Turbo to Parakeet TDT (fp32) when a GPU is detected.

Purpose: The fp32 Parakeet model is the preferred choice for GPU users — the badge should reflect this.
Output: Updated backend recommendation logic and frontend badge rendering.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md

<interfaces>
<!-- Backend: lib.rs ModelInfo and FirstRunStatus -->
From src-tauri/src/lib.rs:
```rust
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ModelInfo {
    id: String,
    name: String,
    description: String,
    recommended: bool,
    downloaded: bool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct FirstRunStatus {
    needs_setup: bool,
    gpu_detected: bool,
    recommended_model: String,
}
```

From src/components/FirstRun.tsx:
```typescript
interface FirstRunProps {
  gpuDetected: boolean;
  recommendedModel: string;
  onComplete: (downloadedModelId: string) => void;
}
// recommendedModel drives isRecommended = model.id === recommendedModel
// isFastest = model.id === 'parakeet-tdt-v2' && gpuDetected (TO BE REMOVED)
```

From src/components/ModelSelector.tsx:
```typescript
export interface ModelInfo {
  id: string;
  name: string;
  description: string;
  recommended: boolean;  // drives "Recommended" pill in settings
  downloaded: boolean;
}
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Update backend recommendation to Parakeet TDT fp32</name>
  <files>src-tauri/src/lib.rs</files>
  <action>
In `list_models()` (around line 872):
1. Change `large-v3-turbo` entry: set `recommended: false` (was `gpu_mode`)
2. Change `small-en` entry: keep `recommended: !gpu_mode` (unchanged — still recommended for CPU users)
3. Change `parakeet-tdt-v2-fp32` entry: set `recommended: gpu_mode` (was `false`)
4. Leave `parakeet-tdt-v2` (int8) entry: keep `recommended: false` (unchanged)

In `check_first_run()` (around line 934):
5. Change `recommended_model` for GPU mode from `"large-v3-turbo".to_string()` to `"parakeet-tdt-v2-fp32".to_string()`
6. Keep CPU mode as `"small-en".to_string()` (unchanged)

Remove or update the comment on line 901 that says "Whisper is recommended per locked decision" — this is no longer accurate.
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && cargo check --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5</automated>
  </verify>
  <done>Backend returns recommended=true for parakeet-tdt-v2-fp32 on GPU, recommended_model="parakeet-tdt-v2-fp32" in FirstRunStatus on GPU. Large v3 Turbo no longer recommended.</done>
</task>

<task type="auto">
  <name>Task 2: Remove Fastest badge and update FirstRun card text</name>
  <files>src/components/FirstRun.tsx</files>
  <action>
1. Delete line 209: `const isFastest = model.id === 'parakeet-tdt-v2' && gpuDetected;`
2. Delete lines 234-237 (the Fastest badge JSX):
   ```tsx
   {isFastest && (
     <span className="absolute -top-2.5 right-3 rounded-full bg-green-500 px-2 py-0.5 text-xs font-semibold text-white">
       Fastest
     </span>
   )}
   ```
3. In the MODELS array, update the `parakeet-tdt-v2` (int8) entry quality text from `'Fastest (GPU)'` to `'Fast (GPU)'` — it is still fast, but removing the superlative since we removed the badge.

No other changes needed — the `isRecommended` logic already uses `model.id === recommendedModel` which will now match `parakeet-tdt-v2-fp32` because the backend change in Task 1 returns that as `recommended_model`.
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && npx tsc --noEmit 2>&1 | tail -5</automated>
  </verify>
  <done>No "Fastest" badge rendered anywhere. Parakeet TDT int8 card says "Fast (GPU)" instead of "Fastest (GPU)". Recommended badge appears on fp32 card when GPU detected.</done>
</task>

</tasks>

<verification>
1. `cargo check` passes (backend compiles)
2. `npx tsc --noEmit` passes (frontend compiles)
3. In FirstRun with GPU: fp32 card has "Recommended" badge, no card has "Fastest" badge
4. In settings ModelSelector with GPU: fp32 model shows "Recommended" pill, Large v3 Turbo does not
</verification>

<success_criteria>
- Fastest badge completely removed from codebase (no references to isFastest)
- Recommended badge targets parakeet-tdt-v2-fp32 on GPU in both FirstRun and settings
- CPU mode still recommends small-en (unchanged)
- Both Rust and TypeScript compile without errors
</success_criteria>

<output>
After completion, create `.planning/quick/17-remove-fastest-badge-and-move-recommende/17-SUMMARY.md`
</output>
