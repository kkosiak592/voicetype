---
created: 2026-03-01T13:10:00.000Z
title: Fix pill position to bottom center above taskbar, remove dragging
area: ui
files:
  - src/components/Pill.tsx
  - src-tauri/src/pill.rs
---

## Problem

The pill is currently draggable, which is not desired. Instead it should have a fixed position: always centered horizontally at the bottom of the screen, positioned just above the Windows taskbar. No drag behavior should remain.

## Solution

- Remove all drag/move logic from the pill window (both Rust side and frontend)
- Calculate fixed position: horizontally centered on the screen, vertically placed just above the taskbar
- Set the pill window position on creation and on show, not allowing user repositioning
- This todo should be done BEFORE multi-monitor support (which builds on fixed positioning)
