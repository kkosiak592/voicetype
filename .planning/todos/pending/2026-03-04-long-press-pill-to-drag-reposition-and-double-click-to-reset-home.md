---
created: 2026-03-04T23:20:00.000Z
title: Long-press pill to drag reposition and double-click to reset home
area: ui
files: []
---

## Problem

The pill overlay is fixed in one position. Users may want to move it out of the way depending on what they're working on, but there's no way to reposition it.

## Solution

iPhone-style long-press to drag interaction:
- **Long-press (hold ~2 seconds):** Enter drag mode — pill becomes movable, user can drag it to any screen position. Visual feedback (jiggle/glow) to indicate drag mode is active, similar to iOS app icon rearranging.
- **Double-click:** Snap pill back to its default "home" position (current fixed location).
- Persist the custom position so it survives app restarts, but double-click always resets to default.
