---
phase: quick-19
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/src/download.rs
  - src-tauri/src/lib.rs
  - src-tauri/src/transcribe_parakeet.rs
  - src/components/FirstRun.tsx
  - src/components/ModelSelector.tsx
  - src/components/sections/ModelSection.tsx
  - src/App.tsx
autonomous: true
requirements: [QUICK-19]

must_haves:
  truths:
    - "No int8 Parakeet model entry appears in list_models output"
    - "No int8 Parakeet model card appears in FirstRun or settings ModelSelector"
    - "download_parakeet_model Tauri command no longer exists"
    - "All Parakeet defaults point to parakeet-tdt-v2-fp32"
    - "App compiles without errors and FirstRun shows 3 GPU model cards"
  artifacts:
    - path: "src-tauri/src/download.rs"
      provides: "Parakeet fp32 download only — no int8 download function or file list"
      contains: "download_parakeet_fp32_model"
    - path: "src-tauri/src/lib.rs"
      provides: "list_models with no int8 entry, defaults changed to fp32"
      contains: "parakeet-tdt-v2-fp32"
    - path: "src/components/FirstRun.tsx"
      provides: "3-card GPU layout with no int8 card"
    - path: "src/components/ModelSelector.tsx"
      provides: "Simplified Parakeet handling — fp32 only"
    - path: "src/App.tsx"
      provides: "Engine reconciliation using fp32 model ID"
  key_links:
    - from: "src-tauri/src/lib.rs"
      to: "src-tauri/src/download.rs"
      via: "download::parakeet_fp32_model_dir() and parakeet_fp32_model_exists()"
      pattern: "parakeet_fp32"
    - from: "src/components/sections/ModelSection.tsx"
      to: "src/components/ModelSelector.tsx"
      via: "fp32 download props only — no int8 parakeet props"
      pattern: "fp32"
    - from: "src/App.tsx"
      to: "invoke('get_engine')"
      via: "Engine reconciliation defaults to fp32"
      pattern: "parakeet-tdt-v2-fp32"
---

<objective>
Remove the Parakeet TDT int8 model variant from the entire codebase — backend download infrastructure, model listing, frontend model cards, and all fallback defaults. After this change, only the fp32 variant exists for Parakeet.

Purpose: The int8 model is superseded by fp32. Keeping it adds UI clutter and code complexity for a variant that should no longer be offered.
Output: Clean codebase with no int8 references in active code paths.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src-tauri/src/download.rs
@src-tauri/src/lib.rs
@src-tauri/src/transcribe_parakeet.rs
@src/components/FirstRun.tsx
@src/components/ModelSelector.tsx
@src/components/sections/ModelSection.tsx
@src/App.tsx
</context>

<tasks>

<task type="auto">
  <name>Task 1: Remove int8 Parakeet from Rust backend</name>
  <files>src-tauri/src/download.rs, src-tauri/src/lib.rs, src-tauri/src/transcribe_parakeet.rs</files>
  <action>
