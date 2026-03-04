---
created: 2026-03-04T23:15:00.000Z
title: Add per-application profiles with auto-switch on focused window
area: general
files: []
---

## Problem

Currently the same transcription settings apply regardless of which application the user is dictating into. Different apps benefit from different correction dictionaries, formatting rules, and output styles (e.g., formal tone for Outlook, casual for Slack, code-friendly for VS Code).

## Solution

Detect the focused application via Windows API (e.g., `GetForegroundWindow` + process name) and auto-switch correction dictionaries, formatting rules, or output style based on configurable per-application profiles. Users should be able to map application executables to profiles in settings.
