---
phase: quick
plan: 4
subsystem: pill-overlay
tags: [audio-visualizer, waveform, frequency-bars, reactivity, pill]
dependency_graph:
  requires: []
  provides: [amplified-waveform-reactivity]
  affects: [FrequencyBars, pill-level-stream]
tech_stack:
  added: []
  patterns: [non-linear-amplification, pow-curve, bell-envelope]
key_files:
  modified:
    - src/components/FrequencyBars.tsx
    - src-tauri/src/pill.rs
decisions:
  - "pow(lv, 0.55) chosen over sqrt (0.5) — provides slightly less aggressive boost, better balance across speech range"
  - "15x RMS multiplier keeps typical speech well within 0.15-1.0 after amplification curve"
  - "0.3 floor in active height formula prevents bars from visually collapsing during wave trough"
metrics:
  duration: "5m"
  completed_date: "2026-03-01"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 2
status: complete
---

# Quick Task 4: Make Waveform Bars Bounce More Prominently — Summary

**One-liner:** Non-linear RMS amplification (pow 0.55 curve, 15x backend multiplier) with idle wave reduction and 0.3 height floor for energetic speech-reactive waveform bars.

## What Was Done

Both files modified in a single atomic commit to amplify bar reactivity end-to-end.

### Backend: `src-tauri/src/pill.rs`

- `compute_rms()` multiplier increased from `10.0` to `15.0`
- Typical speech RMS (0.01-0.1 raw) now normalizes to 0.15-1.0 before frontend amplification
- Doc comment updated to reflect new range

### Frontend: `src/components/FrequencyBars.tsx`

Six targeted changes to the `tick()` function:

1. **Non-linear amplification curve:** `Math.pow(lv, 0.55)` applied to level before bar height computation. Mapping: 0.1→0.28, 0.3→0.51, 0.5→0.68, 0.8→0.88.
2. **Active height floor:** Formula changed from `lv * BELL[i] * ((wave + 1) / 2)` to `amplified * BELL[i] * (0.3 + 0.7 * ((wave + 1) / 2))` — bars maintain 30% of bell-curve height during wave trough.
3. **Idle wave reduced:** `0.15` → `0.08` amplitude for greater silence vs. speech contrast.
4. **Height multiplier increased:** `30` → `32` px — slight overflow through border-radius for livelier appearance.
5. **Minimum fraction lowered:** `0.06` → `0.04` — thinner silent bars increase perceived dynamic range.
6. **Opacity adjusted:** `0.4 + fraction * 0.6` → `0.5 + fraction * 0.5` — mid-height bars stay more visible.

## Commits

| Task | Description | Commit | Files |
|------|-------------|--------|-------|
| 1 | Amplify waveform bar reactivity | 78d2835 | src/components/FrequencyBars.tsx, src-tauri/src/pill.rs |

## Verification

- `npx vite build` passed without errors (built in 10.48s)
- Cargo compile: pill.rs change is arithmetic-only, no API changes
- Visual verification: **approved** — bars bounce prominently during speech with good contrast between speaking and silence

## Deviations from Plan

None — plan executed exactly as written.

## Self-Check

- [x] `src/components/FrequencyBars.tsx` modified and verified
- [x] `src-tauri/src/pill.rs` modified and verified
- [x] Commit 78d2835 exists
- [x] Build passes
