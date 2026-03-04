---
created: 2026-03-04T16:11:53.837Z
title: Add transcription history panel to settings
area: ui
files:
  - src/components/sections/ModelSection.tsx
  - src-tauri/src/pipeline.rs
  - src-tauri/src/lib.rs
---

## Problem

When a dictation is processed but the paste fails (e.g., focus changed, clipboard conflict, app blocked paste), the transcribed text is lost with no way to recover it. Users have no visibility into what was transcribed — if the text doesn't appear at the cursor, it's gone.

## Solution

Add a "Transcription History" section in the settings toolbar that shows recent dictations:
- Store last N transcriptions (e.g., 20) with timestamp, engine used, and the transcribed text
- Display as a scrollable list in settings — most recent first
- Each entry is selectable/copyable so users can manually paste if auto-paste failed
- Backend: persist to a small JSON file or in-memory ring buffer emitted via Tauri events
- Frontend: new section in settings panel showing the history list with copy-to-clipboard buttons
