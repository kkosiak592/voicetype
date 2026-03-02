---
phase: quick-18
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/src/lib.rs
autonomous: true
requirements:
  - GPU-WHISPER-01

must_haves:
  truths:
    - "Whisper small.en uses GPU acceleration when NVIDIA GPU is detected"
    - "Whisper small.en falls back to CPU when no NVIDIA GPU is present"
    - "Whisper large-v3-turbo continues to use GPU when NVIDIA GPU is detected"
    - "Model description for small-en reflects GPU capability when GPU is available"
  artifacts:
    - path: "src-tauri/src/lib.rs"
      provides: "CachedGpuMode-based mode selection for all Whisper models"
      contains: "app.state::<CachedGpuMode>()"
  key_links:
    - from: "set_model()"
      to: "CachedGpuMode"
      via: "app.state::<CachedGpuMode>().0.clone()"
      pattern: "app\\.state::<CachedGpuMode>"
    - from: "startup loader (setup)"
      to: "CachedGpuMode"
      via: "app.state::<CachedGpuMode>().0.clone()"
      pattern: "app\\.state::<CachedGpuMode>"
---

<objective>
Enable GPU acceleration for the Whisper small.en model when an NVIDIA GPU is detected, instead of hardcoding it to CPU-only.

Purpose: Currently only large-v3-turbo gets GPU mode. The small.en model is forced to CPU even when a GPU is available, leaving performance on the table. The CachedGpuMode state already holds the correct GPU detection result — it just needs to be used instead of the hardcoded model_id check.

Output: Updated lib.rs where ALL Whisper models use GPU mode based on CachedGpuMode, not based on model_id matching.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
</context>

<interfaces>
<!-- Key types the executor needs -->

From src-tauri/src/lib.rs:
```rust
// Line 112 — cached GPU detection from startup
pub struct CachedGpuMode(pub transcribe::ModelMode);

// ModelMode enum (in transcribe.rs)
// ModelMode::Gpu — NVIDIA GPU detected
// ModelMode::Cpu — no GPU or fallback

// load_whisper_context(model_path, mode) — already handles use_gpu based on mode
```
</interfaces>

<tasks>

<task type="auto">
  <name>Task 1: Replace hardcoded model_id GPU check with CachedGpuMode lookup</name>
  <files>src-tauri/src/lib.rs</files>
  <action>
Three changes in src-tauri/src/lib.rs:

1. **set_model() command (~line 993-998):** Replace the hardcoded model_id check:
   ```rust
   // BEFORE:
   let mode = if model_id == "large-v3-turbo" {
       crate::transcribe::ModelMode::Gpu
   } else {
       crate::transcribe::ModelMode::Cpu
   };
   ```
   With CachedGpuMode lookup:
   ```rust
   let mode = app.state::<CachedGpuMode>().0.clone();
   ```
   This makes set_model() use GPU for ANY Whisper model when GPU is available.

2. **Startup loader (~line 1438-1442):** Replace the hardcoded model_id check in the saved-model loading branch:
   ```rust
   // BEFORE:
   let mode = if model_id == "large-v3-turbo" {
       transcribe::ModelMode::Gpu
   } else {
       transcribe::ModelMode::Cpu
   };
   ```
   With CachedGpuMode lookup:
   ```rust
   let mode = app.state::<CachedGpuMode>().0.clone();
   ```
   Note: The `app` variable in setup() is `&tauri::AppHandle`, so `.state::<CachedGpuMode>()` works the same way it already does at line 1465.

3. **list_models() (~line 889):** Update the small-en description to reflect GPU capability. Change:
   ```rust
   description: "Fastest — 190 MB — works on any CPU".to_string(),
   ```
   To a conditional description based on gpu_mode (the `gpu_mode` bool is already computed at line 875):
   ```rust
   description: if gpu_mode {
       "Fast — 190 MB — GPU accelerated".to_string()
   } else {
       "Fast — 190 MB — works on any CPU".to_string()
   },
   ```

Do NOT change the fallback path at line 1465 — that already uses CachedGpuMode correctly.
  </action>
  <verify>
    <automated>cd src-tauri && cargo check --features whisper 2>&1 | tail -5</automated>
  </verify>
  <done>
    - set_model() uses CachedGpuMode instead of model_id match for GPU mode selection
    - Startup saved-model loader uses CachedGpuMode instead of model_id match
    - small-en description dynamically reflects GPU/CPU capability
    - cargo check passes with no errors
  </done>
</task>

</tasks>

<verification>
- `cargo check --features whisper` compiles without errors
- Grep confirms no remaining `if model_id == "large-v3-turbo"` patterns for GPU mode selection in set_model or startup loader
- The fallback path at ~line 1465 still uses CachedGpuMode (unchanged)
</verification>

<success_criteria>
All Whisper models (small.en and large-v3-turbo) use GPU acceleration when CachedGpuMode is Gpu, and CPU when CachedGpuMode is Cpu. No hardcoded model-to-mode mapping remains for GPU selection.
</success_criteria>

<output>
After completion, create `.planning/quick/18-enable-gpu-acceleration-for-whisper-smal/18-SUMMARY.md`
</output>
