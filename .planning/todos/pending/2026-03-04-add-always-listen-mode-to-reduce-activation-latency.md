---
created: 2026-03-04T23:22:00.000Z
title: Add always-listen mode to reduce activation latency
area: general
files: []
---

## Problem

There's a small latency when activating voice recording because the microphone stream has to be initialized each time. This delay is noticeable and breaks the flow of dictation.

## Solution

Add an "always listen" mode where the microphone stays open continuously, using VAD (Silero) to detect when speech starts. This eliminates the mic initialization latency — when the user starts speaking (or presses the hotkey), audio is already being captured. Should be a toggleable setting since keeping the mic open may have battery/resource implications.
