---
created: 2026-03-04T23:25:00.000Z
title: Add local LLM intent-to-text cleanup pass over raw transcription
area: general
files: []
---

## Problem

Raw transcription output is messy — it includes filler words, false starts, poor grammar, and lacks proper formatting. Users have to manually clean up dictated text. No modern voice-to-text tool provides an intelligent cleanup pass. This is the #1 differentiator opportunity in the market right now.

## Solution

Run a local LLM (e.g., small Llama or Phi model via ONNX Runtime) over the raw transcription to produce polished prose. Package the model with the distribution so users can download it. Example:

- Raw: "Uh so like I was thinking maybe we should uh move the meeting"
- Cleaned: "I think we should move the meeting."

The LLM pass would:
- Strip filler words and false starts (supersedes the simple regex filler removal todo)
- Fix grammar and punctuation
- Preserve the user's intent while producing natural prose
- Run entirely on-device with zero internet dependency
- Be toggleable in settings (raw vs. cleaned output)
- Need to be fast enough to not add significant latency — small model (1-3B params) should work
