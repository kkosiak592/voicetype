---
created: 2026-03-01T13:09:02.330Z
title: Investigate microphone icon persisting in system tray
area: ui
files:
  - src-tauri/src/tray.rs
---

## Problem

The microphone icon remains visible in the system tray (icon tray) at all times, even when it should presumably be hidden or change state. Need to investigate what controls the tray icon visibility lifecycle and determine why it isn't being dismissed/hidden when expected.

## Solution

TBD - Investigate tray icon management in `tray.rs` and related Tauri system tray configuration. Check if there's a missing cleanup, incorrect visibility toggle, or lifecycle event not being handled.
