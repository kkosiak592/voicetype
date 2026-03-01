---
created: 2026-03-01T13:10:01.000Z
title: Multi-monitor support — pill follows active cursor monitor
area: ui
files:
  - src-tauri/src/pill.rs
---

## Problem

The pill currently only appears on one monitor. On multi-monitor setups, the pill should appear on whichever monitor the user's cursor is currently on. This ensures the pill is always visible and contextually relevant to the user's active workspace.

## Solution

- On pill show/activation, detect which monitor the cursor is currently on
- Use that monitor's work area (bounds minus taskbar) to calculate the bottom-center position
- Position the pill at horizontal center, just above the taskbar of that specific monitor
- Depends on fixed-position pill work being done first (no more dragging)
