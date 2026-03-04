---
phase: quick-38
plan: "01"
subsystem: vocabulary-removal
tags: [dead-code, cleanup, whisper, frontend, rust]
dependency_graph:
  requires: []
  provides: [QUICK-38]
  affects: [src/components/Sidebar.tsx, src/App.tsx, src-tauri/src/profiles.rs, src-tauri/src/lib.rs, src-tauri/src/pipeline.rs, src-tauri/src/transcribe.rs]
tech_stack:
  added: []
  patterns: [dead-code-removal]
key_files:
  created: []
  modified:
    - src/components/Sidebar.tsx
    - src/App.tsx
    - src-tauri/src/profiles.rs
    - src-tauri/src/lib.rs
    - src-tauri/src/pipeline.rs
    - src-tauri/src/transcribe.rs
    - src-tauri/src/corrections_tests.rs
  deleted:
    - src/components/sections/VocabularySection.tsx
decisions:
  - "transcribe_audio always sets no_context=true unconditionally — no vocabulary prompt support"
  - "corrections_tests.rs stripped of multi-profile test stubs that referenced non-existent functions"
metrics:
  duration_minutes: 15
  tasks_completed: 2
  files_changed: 8
  completed_date: "2026-03-04"
---

# Quick Task 38: Remove Entire Vocabulary Section Summary

**One-liner:** Deleted VocabularySection.tsx and removed all vocabulary_prompt/initial_prompt plumbing — Whisper now always runs with no_context=true and no initial_prompt parameter.

## Tasks Completed

| # | Name | Commit | Files |
|---|------|--------|-------|
| 1 | Remove vocabulary from frontend | 25bc5fd | VocabularySection.tsx (deleted), Sidebar.tsx, App.tsx |
| 2 | Remove initial_prompt/vocabulary_prompt from Rust backend | f847750 | profiles.rs, lib.rs, pipeline.rs, transcribe.rs |
| — | Fix dead test assertions | 6c3616b | corrections_tests.rs |

## What Was Done

**Task 1 — Frontend:**
- Deleted `src/components/sections/VocabularySection.tsx` entirely
- Removed `'vocabulary'` from the `SectionId` union type in `Sidebar.tsx`
- Removed the vocabulary `ITEMS` entry (`{ id: 'vocabulary', label: 'Vocabulary', icon: '◈' }`) from the sidebar
- Removed the `VocabularySection` import and JSX render line (`{activeSection === 'vocabulary' && <VocabularySection />}`) from `App.tsx`

**Task 2 — Rust backend:**
- Removed `initial_prompt: String` field and its doc comment from `Profile` struct in `profiles.rs`
- Removed `initial_prompt: String::new()` from `default_profile()`
- Deleted `get_vocabulary_prompt` and `set_vocabulary_prompt` Tauri command functions from `lib.rs`
- Removed both entries from the `invoke_handler` list in `lib.rs`
- Removed the "Load vocabulary_prompt from settings" block from setup in `lib.rs`
- Removed `prompt_len={}` format argument from the vocabulary profile loaded log line in `lib.rs`
- Removed `#[cfg(feature = "whisper")] let initial_prompt: String = { ... }` block from `pipeline.rs`
- Updated `transcribe_audio(&ctx, &samples, &initial_prompt)` to `transcribe_audio(&ctx, &samples)` in `pipeline.rs`
- Updated `transcribe_audio` signature from `(ctx, audio, initial_prompt: &str)` to `(ctx, audio)` in `transcribe.rs`
- Replaced conditional `if !initial_prompt.is_empty()` block with unconditional `params.set_no_context(true)` in `transcribe.rs`

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Two additional transcribe_audio call sites in lib.rs used old signature**
- **Found during:** Task 2 — cargo check
- **Issue:** `save_test_wav` and a CPU inference path in `lib.rs` called `transcribe::transcribe_audio(&ctx, &audio_f32, "")` with the removed third argument
- **Fix:** Updated both calls to `transcribe::transcribe_audio(&ctx, &audio_f32)` (lines ~1414 and ~1455)
- **Files modified:** `src-tauri/src/lib.rs`
- **Commit:** f847750

**2. [Rule 1 - Bug] corrections_tests.rs referenced removed initial_prompt field and non-existent profile functions**
- **Found during:** Post-task verification grep
- **Issue:** `corrections_tests.rs` had tests calling `structural_engineering_profile()`, `general_profile()`, and asserting `p.initial_prompt.contains(...)` — functions and field that no longer exist
- **Fix:** Removed the two broken test functions (`structural_engineering_profile_fields` and `general_profile_fields`); retained all working `CorrectionsEngine` tests
- **Files modified:** `src-tauri/src/corrections_tests.rs`
- **Commit:** 6c3616b

## Verification

- `npx tsc --noEmit` — passes with no errors
- `cargo check` — passes with no errors (only a pre-existing dead_code warning in benchmark binary unrelated to this change)
- `grep -r "vocabulary_prompt|initial_prompt" src/ src-tauri/src/` — no matches in `.rs`, `.tsx`, or `.ts` files
- `ls src/components/sections/VocabularySection.tsx` — no such file

## Self-Check: PASSED

- VocabularySection.tsx: DELETED (confirmed)
- Sidebar.tsx: 'vocabulary' removed from SectionId and ITEMS array
- App.tsx: VocabularySection import and JSX render removed
- profiles.rs: no initial_prompt field
- lib.rs: no get_vocabulary_prompt / set_vocabulary_prompt / vocabulary_prompt references
- pipeline.rs: transcribe_audio called with (ctx, samples) only
- transcribe.rs: signature is (ctx, audio) only; always sets no_context(true)
- Commits 25bc5fd, f847750, 6c3616b: all present
