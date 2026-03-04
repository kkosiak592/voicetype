---
phase: quick-36
plan: 01
subsystem: benchmark
tags: [benchmark, wav-fixtures, pivot-tables, 90s-clips]
dependency_graph:
  requires: []
  provides: [90s-benchmark-clips, 12-wav-fixture-support]
  affects: [benchmark-binary, generate-benchmark-wavs.ps1]
tech_stack:
  added: []
  patterns: [backslash-continuation-rust-string-literals, powershell-here-strings]
key_files:
  created: []
  modified:
    - test-fixtures/generate-benchmark-wavs.ps1
    - src-tauri/src/bin/benchmark.rs
decisions:
  - "22-sentence passages chosen for ~90s TTS duration; topics: deep-sea oceanography, aviation history, renewable energy"
  - "Rust constants use backslash continuation (same pattern as REF_60S_B/C) to match PS1 here-string text exactly"
  - "Separator widths changed from 68 to 82 chars (+14 = ' | ' + 10 col width + 1 pad) for the new 90s column"
metrics:
  duration: "5 minutes"
  completed: "2026-03-04"
  tasks: 2
  files: 2
---

# Phase quick-36 Plan 01: Add 90s Benchmark Clips Summary

Extended the benchmark system from 9 WAV files (3 durations) to 12 WAV files (4 durations) by adding 90-second clips covering deep-sea oceanography, aviation history, and renewable energy — with matching Rust reference constants and 4-column pivot tables in both console and markdown output.

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Add 90s TTS phrases to generate-benchmark-wavs.ps1 | d39d6d8 | test-fixtures/generate-benchmark-wavs.ps1 |
| 2 | Add 90s reference transcriptions, clip entries, and pivot table columns to benchmark.rs | e8bea88 | src-tauri/src/bin/benchmark.rs |

## What Was Built

**Task 1 — PowerShell script (`generate-benchmark-wavs.ps1`):**
- Added `benchmark-90s.wav` section: 22-sentence passage on deep-sea oceanography (hydrothermal vents, pressure, marine biology, Mariana Trench, submersibles)
- Added `benchmark-90s-b.wav` section: 22-sentence passage on aviation history (Wright brothers through space planes)
- Added `benchmark-90s-c.wav` section: 22-sentence passage on renewable energy (solar, wind, storage, grid, geothermal, tidal)
- Updated final `Write-Host` line from "9 WAV files" to "12 WAV files"

**Task 2 — Benchmark binary (`benchmark.rs`):**
- Added `REF_90S`, `REF_90S_B`, `REF_90S_C` string constants after `REF_60S_C` (line 344), using backslash continuation pattern; text exactly matches the PS1 passages
- Added 3 match arms in `reference_for_clip()` for "90s", "90s-b", "90s-c"
- Added 3 clip entries to `clips` vector: `("benchmark-90s.wav", "90s")`, `("benchmark-90s-b.wav", "90s-b")`, `("benchmark-90s-c.wav", "90s-c")`
- Updated `duration_groups` in both `print_summary` and `write_markdown_report` from `["5s", "30s", "60s"]` to `["5s", "30s", "60s", "90s"]`
- Updated `print_summary` latency/WER headers and rows to 5-column format (`{:<30} | {:>10} | {:>10} | {:>10} | {:>10}`)
- Changed separator `.repeat(68)` to `.repeat(82)` in all 4 locations (latency dashes, latency equals, WER dashes, WER equals)
- Updated markdown pivot headers to `"| Model | 5s | 30s | 60s | 90s |"` and rows to write 4 data columns
- Updated `clip_labels` array from 9 to 12 entries (added "90s", "90s-b", "90s-c")
- Updated top-level doc comment from 9 to 12 WAV fixtures

## Deviations from Plan

None - plan executed exactly as written.

## Verification

- `cargo check --bin benchmark --features bench_extra`: finished with 0 errors (1 pre-existing dead_code warning in audio.rs, out of scope)
- `REF_90S`, `REF_90S_B`, `REF_90S_C` constants confirmed at lines 344, 367, 390
- "90s" appears in clips vector (line 701), reference_for_clip match (line 487), duration_groups (lines 1967, 2140), pivot headers (lines 1991, 2009), and clip_labels (line 2178)
- `benchmark-90s.wav`, `benchmark-90s-b.wav`, `benchmark-90s-c.wav` confirmed in generate-benchmark-wavs.ps1
- Final PS1 line reads "12 WAV files"

## Self-Check: PASSED

- C:\Users\kkosiak.TITANPC\Desktop\Code\voice-to-text\test-fixtures\generate-benchmark-wavs.ps1 — FOUND, modified
- C:\Users\kkosiak.TITANPC\Desktop\Code\voice-to-text\src-tauri\src\bin\benchmark.rs — FOUND, modified
- Commit d39d6d8 — FOUND (feat(quick-36): add 90s TTS phrases)
- Commit e8bea88 — FOUND (feat(quick-36): add 90s clips and 4-column pivot tables)
