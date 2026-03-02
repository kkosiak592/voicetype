---
phase: quick-21
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/src/lib.rs
  - src/components/FirstRun.tsx
autonomous: true
requirements: [QUICK-21]

must_haves:
  truths:
    - "Settings page shows 'Most accurate' for Large v3 Turbo, not 'Best accuracy'"
    - "Settings page shows 'Lightweight' for Small, not 'Fast'"
    - "Settings page shows 'GPU accelerated (CUDA or DirectML)' for Parakeet, not 'requires NVIDIA GPU (ONNX)'"
    - "First Run page shows 'Most accurate' for Large v3 Turbo, not 'Best accuracy'"
    - "First Run page shows 'Fast and accurate' for Parakeet, not 'Full precision'"
  artifacts:
    - path: "src-tauri/src/lib.rs"
      provides: "Updated model descriptions for Settings ModelSelector"
      contains: "Most accurate"
    - path: "src/components/FirstRun.tsx"
      provides: "Updated model quality labels for First Run cards"
      contains: "Most accurate"
  key_links:
    - from: "src-tauri/src/lib.rs"
      to: "src/components/ModelSelector.tsx"
      via: "list_models Tauri command returns ModelInfo.description"
      pattern: "model\\.description"
---

<objective>
Update model description text across both the Settings page (backend ModelInfo descriptions rendered by ModelSelector) and the First Run page (frontend MODELS array) to use accurate, consistent wording.

Purpose: Current descriptions contain inaccuracies (Parakeet says "requires NVIDIA GPU (ONNX)" but it supports CUDA, DirectML, and CPU) and inconsistent quality labels between the two screens.
Output: Consistent, accurate model descriptions in both screens.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src-tauri/src/lib.rs (lines 882-919 — list_models function)
@src/components/FirstRun.tsx (lines 20-45 — MODELS array)
@src/components/ModelSelector.tsx (line 194 — renders model.description)
</context>

<tasks>

<task type="auto">
  <name>Task 1: Update backend model descriptions in list_models</name>
  <files>src-tauri/src/lib.rs</files>
  <action>
In the `list_models` function (around line 882), update the three model descriptions:

1. **Large v3 Turbo** (line 892): Change description from
   `"Best accuracy — 574 MB — requires NVIDIA GPU"` to
   `"Most accurate — 574 MB — requires NVIDIA GPU"`

2. **Small (English)** (lines 899-903): Change the conditional descriptions from
   - GPU branch: `"Fast — 190 MB — GPU accelerated"` to `"Lightweight — 190 MB — GPU accelerated when available"`
   - CPU branch: `"Fast — 190 MB — works on any CPU"` to `"Lightweight — 190 MB — GPU accelerated when available"`
   Since both branches now have the same text, remove the `if gpu_mode` conditional entirely and use a single string: `"Lightweight — 190 MB — GPU accelerated when available".to_string()`

3. **Parakeet TDT fp32** (line 913): Change description from
   `"Full precision — 2.56 GB — requires NVIDIA GPU (ONNX)"` to
   `"Fast and accurate — 2.56 GB — GPU accelerated (CUDA or DirectML)"`
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && cargo check --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5</automated>
  </verify>
  <done>
    - Large v3 Turbo description reads "Most accurate — 574 MB — requires NVIDIA GPU"
    - Small description reads "Lightweight — 190 MB — GPU accelerated when available" (no conditional)
    - Parakeet description reads "Fast and accurate — 2.56 GB — GPU accelerated (CUDA or DirectML)"
  </done>
</task>

<task type="auto">
  <name>Task 2: Update FirstRun MODELS array descriptions</name>
  <files>src/components/FirstRun.tsx</files>
  <action>
In the `MODELS` array at the top of FirstRun.tsx (lines 20-45), update two model entries:

1. **Large v3 Turbo** (line 25): Change `quality` from `'Best accuracy'` to `'Most accurate'`.
   The `requirement` field (`'NVIDIA GPU required'`) is already correct — leave it.

2. **Parakeet TDT fp32** (line 33): Change `quality` from `'Full precision'` to `'Fast and accurate'`.
   The `requirement` field (`'GPU accelerated (CUDA or DirectML)'`) is already correct — leave it.

3. **Small (English)** — both `quality` (`'Fast and lightweight'`) and `requirement` (`'GPU accelerated when available'`) are already correct. No changes needed.
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && npx tsc --noEmit 2>&1 | tail -5</automated>
  </verify>
  <done>
    - Large v3 Turbo quality reads "Most accurate"
    - Parakeet quality reads "Fast and accurate"
    - Small (English) unchanged (already correct)
    - All three models render with accurate, consistent descriptions across both Settings and First Run screens
  </done>
</task>

</tasks>

<verification>
- `cargo check` passes for backend changes
- `npx tsc --noEmit` passes for frontend changes
- Grep confirms no remaining instances of "Best accuracy" or "Full precision" or "requires NVIDIA GPU (ONNX)" in source files
</verification>

<success_criteria>
Model descriptions are accurate and consistent:
- Large v3 Turbo: "Most accurate" in both screens, NVIDIA GPU requirement
- Small: "Lightweight" in Settings, "Fast and lightweight" in First Run, GPU accelerated when available
- Parakeet: "Fast and accurate" in both screens, GPU accelerated (CUDA or DirectML)
</success_criteria>

<output>
After completion, create `.planning/quick/21-update-model-descriptions-across-setting/21-SUMMARY.md`
</output>
