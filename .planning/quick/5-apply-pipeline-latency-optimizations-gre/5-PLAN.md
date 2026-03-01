---
phase: quick-5
plan: 5
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/src/transcribe.rs
  - src-tauri/src/inject.rs
autonomous: false
must_haves:
  truths:
    - "Whisper inference uses greedy decoding (best_of=1) instead of beam search"
    - "Flash attention is enabled on WhisperContext initialization"
    - "Temperature fallback, multi-segment search, and context carryover are disabled"
    - "Injection delays reduced from 195ms total to 80ms total"
  artifacts:
    - path: "src-tauri/src/transcribe.rs"
      provides: "Optimized whisper inference parameters"
      contains: "SamplingStrategy::Greedy"
    - path: "src-tauri/src/inject.rs"
      provides: "Reduced injection delays"
      contains: "from_millis(30)"
  key_links:
    - from: "src-tauri/src/transcribe.rs"
      to: "whisper-rs inference"
      via: "FullParams + WhisperContextParameters"
      pattern: "Greedy.*best_of.*1"
---

<objective>
Apply pipeline latency optimizations to cut end-to-end transcription time by 40-60%.

Purpose: Current pipeline takes 800-1200ms (600-1000ms whisper inference with beam_size=5, plus 195ms injection delays). These parameter changes should bring it to 400-700ms with zero model changes.

Output: Modified transcribe.rs (greedy decoding, flash attention, single segment, no context, no temperature fallback) and inject.rs (reduced sleep durations).
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@artifacts/research/2026-02-28-pipeline-latency-optimization-technical.md

<interfaces>
<!-- Current code that will be modified -->

From src-tauri/src/transcribe.rs (lines 108-131, load_whisper_context):
```rust
pub fn load_whisper_context(model_path: &str, mode: &ModelMode) -> Result<WhisperContext, String> {
    let start = Instant::now();
    let use_gpu = matches!(mode, ModelMode::Gpu);
    let mut ctx_params = WhisperContextParameters::default();
    ctx_params.use_gpu(use_gpu);
    let ctx = WhisperContext::new_with_params(model_path, ctx_params)
        .map_err(|e| format!("Failed to load whisper model from '{}': {}", model_path, e))?;
    Ok(ctx)
}
```

From src-tauri/src/transcribe.rs (lines 140-178, transcribe_audio):
```rust
pub fn transcribe_audio(ctx: &WhisperContext, audio: &[f32]) -> Result<String, String> {
    let start = Instant::now();
    let mut state = ctx.create_state().map_err(|e| e.to_string())?;
    let mut params = FullParams::new(SamplingStrategy::BeamSearch {
        beam_size: 5,
        patience: -1.0,
    });
    params.set_language(Some("en"));
    params.set_temperature(0.0);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    state.full(params, audio).map_err(|e| e.to_string())?;
    // ... segment collection ...
}
```