**download.rs:**
- Delete the `PARAKEET_FILES` const (lines 13-19, the int8 file list with encoder-model.int8.onnx entries).
- Delete the `parakeet_model_dir()` function (returns models_dir().join("parakeet-tdt-v2")).
- Delete the `parakeet_model_exists()` function (checks int8 encoder-model.onnx).
- Delete the `parakeet_download_url()` helper function.
- Delete the entire `download_parakeet_model()` async function (the #[tauri::command] that downloads int8 files).
- Update the doc comment on `PARAKEET_FP32_FILES` to remove the "unlike int8 which has .int8. prefix" aside (line 221).
- Update the top-of-file doc comment (line 8) to describe only the fp32 variant.

**lib.rs:**
- `read_saved_parakeet_model()` (line ~195-202): Change default from `"parakeet-tdt-v2"` to `"parakeet-tdt-v2-fp32"`. Update doc comment.
- `read_saved_parakeet_model_startup()` (line ~206-225): Change all four `"parakeet-tdt-v2"` fallback strings to `"parakeet-tdt-v2-fp32"`. Update doc comment.
- `resolve_parakeet_dir()` (line ~228-233): Change the wildcard fallback `_ => download::parakeet_model_dir()` to `_ => download::parakeet_fp32_model_dir()`. Remove "default to int8" comment.
- `list_models()` (line ~899-907): Delete the entire `models.push(ModelInfo { id: "parakeet-tdt-v2" ... })` block for int8.
- `check_first_run()` (line ~946): Remove the `let parakeet_exists = crate::download::parakeet_model_exists();` line and remove `!parakeet_exists` from the `needs_setup` condition. Keep only `!parakeet_fp32_exists`.
- Command registration (line ~1243): Remove `download::download_parakeet_model,` from the invoke_handler list.

**transcribe_parakeet.rs:**
- Update the doc comment on line 14 that says "int8 variant" — change to reference "fp32 variant" or make it generic ("from the istupakov/parakeet-tdt-0.6b-v2-onnx HuggingFace repo").
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text/src-tauri && cargo check 2>&1 | tail -5</automated>
  </verify>
  <done>Rust compiles clean. No "parakeet-tdt-v2" (without -fp32 suffix) string literals remain in lib.rs or download.rs active code. download_parakeet_model function deleted. PARAKEET_FILES const deleted. parakeet_model_dir and parakeet_model_exists functions deleted.</done>
</task>

<task type="auto">
  <name>Task 2: Remove int8 Parakeet from frontend</name>
  <files>src/components/FirstRun.tsx, src/components/ModelSelector.tsx, src/components/sections/ModelSection.tsx, src/App.tsx</files>
  <action>
**FirstRun.tsx:**
- Remove the int8 model entry from the MODELS array (the object with `id: 'parakeet-tdt-v2'`, `name: 'Parakeet TDT (int8)'`).
- Update `gridClass`: GPU users now see 3 models (large-v3-turbo, parakeet-tdt-v2-fp32, small-en), so change `lg:grid-cols-4` to `lg:grid-cols-3`.
- In the `handleDownload` function: remove the `if (modelId === 'parakeet-tdt-v2')` branch (line ~136-137). The `parakeet-tdt-v2-fp32` branch and the else (Whisper download) branch remain.
- In the `useEffect` for download complete: change the condition `downloadingId === 'parakeet-tdt-v2' || downloadingId === 'parakeet-tdt-v2-fp32'` to just `downloadingId === 'parakeet-tdt-v2-fp32'`.

**ModelSelector.tsx:**
- Remove the `onParakeetDownload`, `parakeetDownloading`, `parakeetPercent`, `parakeetError` props from the `ModelSelectorProps` interface. Keep only the `onFp32Download`, `fp32Downloading`, `fp32Percent`, `fp32Error` props.
- Remove the `isParakeetInt8` variable (line 119).
- Change `isParakeetFp32` to just `isParakeet` (since int8 is gone, fp32 IS the only Parakeet). Set: `const isParakeet = model.id === 'parakeet-tdt-v2-fp32';`
- Simplify the per-variant download state resolution: Remove the ternary chains that checked `isParakeetInt8 ? parakeetDownloading : isParakeetFp32 ? fp32Downloading : false`. Instead directly use fp32 state variables: `const thisDownloading = isParakeet ? fp32Downloading : false;` etc.
- Remove the `parakeetDownloading` references from the `disabled` calculation — only `fp32Downloading` remains.
- Update comments: remove "(int8 or fp32)" references, simplify to just "Parakeet download".
- In the function signature destructuring, remove `onParakeetDownload`, `parakeetDownloading = false`, `parakeetPercent = 0`, `parakeetError = null`.

**ModelSection.tsx:**
- Delete the entire `handleParakeetDownload()` function (lines ~87-124) — this handled int8 downloads.
- In `handleModelSelect()`: change `modelId === 'parakeet-tdt-v2' || modelId === 'parakeet-tdt-v2-fp32'` to just `modelId === 'parakeet-tdt-v2-fp32'`. Remove the comment about "int8 -> fp32 or fp32 -> int8" variant switching — simplify to just "variant switch".
- Delete the `parakeetDownloading`, `parakeetPercent`, `parakeetError` state declarations.
- Remove `onParakeetDownload={handleParakeetDownload}`, `parakeetDownloading={parakeetDownloading}`, `parakeetPercent={parakeetPercent}`, `parakeetError={parakeetError}` from the `<ModelSelector>` JSX props.

**App.tsx:**
- Line 68: Change `setSelectedModel('parakeet-tdt-v2')` to `setSelectedModel('parakeet-tdt-v2-fp32')`.
- Line 69: Change `savedSelectedModel !== 'parakeet-tdt-v2'` to `savedSelectedModel !== 'parakeet-tdt-v2-fp32'`.
- Line 74: Change `m.id !== 'parakeet-tdt-v2'` to `m.id !== 'parakeet-tdt-v2-fp32'`.
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && npx tsc --noEmit 2>&1 | tail -10</automated>
  </verify>
  <done>TypeScript compiles clean. No 'parakeet-tdt-v2' (without -fp32 suffix) string literals remain in any .tsx file. ModelSelector no longer accepts int8-specific parakeet props. FirstRun shows 3 GPU cards, not 4. ModelSection has no int8 download handler.</done>
</task>

</tasks>

<verification>
After both tasks:
1. `cargo check` in src-tauri passes with no errors
2. `npx tsc --noEmit` in root passes with no errors
3. Grep for exact string `'parakeet-tdt-v2'` (without fp32 suffix) across src/ and src-tauri/src/ returns zero matches in active code (comments/planning docs excluded)
4. Grep for `download_parakeet_model` (without fp32) returns zero matches in lib.rs invoke_handler
5. Grep for `parakeet_model_dir()` (without fp32) returns zero matches
</verification>

<success_criteria>
- Rust backend compiles cleanly
- TypeScript compiles cleanly
- No int8 Parakeet model ID ("parakeet-tdt-v2" without fp32) in active code
- No int8 download function, file list, or directory helper in download.rs
- All Parakeet defaults resolve to fp32 variant
- FirstRun shows 3 GPU model cards (large-v3-turbo, parakeet-tdt-v2-fp32, small-en)
- ModelSelector/ModelSection only handle fp32 Parakeet downloads
</success_criteria>

<output>
After completion, create `.planning/quick/19-remove-herakeet-int8-model-from-entire-c/19-SUMMARY.md`
</output>
