---
phase: quick-45
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/src/correction_log.rs
  - src-tauri/src/lib.rs
  - src-tauri/src/history.rs
  - src-tauri/src/pipeline.rs
  - src/components/sections/HistorySection.tsx
autonomous: true
requirements: [QUICK-45]

must_haves:
  truths:
    - "History entries show the raw transcription and allow the user to submit a corrected version"
    - "Submitting a correction logs the from->to word pair with a count"
    - "When a correction reaches 3 occurrences it is auto-added to the corrections dictionary"
    - "User sees an inline notification when a correction is auto-promoted to the dictionary"
  artifacts:
    - path: "src-tauri/src/correction_log.rs"
      provides: "CorrectionLog struct — load/save/increment/promote logic"
      exports: ["CorrectionLog", "CorrectionLogState", "load_correction_log"]
    - path: "src-tauri/src/history.rs"
      provides: "HistoryEntry with raw_text field"
      contains: "raw_text"
    - path: "src/components/sections/HistorySection.tsx"
      provides: "Inline correction editor per history entry + auto-promote notification"
  key_links:
    - from: "src-tauri/src/pipeline.rs"
      to: "history::append_history"
      via: "passes raw (pre-correction) text alongside formatted text"
      pattern: "append_history.*raw"
    - from: "src/components/sections/HistorySection.tsx"
      to: "src-tauri/src/correction_log.rs"
      via: "invoke('submit_correction')"
      pattern: "invoke.*submit_correction"
    - from: "src-tauri/src/correction_log.rs"
      to: "src-tauri/src/corrections.rs"
      via: "promote() calls save_corrections to add to dictionary"
      pattern: "save_corrections|corrections\\.insert"
---

<objective>
Add a correction learning system: track user-submitted corrections against raw transcriptions, log from->to word pairs with occurrence counts, and auto-promote corrections to the dictionary after 3 repetitions.

Purpose: Dragon NaturallySpeaking-style adaptive learning — the dictionary improves automatically as the user corrects transcription mistakes.
Output: Backend correction log module, updated history with raw text, frontend correction editor in history panel.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@src-tauri/src/corrections.rs
@src-tauri/src/history.rs
@src-tauri/src/pipeline.rs
@src-tauri/src/lib.rs
@src/components/sections/HistorySection.tsx
@src/components/DictionaryEditor.tsx

<interfaces>
<!-- Key types and contracts the executor needs. -->

From src-tauri/src/corrections.rs:
```rust
pub struct CorrectionsEngine { rules: Vec<Rule> }
impl CorrectionsEngine {
    pub fn from_map(map: &HashMap<String, String>) -> Result<Self, String>;
    pub fn apply(&self, text: &str) -> String;
}
pub struct CorrectionsState(pub std::sync::Mutex<CorrectionsEngine>);
```

From src-tauri/src/history.rs:
```rust
pub struct HistoryEntry {
    pub text: String,        // final (post-correction) text
    pub timestamp_ms: u64,
    pub engine: String,
}
pub struct HistoryState(pub std::sync::Mutex<Vec<HistoryEntry>>);
pub fn append_history(app: &AppHandle, text: &str, engine: &str);
```

From src-tauri/src/profiles.rs:
```rust
pub struct Profile {
    pub corrections: HashMap<String, String>,
    pub all_caps: bool,
    pub filler_removal: bool,
}
pub struct ActiveProfile(pub std::sync::Mutex<Profile>);
```

From src-tauri/src/lib.rs:
```rust
fn save_corrections(app: AppHandle, corrections: HashMap<String, String>) -> Result<(), String>;
// Persists to settings.json under "corrections.default", rebuilds CorrectionsEngine
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Backend — correction log module + raw text in history + IPC commands</name>
  <files>
    src-tauri/src/correction_log.rs,
    src-tauri/src/history.rs,
    src-tauri/src/pipeline.rs,
    src-tauri/src/lib.rs
  </files>
  <action>
**1. Create `src-tauri/src/correction_log.rs`:**

Define a `CorrectionEntry` struct with fields: `from: String`, `to: String`, `count: u32`. Define `CorrectionLog` as a `Vec<CorrectionEntry>` wrapper with methods:
- `load(app: &AppHandle) -> Self` — reads `corrections_log.json` from app_data_dir. Returns empty if missing/corrupt.
- `save(&self, app: &AppHandle)` — writes JSON to `corrections_log.json`.
- `record(&mut self, from: String, to: String) -> Option<CorrectionEntry>` — find existing entry with same from/to (case-insensitive on `from`), increment count. If new, insert with count=1. Returns the entry if count reaches 3 (the promotion threshold). If count was already >= 3, returns None (already promoted).
- `remove(&mut self, from: &str, to: &str)` — remove an entry (for undo of auto-promote).

Define `CorrectionLogState(pub std::sync::Mutex<CorrectionLog>)` for Tauri managed state.

**2. Modify `src-tauri/src/history.rs`:**

Add `raw_text: Option<String>` field to `HistoryEntry` (Option for backward compat with existing history.json — old entries will deserialize with None). Update `append_history` signature to accept an optional `raw_text: Option<&str>` parameter and store it in the entry.

Use `#[serde(default)]` on the new field so existing history.json files don't break.

**3. Modify `src-tauri/src/pipeline.rs`:**

