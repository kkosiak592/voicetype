---
created: 2026-03-04T23:08:08.568Z
title: Remove stale Parakeet vocabulary prompting warning from model page
area: ui
files:
  - src/components/sections/ModelSection.tsx:235
---

## Problem

The model settings page still shows a warning message: "[Engine] doesn't support vocabulary prompting. Your corrections dictionary still applies." However, vocabulary prompting was already fully removed (quick task #38, commit 6c3616b). This text is now stale and confusing.

## Solution

Remove the warning text at `ModelSection.tsx:235` since vocabulary prompting no longer exists in the application. The entire vocabulary section and initial_prompt plumbing was already removed.
