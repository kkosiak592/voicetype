---
phase: quick-44
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/src/filler.rs
  - src-tauri/src/profiles.rs
  - src-tauri/src/pipeline.rs
  - src-tauri/src/lib.rs
  - src/components/FillerRemovalToggle.tsx
  - src/components/sections/GeneralSection.tsx
autonomous: true
requirements: [QUICK-44]

must_haves:
  truths:
    - "Filler words (um, uh, uh huh, hmm, er, ah) are stripped from transcription output when enabled"
    - "Filler removal toggle appears in General > Output section of settings"
    - "Filler removal is off by default and persists across restarts"
    - "Filler removal runs before corrections in the pipeline"
  artifacts:
    - path: "src-tauri/src/filler.rs"
      provides: "Filler word removal engine"
      contains: "pub fn remove_fillers"
    - path: "src/components/FillerRemovalToggle.tsx"
      provides: "Toggle UI component"
      exports: ["FillerRemovalToggle"]
  key_links:
    - from: "src-tauri/src/pipeline.rs"
      to: "src-tauri/src/filler.rs"
      via: "remove_fillers call before corrections"
      pattern: "filler::remove_fillers"
    - from: "src/components/FillerRemovalToggle.tsx"
      to: "src-tauri/src/lib.rs"
      via: "invoke set_filler_removal / store.get filler_removal"
      pattern: "set_filler_removal"
---

<objective>
Add toggleable filler word removal to the transcription pipeline. Strips hesitation sounds (um, uh, uh huh, hmm, er, ah) from transcribed text before corrections are applied.

Purpose: Clean up transcription output by removing common hesitation sounds that clutter dictated text.
Output: New filler.rs module, updated pipeline, settings toggle in UI.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@.planning/quick/44-add-filler-word-removal-to-transcription/44-CONTEXT.md

<interfaces>
<!-- Existing patterns to follow exactly -->

From src-tauri/src/profiles.rs:
```rust
#[derive(Clone, Serialize, Deserialize)]
pub struct Profile {
    pub corrections: HashMap<String, String>,
    pub all_caps: bool,
}
pub struct ActiveProfile(pub std::sync::Mutex<Profile>);
```

From src-tauri/src/pipeline.rs (step 5, lines 329-346):
```rust
// 5. Apply corrections
let corrected = {
    let engine = app.state::<crate::corrections::CorrectionsState>();
    let guard = engine.0.lock().unwrap_or_else(|e| e.into_inner());
    guard.apply(trimmed)
};
// Apply ALL CAPS
let formatted = {
    let profile = app.state::<crate::profiles::ActiveProfile>();
    let guard = profile.0.lock().map_err(|e| format!("state lock: {}", e))?;
    if guard.all_caps { corrected.to_uppercase() } else { corrected }
};
```

From src-tauri/src/lib.rs IPC pattern (set_all_caps):
```rust
#[tauri::command]
fn set_all_caps(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    { let state = app.state::<profiles::ActiveProfile>();
      let mut guard = state.0.lock().map_err(|e| format!("state lock failed: {}", e))?;
      guard.all_caps = enabled; }
    let mut json = read_settings(&app)?;
    json["all_caps"] = serde_json::Value::Bool(enabled);
    write_settings(&app, &json)?;
    Ok(())
}
```

From src/components/AllCapsToggle.tsx (UI pattern):
```typescript
// Reads from store.get<boolean>('all_caps') on mount
// Toggles via invoke('set_all_caps', { enabled: next })
```