In the pipeline, capture the `defillered` text (post-filler-removal, pre-correction) and pass it to `append_history` as the `raw_text` argument. The `defillered` text is the raw transcription the user would compare against. Pass `Some(&defillered)` when calling `append_history`. If the raw text equals the final formatted text, pass `None` (no corrections were applied, no raw needed).

**4. Modify `src-tauri/src/lib.rs`:**

- Add `mod correction_log;` declaration.
- Register `CorrectionLogState` in the setup closure (load from disk at startup, `app.manage(...)`).
- Add IPC command `submit_correction(app: AppHandle, from: String, to: String) -> Result<Option<PromotedCorrection>, String>`:
  - Locks CorrectionLogState, calls `record(from, to)`.
  - If record() returns Some (hit threshold): auto-promote by inserting into ActiveProfile.corrections, calling the existing `save_corrections` logic (persist to settings.json + rebuild CorrectionsEngine). Return a `PromotedCorrection { from, to }` so frontend can show notification.
  - Save correction log to disk.
- Add IPC command `undo_promotion(app: AppHandle, from: String, to: String) -> Result<(), String>`:
  - Remove the from->to entry from ActiveProfile.corrections.
  - Remove from CorrectionLogState.
  - Re-persist corrections (save_corrections logic) and correction log.
- Register both commands in the `.invoke_handler(tauri::generate_handler![...])` list.

Define `PromotedCorrection` as a simple serializable struct with `from` and `to` fields.
  </action>
  <verify>
    <automated>cd /c/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && cargo check --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5</automated>
  </verify>
  <done>
    - correction_log.rs exists with CorrectionLog load/save/record/remove
    - HistoryEntry has raw_text: Option&lt;String&gt; with serde(default)
    - pipeline.rs passes raw (pre-correction) text to append_history
    - submit_correction and undo_promotion IPC commands registered
    - CorrectionLogState managed in setup()
    - cargo check passes
  </done>
</task>

<task type="auto">
  <name>Task 2: Frontend — inline correction editor in history + auto-promote notification</name>
  <files>
    src/components/sections/HistorySection.tsx
  </files>
  <action>
**Modify `src/components/sections/HistorySection.tsx`:**

1. Update the `HistoryEntry` TypeScript interface to add `rawText?: string`.

2. For each history entry that has a non-null `rawText` (meaning corrections were applied), render an "Edit" button (pencil icon from lucide-react) next to the copy button. Clicking it opens an inline editable textarea pre-filled with the `rawText` value (the raw transcription before corrections). The user edits this to what it SHOULD have been.

3. Add a "Submit Correction" button below the textarea. On submit:
   - Compute word-level diff between `rawText` and the user's edited text. Use a simple approach: split both by whitespace, find differing tokens at the same positions. For each diff pair where the raw word differs from the corrected word, that is a correction candidate (from=raw_word, to=corrected_word). Skip pairs where both are identical.
   - For each extracted correction pair, call `invoke('submit_correction', { from, to })`.
   - If any invocation returns a `PromotedCorrection` (non-null), show an inline green notification bar at the top of the history section: "Auto-added: {from} -> {to}" with an "Undo" button. The undo button calls `invoke('undo_promotion', { from, to })` and dismisses the notification.
   - Auto-dismiss the notification after 10 seconds if not interacted with.
   - Close the edit textarea after submission.

4. Styling: Keep consistent with existing history card styling. The edit textarea should use the same rounded border, dark mode classes. The notification bar should be a simple `bg-emerald-50 dark:bg-emerald-900/20 text-emerald-700 dark:text-emerald-300` strip with rounded corners.

5. If the history entry has NO `rawText` (no corrections were applied, or old entry), do NOT show the edit button — there is nothing to correct against.

**Diff algorithm details (Claude's discretion):**
Use positional word comparison — split raw and final text by whitespace, iterate both arrays simultaneously. When words at the same index differ, record `{from: rawWord, to: correctedWord}`. If arrays have different lengths (user added/removed words), skip unmatched tail. This is intentionally simple — covers the 90% case of word substitutions, which is what the corrections dictionary handles.
  </action>
  <verify>
    <automated>cd /c/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && npx tsc --noEmit 2>&1 | tail -5</automated>
  </verify>
  <done>
    - History entries with rawText show an edit button
    - Clicking edit opens inline textarea pre-filled with raw transcription
    - Submitting extracts word-level diffs and calls submit_correction for each
    - Auto-promoted corrections show a green notification with undo capability
    - Notification auto-dismisses after 10 seconds
    - TypeScript compiles without errors
  </done>
</task>

</tasks>

<verification>
1. `cargo check` passes in src-tauri
2. `npx tsc --noEmit` passes in frontend
3. Manual test: dictate text with a known corrections dictionary entry, verify history shows rawText, click edit, modify a word, submit, check corrections_log.json in app data dir shows the entry with count 1
4. Submit same correction 3 times total (across 3 separate dictations or by editing the same entry), verify the correction is auto-added to the dictionary and the green notification appears
</verification>

<success_criteria>
- Correction log persists to corrections_log.json in app data directory
- History entries include raw (pre-correction) text when corrections were applied
- User can edit a history entry's raw text and submit corrections
- Corrections reaching 3 occurrences are auto-promoted to the dictionary
- Auto-promoted corrections show an inline notification with undo
- Existing history.json backward compatible (rawText is optional)
</success_criteria>

<output>
After completion, create `.planning/quick/45-learn-from-user-corrections-to-auto-impr/45-SUMMARY.md`
</output>
