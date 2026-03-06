---
created: 2026-03-06T15:37:46.909Z
title: Auto-detect caps lock state instead of manual toggle
area: general
files: []
---

## Problem

Currently caps lock dictation mode requires a manual toggle in the UI. The user wants the app to automatically detect the keyboard's caps lock state and mirror it in dictation output — if caps lock is on, transcribed text should be uppercased; if caps lock is off, normal casing applies. This removes the need for a separate toggle and keeps behavior consistent with what the user sees on their physical keyboard.

## Solution

- Use OS-level API to query the current caps lock state (e.g., `GetKeyState(VK_CAPITAL)` on Windows)
- Before inserting transcribed text, check caps lock state and transform casing accordingly
- Remove or deprecate the manual caps lock toggle from the UI
- Consider polling vs event-driven approach for detecting state changes mid-dictation
