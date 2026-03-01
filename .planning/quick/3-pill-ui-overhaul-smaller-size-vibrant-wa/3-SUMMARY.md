---
phase: quick-3
plan: 1
subsystem: pill-ui
tags: [pill, ui, animation, css, tauri]
dependency_graph:
  requires: []
  provides: [vibrant-pill-ui]
  affects: [pill-overlay]
tech_stack:
  added: []
  patterns: [conic-gradient, CSS @property, HSL rainbow mapping, bounce animation]
key_files:
  created: []
  modified:
    - src-tauri/tauri.conf.json
    - src/Pill.tsx
    - src/pill.css
    - src/components/FrequencyBars.tsx
    - src/components/ProcessingDots.tsx
decisions:
  - "Per-bar HSL hue mapping (i/BAR_COUNT)*300 gives full rainbow spectrum across 24 bars without manual color array"
  - "CSS @property --border-angle enables animatable conic-gradient border via WebView2-supported Houdini @property"
  - "::after pseudo-element with inset: -2px and z-index: -1 creates border effect without wrapping element"
  - "pill-rainbow-border sets position: relative so ::after z-index stacking works correctly"
  - "Legacy dot-pulse keyframe kept (harmless) alongside new dot-bounce for cleaner forward-compat"
metrics:
  duration: ~8 minutes
  completed_date: "2026-02-28"
  tasks_completed: 2
  tasks_total: 3
  files_changed: 5
---

# Quick Task 3: Pill UI Overhaul — Smaller Size, Vibrant Waveform, Rainbow Border

**One-liner:** Compact 200x44 pill with animated conic-gradient rainbow border on recording, HSL rainbow frequency bars, and bouncy purple-gradient processing dots.

## What Was Built

A full visual overhaul of the pill overlay:

1. **Smaller pill** — 280x56 -> 200x44px in both `tauri.conf.json` and `Pill.tsx`. Position calculation updated for new dimensions. Recording content padding tightened (px-4 -> px-3).

2. **Rainbow border during recording** — `pill-rainbow-border` CSS class applied to pill when `displayState === "recording"`. Uses CSS `@property --border-angle` (supported in WebView2) to animate a `conic-gradient` via `@keyframes rainbow-rotate`. A `::after` pseudo-element with `inset: -2px` and `z-index: -1` creates the 2px ring effect, with the dark `pill-glass` background covering the center.

3. **Vibrant rainbow frequency bars** — Each of the 24 bars receives a per-bar HSL hue: `hsl((i/24)*300, 90%, 65%)`, producing red on bar 0 through orange/yellow/green/blue to magenta on bar 23. Bar width reduced 4px -> 3px, gap 2px -> 1.5px, container height 36px -> 28px to fit the smaller pill.

4. **Bouncy purple processing dots** — Replaced opacity `dot-pulse` with vertical `dot-bounce` (translateY(-6px) + scale(1.15) at 50%). Dots changed to purple gradient (`#a78bfa -> #818cf8`), size 6px -> 5px, stagger 200ms -> 150ms for snappier feel.

## Commits

| Hash | Description |
|------|-------------|
| dbb2afc | feat(quick-3): shrink pill to 200x44 and add rainbow border CSS |
| 0980711 | feat(quick-3): vibrant rainbow bars, bouncy purple processing dots |

## Deviations from Plan

None — plan executed exactly as written.

## Verification Status

- Vite build: PASSED (both tasks)
- Human visual verification: PENDING (Task 3 checkpoint)
