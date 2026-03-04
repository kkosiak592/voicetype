---
created: 2026-03-01T23:28:07.496Z
title: Remove structural profile and simplify to single profile with editable prompt
area: ui
files:
  - src/ (profile and settings components)
  - src-tauri/src/profiles.rs
  - src-tauri/src/transcribe.rs
---

## Problem

The "structural" profile option adds complexity with little value. The only real difference between general/structural is the Whisper `initial_prompt` — and the user no longer wants the structural option at all. Additionally, non-Whisper engines (Parakeet, Moonshine) don't use `initial_prompt`, making the profile concept even less useful.

Current state:
- Profile selector in UI with general/structural options
- Per-profile correction dictionaries (but dictionaries never actually differ)
- Per-profile `initial_prompt` for Whisper (the only meaningful difference)
- Profile selector should be disabled for Parakeet/Moonshine but isn't always

## Solution

1. Remove the structural profile entirely — no profile selector in UI
2. Merge correction dictionaries into a single shared dictionary (not per-profile)
3. Keep `initial_prompt` as a single editable text field in settings (not tied to a profile)
4. When engine is Parakeet or Moonshine, hide or disable the prompt field (they don't use it)
5. Clean up backend: simplify `ActiveProfile` to just hold the shared dictionary + prompt string
