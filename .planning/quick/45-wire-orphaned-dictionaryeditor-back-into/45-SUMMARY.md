---
phase: quick-45
plan: 01
subsystem: frontend
tags: [corrections, dictionary, settings, ipc]
dependency_graph:
  requires: [DictionaryEditor component, get_corrections IPC, save_corrections IPC]
  provides: [DictionaryEditor rendered in General settings]
  affects: [src/components/sections/GeneralSection.tsx]
tech_stack:
  added: []
  patterns: [IPC load-on-mount with useEffect, IPC save-on-change handler]
key_files:
  created: []
  modified:
    - src/components/sections/GeneralSection.tsx
decisions:
  - Corrections state is self-contained in GeneralSection (not lifted to App.tsx) — no prop threading needed since corrections are only used in this section
metrics:
  duration: "~3 minutes"
  completed: "2026-03-05"
---

# Quick Task 45: Wire DictionaryEditor into GeneralSection Summary

**One-liner:** DictionaryEditor wired into GeneralSection as Card 3 via useState/useEffect loading from get_corrections IPC and saving on change via save_corrections IPC.

## What Was Done

Added the Corrections Dictionary card to GeneralSection.tsx as the third card below Output. The component loads correction entries from the Tauri backend on mount and persists changes immediately on every edit.

## Tasks Completed

| Task | Description | Commit |
|------|-------------|--------|
| 1 | Add DictionaryEditor card to GeneralSection with IPC load/save | 24e4edd |

## Changes

**`src/components/sections/GeneralSection.tsx`**
- Added imports: `useState`, `useEffect` from react; `invoke` from `@tauri-apps/api/core`; `DictionaryEditor` from `../DictionaryEditor`
- Added `corrections` state initialized to `{}`
- Added `useEffect` that calls `invoke('get_corrections')` on mount and sets state; errors logged to console
- Added `handleCorrectionsChange` async function that updates state and calls `invoke('save_corrections', { corrections: updated })`; errors logged to console
- Added Card 3 (Corrections Dictionary) after Card 2 (Output) rendering `<DictionaryEditor corrections={corrections} onChange={handleCorrectionsChange} />`

## Deviations from Plan

None - plan executed exactly as written.

## Verification

- `npx tsc --noEmit` passes with no errors
- DictionaryEditor import is used (no unused import warning)
- GeneralSection renders three cards: Activation, Output, Corrections Dictionary

## Self-Check: PASSED

- `src/components/sections/GeneralSection.tsx` exists and contains DictionaryEditor import and Card 3 JSX
- Commit `24e4edd` exists in git log
