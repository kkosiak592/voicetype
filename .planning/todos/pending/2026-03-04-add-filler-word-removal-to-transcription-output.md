---
created: 2026-03-04T23:12:00.000Z
title: Add filler word removal to transcription output
area: general
files: []
---

## Problem

Transcription output includes filler words like "um", "uh", "like", "you know" which clutter the final text and require manual cleanup.

## Solution

Simple regex/dictionary approach — maintain a list of common filler words/phrases and strip them from the transcription output as a post-processing step before text injection. Should be toggleable in settings.
