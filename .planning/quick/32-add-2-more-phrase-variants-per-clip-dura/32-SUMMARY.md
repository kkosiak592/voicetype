---
phase: quick-32
plan: 01
subsystem: benchmark
tags: [benchmark, wav-generation, wer, markdown-report]
dependency_graph:
  requires: []
  provides: [9-clip-benchmark, markdown-report-output]
  affects: [test-fixtures/generate-benchmark-wavs.ps1, src-tauri/src/bin/benchmark.rs]
tech_stack:
  added: []
  patterns: [write_markdown_report, per-clip-variant-refs]
key_files:
  created: []
  modified:
    - test-fixtures/generate-benchmark-wavs.ps1
    - src-tauri/src/bin/benchmark.rs
decisions:
  - Duplicated scoring logic in write_markdown_report rather than extracting shared helper -- acceptable for benchmark tooling
  - Used Unix epoch timestamp for report (no chrono dependency added)
  - Clip labels use suffix convention: 5s, 5s-b, 5s-c (original has no suffix)
metrics:
  duration_seconds: 171
  completed: "2026-03-03T20:04:25Z"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 2
---

# Quick Task 32: Add 2 More Phrase Variants Per Clip Duration + Markdown Report

Expanded benchmark from 3 to 9 WAV clips (3 durations x 3 real-world content variants) and added markdown report writer to benchmark binary.

## Tasks Completed

### Task 1: Add 6 new phrase variants to PowerShell WAV generator
**Commit:** a0c7041
**Files:** test-fixtures/generate-benchmark-wavs.ps1

Added 6 new TTS generation sections using diverse real-world content (not self-referential benchmark text):
- 5s-b: copper wire / circuit board
- 5s-c: satellite orbit / photography
- 30s-b: steel manufacturing process
- 30s-c: Mediterranean cooking and food science
- 60s-b: Panama Canal engineering and operations
- 60s-c: human immune system biology

### Task 2: Add new clip references and markdown report to benchmark binary
**Commit:** 220b0bf
**Files:** src-tauri/src/bin/benchmark.rs

- Added 6 new `REF_*` constants with text matching PowerShell phrases exactly
- Updated `reference_for_clip()` from 3 arms to 9
- Expanded `clips` vec from 3 to 9 entries
- Added `write_markdown_report()` function producing `benchmark-results.md` with:
  - Results table (model, clip, avg/min/max ms, WER%)
  - Model rankings (scored by geometric mean of speed and accuracy)
  - Reference transcriptions (all 9 clips with blockquoted text)
  - Transcription samples (first-run output per model/clip)
- Added `use std::io::Write` import
- Updated doc comment to reflect 9 WAV fixture expectation

## Deviations from Plan

None -- plan executed exactly as written.

## Verification

- `cargo check --bin benchmark --features whisper,parakeet` compiles cleanly
- PowerShell script contains 18 WAV filename references (9 files x 2 occurrences each)
- All 9 clip labels have corresponding REF_* constants
- `reference_for_clip` covers all 9 labels

## Self-Check: PASSED

- test-fixtures/generate-benchmark-wavs.ps1: FOUND
- src-tauri/src/bin/benchmark.rs: FOUND
- Commit a0c7041: FOUND
- Commit 220b0bf: FOUND
