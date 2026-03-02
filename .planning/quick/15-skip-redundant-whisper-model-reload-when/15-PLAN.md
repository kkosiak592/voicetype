---
phase: quick-15
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/src/lib.rs
autonomous: true
requirements: [QUICK-15]
must_haves:
  truths:
    - "Selecting the already-active Whisper model returns immediately without disk reload"
    - "Selecting a different Whisper model still reloads from disk as before"
    - "Settings.json whisper_model_id still persisted correctly on actual model switch"
  artifacts:
    - path: "src-tauri/src/lib.rs"
      provides: "Early-return guard in set_model()"
      contains: "already loaded"
  key_links:
    - from: "set_model()"
      to: "read_settings()"
      via: "whisper_model_id comparison"
      pattern: "whisper_model_id.*model_id"
---

<objective>
Add an early-return guard to set_model() in lib.rs that skips the expensive WhisperContext reload when the requested model_id matches the currently loaded one.

Purpose: Eliminate redundant multi-second disk loads when the user re-selects the same Whisper model (e.g., clicking the already-selected model in ModelSection).
Output: Modified set_model() with same-model short-circuit.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src-tauri/src/lib.rs (lines 918-956: set_model function, lines 782-794: read_saved_model_id, lines 115-130: read_settings/write_settings)
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add same-model early-return guard to set_model()</name>
  <files>src-tauri/src/lib.rs</files>
  <action>
In the `set_model()` function (line 918), add an early-return check at the top of the function body (after the existing `model_path.exists()` check is fine, but before the expensive `load_whisper_context` call).

The guard reads the current whisper_model_id from settings.json via `read_settings()` and compares it to the requested `model_id`. If they match, log an info message and return Ok(()) immediately, skipping the disk reload entirely.

Implementation:

```rust
// Skip reload if the requested model is already loaded
{
    let json = read_settings(&app)?;
    if let Some(current) = json.get("whisper_model_id").and_then(|v| v.as_str()) {
        if current == model_id {
            log::info!("Whisper model '{}' already loaded, skipping reload", model_id);
            return Ok(());
        }
    }
}
```

Insert this block right after the existing `model_path.exists()` check (after line 922) and before the `let path_str = ...` line (line 924). The block is scoped with braces so the json borrow doesn't interfere with the rest of the function.

This mirrors the pattern used by `set_engine()` (line 248-285) which checks `is_none()` before loading Parakeet. The key difference is Whisper uses settings.json as the source of truth for the current model_id rather than an Option check, since WhisperStateMutex is always Some after startup.

Do NOT add any frontend changes — the backend guard handles all callers uniformly.
  </action>
  <verify>
    <automated>cd src-tauri && cargo check --features whisper 2>&1 | tail -5</automated>
  </verify>
  <done>set_model() returns immediately with Ok(()) and a log message when the requested model_id matches the persisted whisper_model_id in settings.json. Different model_id still triggers full reload path. Compiles without warnings.</done>
</task>

</tasks>

<verification>
- `cargo check --features whisper` passes with no errors
- Reading set_model() shows the early-return guard before the spawn_blocking call
- The guard uses settings.json whisper_model_id as source of truth (consistent with how model_id is persisted)
</verification>

<success_criteria>
- Redundant Whisper model reloads eliminated when re-selecting the same model
- Actual model switches (different model_id) still work as before
- No new state types or Tauri managed state required
</success_criteria>

<output>
After completion, create `.planning/quick/15-skip-redundant-whisper-model-reload-when/15-SUMMARY.md`
</output>
