---
phase: quick-39
plan: 01
subsystem: frontend-ui
tags: [ui, settings, all-caps, toggle, tauri-ipc]
dependency_graph:
  requires: [get_all_caps backend command, set_all_caps backend command]
  provides: [AllCapsToggle component, Output card in General Settings]
  affects: [GeneralSection.tsx]
tech_stack:
  added: []
  patterns: [Tauri IPC invoke, pulse skeleton loading state, emerald toggle button]
key_files:
  created:
    - src/components/AllCapsToggle.tsx
  modified:
    - src/components/sections/GeneralSection.tsx
decisions:
  - Backend is source of truth for all_caps — no store persistence in AllCapsToggle (matches plan intent, profiles.rs owns state)
metrics:
  duration: ~5 minutes
  completed: "2026-03-04T23:15:19Z"
  tasks_completed: 2
  files_changed: 2
---

# Phase quick-39 Plan 01: Add ALL CAPS Toggle to Frontend UI Summary

**One-liner:** Self-contained AllCapsToggle component wired to get_all_caps/set_all_caps Tauri IPC, rendered in a new Output card in General Settings.

## What Was Built

- `src/components/AllCapsToggle.tsx`: Component modelled on AutostartToggle. Calls `invoke<boolean>('get_all_caps')` on mount to read current state; shows a pulse skeleton while loading; calls `invoke('set_all_caps', { enabled: next })` on click. Uses emerald/gray toggle styling with `role="switch"`, `aria-checked`, and `sr-only` label.

- `src/components/sections/GeneralSection.tsx`: Added a second card below the existing Activation card. The new Output card contains a row with "ALL CAPS" label, descriptive subtitle ("Convert all transcribed text to uppercase"), and the self-contained `AllCapsToggle` component.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Create AllCapsToggle component | 14d35ff | src/components/AllCapsToggle.tsx |
| 2 | Add ALL CAPS card to GeneralSection | 0fbdba9 | src/components/sections/GeneralSection.tsx |

## Verification

`npx tsc --noEmit` — zero errors after both tasks.

## Deviations from Plan

None - plan executed exactly as written.
