---
created: 2026-03-04T23:18:00.000Z
title: Learn from user corrections to auto-improve dictionary
area: general
files: []
---

## Problem

When users manually correct transcription output, that correction data is lost. No modern voice-to-text tool learns from user corrections the way Dragon NaturallySpeaking used to — over time Dragon got better at recognizing the user's vocabulary and speech patterns by tracking edits.

## Solution

Track when users manually correct transcription output (e.g., via clipboard monitoring or a correction UI). Log original→corrected pairs and use that data to automatically grow/update the corrections dictionary over time. Could start simple — if a correction is made N times, auto-add it to the dictionary.
