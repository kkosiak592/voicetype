---
phase: quick-38
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/components/sections/VocabularySection.tsx  # DELETE
  - src/components/Sidebar.tsx
  - src/App.tsx
  - src-tauri/src/profiles.rs
  - src-tauri/src/lib.rs
  - src-tauri/src/pipeline.rs
  - src-tauri/src/transcribe.rs
autonomous: true
requirements: [QUICK-38]
must_haves:
  truths:
    - "Vocabulary section no longer appears in settings sidebar"
    - "No vocabulary_prompt or initial_prompt field exists in backend structs or IPC commands"
    - "Whisper transcribe_audio no longer accepts an initial_prompt parameter"
    - "Application compiles and runs without errors"
  artifacts:
    - path: "src/components/sections/VocabularySection.tsx"
      provides: "DELETED — must not exist"
  key_links:
    - from: "src-tauri/src/pipeline.rs"
      to: "src-tauri/src/transcribe.rs"
      via: "transcribe_audio call"
      pattern: "transcribe_audio\\(&ctx, &samples\\)"
---

<objective>
Remove the entire Vocabulary section from the settings UI and all vocabulary_prompt/initial_prompt plumbing from the Rust backend. After this change, Whisper always runs with no_context=true and no initial_prompt parameter.

Purpose: Vocabulary prompting is unused since Moonshine (which doesn't support it) and the corrections dictionary handles domain terminology. Dead code removal.
Output: Cleaner codebase with no vocabulary prompt references.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src/components/sections/VocabularySection.tsx
@src/components/Sidebar.tsx
@src/App.tsx
@src-tauri/src/profiles.rs
@src-tauri/src/lib.rs
@src-tauri/src/pipeline.rs
@src-tauri/src/transcribe.rs
</context>

<tasks>

<task type="auto">
  <name>Task 1: Remove vocabulary from frontend</name>
  <files>src/components/sections/VocabularySection.tsx, src/components/Sidebar.tsx, src/App.tsx</files>
  <action>
1. DELETE src/components/sections/VocabularySection.tsx entirely.

2. In src/components/Sidebar.tsx:
   - Remove 'vocabulary' from the SectionId union type (line 1). Result: `'general' | 'model' | 'microphone' | 'appearance' | 'history'`
   - Remove the `{ id: 'vocabulary', label: 'Vocabulary', icon: '...' }` entry from ITEMS array (line 11).

3. In src/App.tsx:
   - Remove the `import { VocabularySection }` line (line 7).
   - Remove the `{activeSection === 'vocabulary' && <VocabularySection />}` JSX line (line 195).
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && npx tsc --noEmit 2>&1 | head -20</automated>
  </verify>
  <done>VocabularySection.tsx deleted, no vocabulary entry in sidebar, no vocabulary rendering in App.tsx, TypeScript compiles clean.</done>
</task>

<task type="auto">
  <name>Task 2: Remove initial_prompt/vocabulary_prompt from Rust backend</name>
  <files>src-tauri/src/profiles.rs, src-tauri/src/lib.rs, src-tauri/src/pipeline.rs, src-tauri/src/transcribe.rs</files>
  <action>
1. In src-tauri/src/profiles.rs:
   - Remove `initial_prompt: String` field from the Profile struct.
   - Remove the doc comment line about initial_prompt.
   - Remove `initial_prompt: String::new()` from default_profile().

2. In src-tauri/src/lib.rs:
   - Delete the entire `get_vocabulary_prompt` command function (~lines 892-898).
   - Delete the entire `set_vocabulary_prompt` command function (~lines 900-914).
   - Remove `get_vocabulary_prompt` and `set_vocabulary_prompt` from the invoke_handler list (~lines 1607-1608).
   - Remove the "Load vocabulary_prompt from settings" block (~lines 1867-1870) that reads vocabulary_prompt from JSON into active_profile.initial_prompt.
   - In the log::info line (~1893-1897), remove the `prompt_len={}` format arg and the `active_profile.initial_prompt.len()` argument.

3. In src-tauri/src/pipeline.rs:
   - Remove the entire `#[cfg(feature = "whisper")] let initial_prompt: String = { ... }` block (~lines 160-167) that reads initial_prompt from ActiveProfile.
   - Update the transcribe_audio call (~line 194) from `transcribe_audio(&ctx, &samples, &initial_prompt)` to `transcribe_audio(&ctx, &samples)` (remove the third argument).

4. In src-tauri/src/transcribe.rs:
   - Remove the `initial_prompt` parameter from the `transcribe_audio` function signature (~line 271). New signature: `pub fn transcribe_audio(ctx: &WhisperContext, audio: &[f32]) -> Result<String, String>`
   - Remove the doc comment lines about initial_prompt (~lines 265-268).
   - Remove the entire `if !initial_prompt.is_empty() { ... } else { ... }` block (~lines 289-293). Replace with just: `params.set_no_context(true);` (always, unconditionally).
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && cargo check 2>&1 | tail -5</automated>
  </verify>
  <done>No initial_prompt field in Profile, no vocabulary IPC commands, pipeline no longer reads initial_prompt, transcribe_audio takes only (ctx, audio), always sets no_context(true). Cargo check passes.</done>
</task>

</tasks>

<verification>
- `cargo check` passes with no errors
- `npx tsc --noEmit` passes with no errors
- `grep -r "vocabulary_prompt\|initial_prompt" src/ src-tauri/src/ --include="*.rs" --include="*.tsx" --include="*.ts"` returns no matches
- `ls src/components/sections/VocabularySection.tsx` returns "No such file"
</verification>

<success_criteria>
- VocabularySection.tsx deleted
- No "Vocabulary" entry in settings sidebar
- Profile struct has no initial_prompt field
- transcribe_audio signature is (ctx, audio) only
- Both TypeScript and Rust compile cleanly
</success_criteria>

<output>
After completion, create `.planning/quick/38-remove-entire-vocabulary-section-from-se/38-SUMMARY.md`
</output>
