---
phase: quick-43
plan: "01"
subsystem: frontend-settings
tags: [ui, settings, microphone, system]
dependency_graph:
  requires: [quick-42]
  provides: [system-tab-with-mic-selector]
  affects: [src/components/sections/SystemSection.tsx, src/components/Sidebar.tsx, src/App.tsx]
tech_stack:
  added: []
  patterns: [component-merge, sidebar-consolidation]
key_files:
  created: []
  modified:
    - src/components/sections/SystemSection.tsx
    - src/components/Sidebar.tsx
    - src/App.tsx
  deleted:
    - src/components/sections/MicrophoneSection.tsx
decisions:
  - MicrophoneSection absorbed inline into SystemSection rather than composed — avoids prop-threading through an extra wrapper layer
metrics:
  duration: "5 minutes"
  completed: "2026-03-04"
  tasks_completed: 1
  files_modified: 3
  files_deleted: 1
---

# Phase quick-43 Plan 01: Move Mic Selector into System Tab Summary

**One-liner:** Mic input device selector merged into System settings tab as a second card below Inference Status; dedicated Microphone sidebar item removed.

## What Was Built

- `SystemSection.tsx` now accepts `selectedMic` and `onSelectedMicChange` props and renders an "Input Device" card below the existing "Inference Status" card, absorbing all logic from the former `MicrophoneSection` (device list fetch via `list_input_devices`, change handler via `set_microphone` IPC + store persistence)
- `Sidebar.tsx`: removed `'microphone'` from the `SectionId` union type and removed the Microphone entry from the ITEMS array; Mic icon import retained (still used in logo div)
- `App.tsx`: removed `MicrophoneSection` import and its render block; `SystemSection` now receives `selectedMic` and `onSelectedMicChange={setSelectedMic}` props
- `MicrophoneSection.tsx` deleted

## Verification

- `npx tsc --noEmit` — zero errors
- Sidebar now shows: General, Model, Appearance, System, History (no Microphone)
- System tab renders both Inference Status and Input Device cards
- Device selection calls `set_microphone` IPC and persists to store via `getStore()`

## Deviations from Plan

None — plan executed exactly as written.

## Commits

| Hash | Description |
|------|-------------|
| 86966e2 | feat(quick-43): move mic selector into System tab and remove Microphone sidebar item |

## Self-Check: PASSED

- SystemSection.tsx exists and contains `list_input_devices` and `set_microphone` references
- MicrophoneSection.tsx deleted
- Sidebar.tsx no longer contains 'microphone' in SectionId or ITEMS
- Commit 86966e2 verified in git log
