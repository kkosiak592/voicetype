---
phase: quick
plan: 7
subsystem: ui
tags: [pill, css, animation, recording-state]
dependency_graph:
  requires: []
  provides: [clean-recording-pill-appearance]
  affects: [src/Pill.tsx, src/pill.css]
tech_stack:
  added: []
  patterns: []
key_files:
  modified:
    - src/Pill.tsx
    - src/pill.css
decisions:
  - Removed rainbow border entirely — waveform bars are sufficient recording feedback
metrics:
  duration: "<5 minutes"
  completed: "2026-03-01"
  tasks_completed: 1
  files_modified: 2
---

# Quick Task 7: Remove Rainbow Border from Pill Recording State Summary

**One-liner:** Deleted rainbow conic-gradient border and its CSS infrastructure from the recording state; pill now shows dark glass + frequency bars only.

## What Was Done

Removed the `pill-rainbow-border` class and all associated CSS from the recording state of the pill component. The pill during recording now renders only the dark glass background (`pill-glass`) with frequency bars inside — no spinning rainbow ring.

## Task Commits

| Task | Description | Commit | Files |
|------|-------------|--------|-------|
| 1 | Remove rainbow border from pill recording state | 8d36805 | src/Pill.tsx, src/pill.css |

## Changes Made

**src/Pill.tsx (line 148):**
- Deleted: `${displayState === "recording" ? "pill-rainbow-border" : ""}`

**src/pill.css:**
- Updated comment: "leaves room for rainbow border" -> "Center pill within the window"
- Removed entire rainbow border section (34 lines):
  - `@property --border-angle` declaration
  - `@keyframes rainbow-rotate` keyframe
  - `.pill-rainbow-border` rule
  - `.pill-rainbow-border::after` rule
  - Section comment and orphaned dot-animation comment

## What Was Preserved

- `.pill-processing::after` — cyan/violet/pink gradient border during processing state (untouched)
- `FrequencyBars` bar colors — per-bar rainbow hue in bar elements (separate, untouched)
- `pill-entering` / `pill-exiting` entrance/exit animations (untouched)
- `pill-content-fade-in` content fade class (untouched)

## Verification

- `grep -r "rainbow" src/` returns only: `FrequencyBars.tsx` comment about bar hue (expected)
- `grep "pill-rainbow" src/` returns no results
- `npx tsc --noEmit` passes with no errors

## Deviations from Plan

None — plan executed exactly as written.
