---
created: 2026-03-04T19:54:37.340Z
title: Open settings window on tray icon double-click
area: ui
files:
  - src-tauri/src/tray.rs
  - src-tauri/src/lib.rs
---

## Problem

Currently, double-clicking the system tray icon does nothing. Users expect double-clicking the tray icon to open the settings window, which is the standard behavior for tray-based applications on Windows.

## Solution

Add a double-click event handler to the tray icon in `tray.rs` that emits a command or event to open/focus the settings window. Use Tauri's tray event API to detect `DoubleClick` and call `window.show()` / `window.set_focus()` on the main (settings) window.
