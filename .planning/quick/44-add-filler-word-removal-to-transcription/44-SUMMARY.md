---
phase: quick-44
plan: 01
subsystem: transcription-pipeline
tags: [filler-removal, post-processing, settings, ui]
dependency_graph:
  requires: []
  provides: [filler-removal-engine, filler-removal-toggle]
  affects: [pipeline, profiles, settings]
tech_stack:
  added: []
  patterns: [OnceLock-lazy-regex, AllCapsToggle-clone-pattern]
key_files:
  created:
    - src-tauri/src/filler.rs
    - src-tauri/src/filler_tests.rs
    - src/components/FillerRemovalToggle.tsx
  modified:
    - src-tauri/src/profiles.rs
    - src-tauri/src/pipeline.rs
    - src-tauri/src/lib.rs
    - src/components/sections/GeneralSection.tsx
decisions:
  - "Multi-word filler (uh huh) matched before single-word (uh) to prevent partial-strip of the compound"
  - "OnceLock<FillerPatterns> for one-time regex compilation — no repeated regex construction per call"
  - "Filler removal step inserted at 4b in pipeline — between trim (4) and corrections (5) per CONTEXT.md decision"
  - "store.get filler_removal on mount (not IPC get_filler_removal) — mirrors AllCapsToggle to avoid manage() timing issues"
metrics:
  duration: "~15 minutes"
  completed: "2026-03-05"
  tasks: 2
  files: 7
---

# Quick Task 44: Add Filler Word Removal to Transcription

**One-liner:** Regex-based hesitation-sound removal (um/uh/hmm/er/ah/uh-huh) with OnceLock-compiled word-boundary patterns, inserted before corrections in the pipeline, toggleable from General > Output settings.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Create filler removal module and wire into backend | c60987c | filler.rs, filler_tests.rs, profiles.rs, pipeline.rs, lib.rs |
| 2 | Add filler removal toggle to settings UI | 5b30ee8 | FillerRemovalToggle.tsx, GeneralSection.tsx |

## What Was Built

**filler.rs** — Public `remove_fillers(text: &str) -> String` function using `regex::Regex` with `(?i)\b{filler}\b` word-boundary patterns. Multi-word fillers (uh huh) processed first. After all replacements, `split_whitespace().collect::<Vec<_>>().join(" ")` normalises collapsed spaces. Patterns compiled once via `OnceLock<FillerPatterns>`.

**filler_tests.rs** — 17 unit tests covering: each filler word removed, case insensitivity, mid-sentence removal, multiple fillers, space collapsing, leading/trailing trim, false-positive preservation (umbrella, hummingbird, errand), empty-result and no-fillers cases.

**Profile struct** — Added `pub filler_removal: bool` field (default `false`). No migration needed — absent key defaults to false.

**Pipeline step 4b** — After trim, before corrections: reads `profile.filler_removal`, calls `crate::filler::remove_fillers(trimmed)` if enabled, passes `&defillered` to corrections instead of `trimmed`.

**IPC commands** — `get_filler_removal` and `set_filler_removal` mirror `get_all_caps`/`set_all_caps`. `set_filler_removal` persists `filler_removal` boolean to settings.json. Startup setup loads `filler_removal` key into active profile.

**FillerRemovalToggle.tsx** — Exact clone of AllCapsToggle pattern. Reads `store.get<boolean>('filler_removal')` on mount, calls `invoke('set_filler_removal', { enabled: next })` on toggle. Emerald switch with `aria-checked` and sr-only label.

**GeneralSection.tsx** — FillerRemovalToggle added below ALL CAPS row in Card 2 (Output section), separated by a border-t divider.

## Verification Results

- `cargo test filler --features moonshine`: 17/17 passed
- `cargo build --lib --features moonshine`: compiles cleanly (2 pre-existing dead_code warnings, not new)
- `npx tsc --noEmit`: no errors

## Deviations from Plan

None — plan executed exactly as written.

## Self-Check: PASSED

- src-tauri/src/filler.rs: FOUND
- src-tauri/src/filler_tests.rs: FOUND
- src/components/FillerRemovalToggle.tsx: FOUND
- Commit c60987c: FOUND
- Commit 5b30ee8: FOUND
