---
phase: quick-24
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/src/lib.rs
autonomous: true
requirements: [QUICK-24]
must_haves:
  truths:
    - "First launch on GPU system defaults to Parakeet engine (no settings.json yet)"
    - "First launch on CPU-only system defaults to Whisper engine"
    - "Existing users with explicit engine saved in settings.json keep their saved choice"
    - "Missing or corrupt settings.json falls back to GPU-aware default (not hardcoded Whisper)"
  artifacts:
    - path: "src-tauri/src/lib.rs"
      provides: "GPU-aware default engine selection in read_saved_engine()"
      contains: "fn read_saved_engine(app: &tauri::App, gpu_mode: bool)"
  key_links:
    - from: "setup() ~line 1375"
      to: "read_saved_engine()"
      via: "CachedGpuMode state provides gpu_mode bool parameter"
      pattern: "read_saved_engine\\(app,\\s*gpu_mode\\)"
---

<objective>
Change the default transcription engine from hardcoded Whisper to GPU-aware: Parakeet when GPU detected, Whisper otherwise.

Purpose: GPU users get the faster Parakeet engine out of the box on first launch instead of needing to manually switch.
Output: Modified `read_saved_engine()` with `gpu_mode` parameter and updated caller in `setup()`.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src-tauri/src/lib.rs

<interfaces>
<!-- Key types the executor needs -->

From src-tauri/src/lib.rs line 112:
```rust
pub struct CachedGpuMode(pub transcribe::ModelMode);
```

From src-tauri/src/transcribe.rs line 10:
```rust
pub enum ModelMode {
    Cpu,
    Gpu,
}
```

Existing pattern for extracting gpu_mode bool (used at lines 893-894, 1006-1007):
```rust
let cached = app.state::<CachedGpuMode>();
let gpu_mode = matches!(cached.0, ModelMode::Gpu);
```

Current `read_saved_engine` signature (line 179):
```rust
fn read_saved_engine(app: &tauri::App) -> TranscriptionEngine
```

Current caller in setup() (line 1375):
```rust
let saved_engine = read_saved_engine(app);
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Make read_saved_engine GPU-aware and update its caller</name>
  <files>src-tauri/src/lib.rs</files>
  <action>
Two changes in `src-tauri/src/lib.rs`:

**A) Modify `read_saved_engine()` (lines 177-197):**

1. Change signature to `fn read_saved_engine(app: &tauri::App, gpu_mode: bool) -> TranscriptionEngine`

2. At the top of the function body, compute the default:
   ```rust
   let default_engine = if gpu_mode {
       TranscriptionEngine::Parakeet
   } else {
       TranscriptionEngine::Whisper
   };
   ```

3. Replace every `return TranscriptionEngine::Whisper` in the three error paths (lines 182, 187, 191) with `return default_engine`.

4. In the final match on `active_engine` (lines 193-196):
   - Keep `Some("parakeet") => TranscriptionEngine::Parakeet`
   - Add explicit arm: `Some("whisper") => TranscriptionEngine::Whisper`
   - Change catch-all `_ => TranscriptionEngine::Whisper` to `_ => default_engine`

5. Update the doc comment (lines 177-178) to:
   ```rust
   /// Read the saved transcription engine from settings.json.
   /// Defaults to Parakeet when `gpu_mode` is true, Whisper otherwise.
   /// Falls back to the GPU-aware default on first launch, file missing, or parse error.
   ```

**B) Update the caller in `setup()` (~line 1374-1375):**

Inside the existing block (lines 1374-1380), before calling `read_saved_engine`, extract gpu_mode from state:
```rust
{
    use crate::transcribe::ModelMode;
    let cached = app.state::<CachedGpuMode>();
    let gpu_mode = matches!(cached.0, ModelMode::Gpu);
    let saved_engine = read_saved_engine(app, gpu_mode);
    log::info!("Transcription engine (saved): {:?}", saved_engine);
    let engine_state = app.state::<ActiveEngine>();
    let mut guard = engine_state.0.lock().unwrap_or_else(|e| e.into_inner());
    *guard = saved_engine;
}
```

**Do NOT touch:**
- The builder default at line 1297 (`ActiveEngine(... TranscriptionEngine::Whisper)`) -- it is immediately overwritten by setup
- The Parakeet fallback-to-Whisper logic (lines 1420-1438) that checks for missing model files
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && cargo check --manifest-path src-tauri/Cargo.toml --features whisper,parakeet 2>&1 | tail -5</automated>
  </verify>
  <done>
    - `read_saved_engine` accepts `gpu_mode: bool` parameter
    - All error paths return `default_engine` (Parakeet on GPU, Whisper on CPU)
    - Explicit `Some("whisper")` match arm exists
    - Catch-all `_` returns `default_engine` instead of hardcoded Whisper
    - Caller in `setup()` reads `CachedGpuMode` and passes bool to `read_saved_engine`
    - `cargo check` passes with no errors
  </done>
</task>

</tasks>

<verification>
- `cargo check --features whisper,parakeet` compiles without errors or warnings
- `read_saved_engine` signature includes `gpu_mode: bool`
- grep confirms no remaining hardcoded `TranscriptionEngine::Whisper` returns in `read_saved_engine` body (only in explicit `Some("whisper")` match arm)
</verification>

<success_criteria>
- First-launch GPU users will default to Parakeet engine
- First-launch CPU users will default to Whisper engine
- Existing users with saved engine preference are unaffected
- Build compiles cleanly
</success_criteria>

<output>
After completion, create `.planning/quick/24-change-default-engine-from-whisper-to-pa/24-SUMMARY.md`
</output>