From src-tauri/src/inject.rs (lines 18-59, inject_text):
```rust
pub fn inject_text(text: &str) -> Result<(), String> {
    // ... clipboard save ...
    thread::sleep(Duration::from_millis(75));  // line 29
    // ... Ctrl+V simulation ...
    thread::sleep(Duration::from_millis(120)); // line 39
    // ... clipboard restore ...
}
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Optimize whisper inference parameters and reduce injection delays</name>
  <files>src-tauri/src/transcribe.rs, src-tauri/src/inject.rs</files>
  <action>
  In `src-tauri/src/transcribe.rs`, make these changes:

  1. **Enable flash attention** in `load_whisper_context()` (after line 119):
     Add `ctx_params.flash_attn(true);` after the `ctx_params.use_gpu(use_gpu);` line.
     Update the doc comment on the function to mention flash attention.

  2. **Switch to greedy decoding** in `transcribe_audio()` (lines 145-148):
     Replace:
     ```rust
     let mut params = FullParams::new(SamplingStrategy::BeamSearch {
         beam_size: 5,
         patience: -1.0,
     });
     ```
     With:
     ```rust
     let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
     ```

  3. **Add parameter optimizations** in `transcribe_audio()` after the existing `set_print_timestamps(false)` line (after line 154):
     ```rust
     params.set_single_segment(true);   // short dictation = one segment
     params.set_no_context(true);       // no prior context carryover
     params.set_temperature_inc(0.0);   // disable temperature fallback retries
     ```

  4. **Update doc comment** on `transcribe_audio()` (line 134-137): change "beam search (beam_size=5)" to "greedy decoding" and mention the single-segment optimization.

  In `src-tauri/src/inject.rs`, make these changes:

  5. **Reduce clipboard propagation delay** (line 29):
     Change `Duration::from_millis(75)` to `Duration::from_millis(30)`.
     Update the comment to: `// 30ms clipboard propagation (reduced from 75ms — revert to 50ms if any app drops pastes)`

  6. **Reduce paste consumption delay** (line 39):
     Change `Duration::from_millis(120)` to `Duration::from_millis(50)`.
     Update the comment to: `// 50ms paste consumption (reduced from 120ms — revert to 80ms if any app drops pastes)`
  </action>
  <verify>
    <automated>cd "C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text" && grep -c "Greedy" src-tauri/src/transcribe.rs && grep -c "flash_attn" src-tauri/src/transcribe.rs && grep -c "set_single_segment" src-tauri/src/transcribe.rs && grep -c "set_no_context" src-tauri/src/transcribe.rs && grep -c "set_temperature_inc" src-tauri/src/transcribe.rs && grep "from_millis(30)" src-tauri/src/inject.rs && grep "from_millis(50)" src-tauri/src/inject.rs</automated>
  </verify>
  <done>
  - transcribe.rs uses SamplingStrategy::Greedy { best_of: 1 } instead of BeamSearch
  - transcribe.rs has flash_attn(true) in load_whisper_context
  - transcribe.rs has set_single_segment(true), set_no_context(true), set_temperature_inc(0.0)
  - inject.rs uses 30ms + 50ms delays (80ms total, down from 195ms)
  - All doc comments updated to reflect new parameters
  </done>
</task>

<task type="checkpoint:human-verify" gate="blocking">
  <what-built>
  Pipeline latency optimizations applied:
  - Greedy decoding (was beam_size=5) — expected 30-50% inference speedup
  - Flash attention enabled — expected 10-20% additional inference speedup
  - Single segment + no context + no temperature fallback — eliminates unnecessary computation
  - Injection delays reduced from 195ms to 80ms — saves 115ms fixed overhead

  Combined expected improvement: 800-1200ms down to 400-700ms total pipeline latency.
  </what-built>
  <how-to-verify>
  1. Build and run: `npx tauri dev` (or your usual build process via build-whisper.ps1 if needed)
  2. Open the Tauri dev console to see log output
  3. Record a test phrase using hold-to-talk (say "The quick brown fox jumps over the lazy dog")
  4. Check the log for "Transcription completed in Xms" — compare to your prior baseline (~600-1000ms)
  5. Verify the transcription text is accurate (greedy should produce same quality for short phrases)
  6. Test paste injection in your daily apps:
     - VS Code: type in a file
     - Chrome: type in a text field (Google search, chat input)
     - Notepad: basic paste test
     - Any other app you use regularly
  7. If any app drops pastes (text doesn't appear), report which app — the injection delays can be bumped back up

  **If flash attention causes issues on P2000 (Pascal GPU):**
  The flash_attn(true) line can be reverted independently. If inference time is SLOWER or errors appear, report it.
  </how-to-verify>
  <resume-signal>Type "approved" with observed inference times, or describe any issues (paste failures, accuracy problems, flash attention issues)</resume-signal>
</task>

</tasks>

<verification>
- `grep "Greedy" src-tauri/src/transcribe.rs` shows greedy decoding
- `grep "flash_attn" src-tauri/src/transcribe.rs` shows flash attention enabled
- `grep "from_millis" src-tauri/src/inject.rs` shows 30ms and 50ms delays
- No BeamSearch references remain in transcribe_audio function
- Log output shows reduced "Transcription completed in Xms" values
</verification>

<success_criteria>
- Whisper inference time reduced by 30-50% (from ~600-1000ms baseline)
- Injection overhead reduced from 195ms to 80ms
- Transcription accuracy unchanged for typical English dictation
- Paste injection works in VS Code, Chrome, and Notepad at minimum
</success_criteria>

<output>
After completion, create `.planning/quick/5-apply-pipeline-latency-optimizations-gre/5-SUMMARY.md`
</output>
