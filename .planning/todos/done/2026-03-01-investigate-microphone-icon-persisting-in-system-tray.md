---
created: 2026-03-01T13:09:02.330Z
title: Debug multiple duplicate tray icons appearing in system tray
area: ui
files:
  - src-tauri/src/tray.rs
  - src-tauri/src/lib.rs
  - src-tauri/src/pipeline.rs
---

## Problem

Multiple duplicate VoiceType tray icons are consistently appearing in the Windows system tray. Instead of a single tray icon, several identical icons accumulate. This was observed in the system tray overflow area where 4+ identical blue person/headset icons were visible simultaneously.

Possible causes:
- App process spawning multiple instances without cleanup
- Tray icon being re-created on each recording cycle without removing the previous one
- Tauri system tray lifecycle not properly managing icon creation/destruction
- On-demand audio capture (open on record start, drop after pipeline) may be creating new tray entries each time without the OS cleaning up stale ones

## Solution

TBD — Investigate:
1. Whether `set_tray_state()` in `tray.rs` creates new icons vs updating existing ones
2. Whether the audio capture on-demand pattern (Quick 30) introduces phantom tray entries from CPAL stream creation/destruction
3. Whether Windows is caching stale tray icons that only disappear on hover
4. Add explicit tray icon cleanup on app exit and between recording cycles
