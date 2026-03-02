---
phase: quick-11
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/src/transcribe_parakeet.rs
autonomous: true
requirements: []
must_haves:
  truths:
    - "Parakeet transcription uses TimestampMode::Sentences instead of default Tokens"
    - "Word deduplication runs on transcription output via group_by_sentences -> deduplicate_words"
  artifacts:
    - path: "src-tauri/src/transcribe_parakeet.rs"
      provides: "Parakeet transcription with Sentences timestamp mode"
      contains: "TimestampMode::Sentences"
  key_links:
    - from: "src-tauri/src/transcribe_parakeet.rs"
      to: "parakeet_rs::TimestampMode"
      via: "use import and Some(TimestampMode::Sentences) argument"
      pattern: "TimestampMode::Sentences"
---

<objective>
Switch Parakeet TDT transcription from the default TimestampMode::Tokens to TimestampMode::Sentences.

Purpose: The Sentences mode triggers group_by_sentences -> group_by_words -> deduplicate_words in parakeet-rs timestamps.rs, which strips consecutive repeated words from transcription output. The TDT model predicts punctuation, making sentence-level grouping the recommended mode per the parakeet-rs docs.

Output: Modified transcribe_parakeet.rs with Sentences timestamp mode.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src-tauri/src/transcribe_parakeet.rs
@src-tauri/patches/parakeet-rs/src/timestamps.rs
</context>

<interfaces>
<!-- From parakeet-rs lib.rs (top-level re-exports): -->
```rust
pub use timestamps::TimestampMode;
// TimestampMode::Tokens | TimestampMode::Words | TimestampMode::Sentences
```

<!-- Current call site in transcribe_parakeet.rs line 70: -->
```rust
// BEFORE (passes None, defaults to TimestampMode::Tokens):
let result = parakeet
    .transcribe_samples(audio_vec, 16000, 1, None)
    .map_err(|e| format!("Parakeet transcription error: {}", e))?;
```
</interfaces>

<tasks>

<task type="auto">
  <name>Task 1: Switch transcribe_samples call to TimestampMode::Sentences</name>
  <files>src-tauri/src/transcribe_parakeet.rs</files>
  <action>
In src-tauri/src/transcribe_parakeet.rs:

1. Add `TimestampMode` to the existing import on line 8:
   Change `use parakeet_rs::{ExecutionConfig, ExecutionProvider, ParakeetTDT};`
   to `use parakeet_rs::{ExecutionConfig, ExecutionProvider, ParakeetTDT, TimestampMode};`

2. On line 70, change the 4th argument from `None` to `Some(TimestampMode::Sentences)`:
   Change `.transcribe_samples(audio_vec, 16000, 1, None)`
   to `.transcribe_samples(audio_vec, 16000, 1, Some(TimestampMode::Sentences))`

3. Update the doc comment on the function (line 50-58 block) to note that Sentences mode is used for word deduplication. Add a line like:
   `/// Uses TimestampMode::Sentences to enable word-level deduplication (strips repeated tokens).`

That is the entire change. Two lines modified, one doc line added.
  </action>
  <verify>
    <automated>cd src-tauri && cargo check 2>&1 | tail -5</automated>
  </verify>
  <done>transcribe_with_parakeet passes Some(TimestampMode::Sentences) to transcribe_samples; cargo check passes with no errors</done>
</task>

</tasks>

<verification>
- `cargo check` in src-tauri passes
- grep confirms `TimestampMode::Sentences` appears in transcribe_parakeet.rs
- grep confirms `None` no longer appears as the 4th arg to transcribe_samples
</verification>

<success_criteria>
Parakeet transcription uses Sentences timestamp mode, enabling the deduplication pipeline in parakeet-rs timestamps.rs. The project compiles cleanly.
</success_criteria>

<output>
After completion, create `.planning/quick/11-switch-parakeet-transcription-from-times/11-SUMMARY.md`
</output>
