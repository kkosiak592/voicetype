---
phase: quick-11
plan: 01
subsystem: transcription-backend
tags: [parakeet, timestamps, deduplication, rust]
dependency_graph:
  requires: [parakeet-rs TimestampMode enum]
  provides: [Sentences timestamp mode for Parakeet TDT transcription]
  affects: [transcribe_parakeet.rs]
tech_stack:
  added: []
  patterns: [TimestampMode::Sentences for TDT model punctuation-aware grouping]
key_files:
  created: []
  modified:
    - src-tauri/src/transcribe_parakeet.rs
decisions:
  - "TimestampMode::Sentences used (not Words) — TDT model predicts punctuation, enabling sentence boundaries per parakeet-rs docs"
  - "None -> Some(TimestampMode::Sentences) at transcribe_samples call site — activates group_by_sentences -> deduplicate_words pipeline in parakeet-rs"
metrics:
  duration: "~62s (dominated by cargo check)"
  completed_date: "2026-03-01"
  tasks_completed: 1
  files_changed: 1
---

# Quick Task 11: Switch Parakeet Transcription to TimestampMode::Sentences

**One-liner:** Switched Parakeet TDT transcription from default `TimestampMode::Tokens` (via `None`) to `Some(TimestampMode::Sentences)`, activating the `group_by_sentences -> deduplicate_words` deduplication pipeline in parakeet-rs.

## What Was Done

Single change to `src-tauri/src/transcribe_parakeet.rs`:

1. Added `TimestampMode` to the `parakeet_rs` import on line 8.
2. Changed `transcribe_samples(audio_vec, 16000, 1, None)` to `transcribe_samples(audio_vec, 16000, 1, Some(TimestampMode::Sentences))` on line 72.
3. Added doc comment noting that Sentences mode enables word-level deduplication.

## Why Sentences Mode

The parakeet-rs `timestamps.rs` documentation explicitly recommends `Sentences` mode for Parakeet TDT models because TDT predicts punctuation, enabling natural sentence boundary detection. The `group_by_sentences` function calls `group_by_words` internally, then `deduplicate_words` — stripping consecutive repeated tokens that are a known artifact of neural TDT inference.

`Tokens` mode (the previous default via `None`) skips all grouping and deduplication entirely.

## Commits

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Switch transcribe_samples to TimestampMode::Sentences | 2a30c48 | src-tauri/src/transcribe_parakeet.rs |

## Verification

- `cargo check` passed with no errors (1m 00s)
- `grep TimestampMode::Sentences` confirms presence at import, doc comment, and call site
- `None` no longer appears as the 4th argument to `transcribe_samples`

## Deviations from Plan

None — plan executed exactly as written.

## Self-Check: PASSED

- `src-tauri/src/transcribe_parakeet.rs` modified: confirmed
- Commit `2a30c48` exists: confirmed
- `TimestampMode::Sentences` present at call site: confirmed
- `cargo check` passes: confirmed
