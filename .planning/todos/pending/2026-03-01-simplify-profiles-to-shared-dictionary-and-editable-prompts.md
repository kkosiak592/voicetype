---
created: 2026-03-01T23:28:07.496Z
title: Simplify profiles to shared dictionary and editable prompts
area: ui
files:
  - src/ (profile and settings components)
---

## Problem

The current profile system (general vs structural) maintains separate correction dictionaries per profile, but in practice the word dictionaries will never differ between them. The only meaningful difference between profiles is the initial system prompt that guides transcription behavior.

Additionally, the profile's system prompt is not editable from the UI, and there's no handling for disabling the profile selector when the Parakeet model is chosen (since Parakeet doesn't use prompt-based correction).

Three issues to address:
1. **Shared dictionary**: Merge correction dictionaries into a single shared dictionary across all profiles instead of per-profile dictionaries
2. **Editable prompt in UI**: Allow users to edit the system prompt per profile directly in the settings UI
3. **Parakeet profile lock**: When Parakeet TDT model is selected, auto-set profile to "general" and disable the profile selector (Parakeet doesn't use prompt-based profiles)

## Solution

1. Refactor profile storage so dictionaries are stored at the top level (not nested per profile)
2. Profiles only store: name + system prompt text
3. Add a prompt editor textarea in the settings UI that appears based on selected profile
4. Add conditional logic to disable profile selector when engine is Parakeet TDT