From src/components/sections/GeneralSection.tsx (Card 2: Output, lines 77-93):
```tsx
{/* Card 2: Output */}
<div className="bg-white dark:bg-gray-900 ring-1 ...">
  <section>
    <h2>Output</h2>
    <div className="flex items-center justify-between">
      <div>
        <p>ALL CAPS</p>
        <p>Convert all transcribed text to uppercase</p>
      </div>
      <AllCapsToggle />
    </div>
  </section>
</div>
```
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Create filler removal module and wire into backend</name>
  <files>src-tauri/src/filler.rs, src-tauri/src/filler_tests.rs, src-tauri/src/profiles.rs, src-tauri/src/pipeline.rs, src-tauri/src/lib.rs</files>
  <behavior>
    - "um" standalone is removed: "um I think so" -> "I think so"
    - "uh" standalone is removed: "uh what was that" -> "what was that"
    - "uh huh" multi-word is removed: "uh huh that works" -> "that works"
    - "hmm" standalone is removed: "hmm let me think" -> "let me think"
    - "er" standalone is removed: "er the thing is" -> "the thing is"
    - "ah" standalone is removed: "ah I see" -> "I see"
    - Case insensitive: "Um I think" -> "I think"
    - Mid-sentence: "I um think so" -> "I think so"
    - Multiple fillers: "um uh I think" -> "I think"
    - Collapsed double spaces after removal: "I um think" -> "I think" (not "I  think")
    - Leading/trailing whitespace trimmed after removal
    - Non-filler words preserved: "umbrella" unchanged, "hummingbird" unchanged, "errand" unchanged
    - Empty result after all fillers removed returns empty string
  </behavior>
  <action>
    1. Create `src-tauri/src/filler.rs`:
       - `pub fn remove_fillers(text: &str) -> String`
       - Hardcoded list: ["um", "uh", "uh huh", "hmm", "er", "ah"]
       - Use `regex::Regex` with word-boundary matching `(?i)\b{filler}\b` — same pattern as corrections engine
       - Process multi-word fillers first (uh huh) before single-word to avoid partial matches
       - After all replacements, collapse multiple spaces to single space with `.split_whitespace().collect::<Vec<_>>().join(" ")`
       - Use `lazy_static!` or `once_cell::sync::Lazy` (check which is already in Cargo.toml) to compile regexes once

    2. Create `src-tauri/src/filler_tests.rs` with #[cfg(test)] tests covering all behaviors above.

    3. Add `pub filler_removal: bool` to `Profile` in `profiles.rs`, default `false` in `default_profile()`.

    4. In `pipeline.rs`, insert filler removal step BETWEEN step 4 (trim) and step 5 (corrections):
       ```rust
       // 4b. Filler word removal (before corrections, per CONTEXT.md decision)
       let defillered = {
           let profile = app.state::<crate::profiles::ActiveProfile>();
           let guard = profile.0.lock().unwrap_or_else(|e| e.into_inner());
           if guard.filler_removal {
               crate::filler::remove_fillers(trimmed)
           } else {
               trimmed.to_string()
           }
       };
       ```
       Then pass `&defillered` to corrections instead of `trimmed`.

    5. In `lib.rs`:
       - Add `mod filler;` declaration (and `#[cfg(test)] mod filler_tests;` if using separate test file, or inline tests)
       - Add `get_filler_removal` command: reads `profile.filler_removal` (mirror `get_all_caps`)
       - Add `set_filler_removal` command: sets `profile.filler_removal` + persists `filler_removal` key to settings.json (mirror `set_all_caps`)
       - Register both commands in `invoke_handler`
       - In settings load section (near line 2013 where `all_caps` is loaded), add: `if let Some(flag) = json.get("filler_removal").and_then(|v| v.as_bool()) { active_profile.filler_removal = flag; }`
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && cargo test filler --features moonshine 2>&1 | tail -20</automated>
  </verify>
  <done>Filler removal module exists with passing tests. Pipeline calls remove_fillers before corrections when filler_removal=true. IPC commands get/set_filler_removal registered and persist to settings.json.</done>
</task>

<task type="auto">
  <name>Task 2: Add filler removal toggle to settings UI</name>
  <files>src/components/FillerRemovalToggle.tsx, src/components/sections/GeneralSection.tsx</files>
  <action>
    1. Create `src/components/FillerRemovalToggle.tsx` — exact clone of `AllCapsToggle.tsx` pattern:
       - `store.get<boolean>('filler_removal')` on mount
       - `invoke('set_filler_removal', { enabled: next })` on toggle
       - Same toggle button styling (emerald switch)
       - `aria-checked`, sr-only label "Toggle filler word removal"

    2. In `src/components/sections/GeneralSection.tsx`:
       - Import `FillerRemovalToggle`
       - Add a new row BELOW the ALL CAPS row inside Card 2 (Output section)
       - Add a divider `<div className="my-4 border-t border-gray-100 dark:border-gray-800" />` between ALL CAPS and filler removal
       - Row structure:
         ```tsx
         <div className="flex items-center justify-between">
           <div>
             <p className="text-sm font-medium text-gray-900 dark:text-gray-100">Remove Fillers</p>
             <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
               Strip hesitation sounds (um, uh, hmm) from transcribed text
             </p>
           </div>
           <FillerRemovalToggle />
         </div>
         ```
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && npx tsc --noEmit 2>&1 | tail -10</automated>
  </verify>
  <done>FillerRemovalToggle component renders in General > Output section below ALL CAPS. TypeScript compiles without errors. Toggle reads from store and invokes set_filler_removal IPC command.</done>
</task>

</tasks>

<verification>
1. `cargo test filler` — all filler removal unit tests pass
2. `cargo build --features moonshine` — backend compiles
3. `npx tsc --noEmit` — frontend compiles
4. Manual: enable toggle in settings, dictate with filler words, verify they are stripped from output
</verification>

<success_criteria>
- Filler words (um, uh, uh huh, hmm, er, ah) removed from transcription when toggle is on
- Toggle visible in General > Output settings, off by default
- Setting persists across app restarts via settings.json
- Filler removal runs before corrections in pipeline
- No false positives on words containing filler substrings (umbrella, hummingbird, errand)
</success_criteria>

<output>
After completion, create `.planning/quick/44-add-filler-word-removal-to-transcription/44-01-SUMMARY.md`
</output>
