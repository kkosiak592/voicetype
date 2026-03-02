---
phase: quick-18
plan: "01"
subsystem: backend
tags: [gpu, whisper, transcription, performance]
dependency_graph:
  requires: [CachedGpuMode state registered at startup]
  provides: [GPU acceleration for all Whisper models based on CachedGpuMode]
  affects: [set_model command, startup loader, list_models description]
tech_stack:
  added: []
  patterns: [CachedGpuMode state lookup instead of model_id string matching]
key_files:
  modified:
    - src-tauri/src/lib.rs
decisions:
  - "All Whisper models use CachedGpuMode for GPU selection — no model_id-to-mode hardcoding remains"
  - "small-en description dynamically reflects GPU capability: GPU accelerated vs works on any CPU"
metrics:
  duration: "~3 minutes"
  completed: "2026-03-02"
  tasks_completed: 1
  files_modified: 1
---

# Quick Task 18: Enable GPU Acceleration for Whisper small.en Summary

**One-liner:** All Whisper models now use CachedGpuMode for GPU selection — small.en gets GPU acceleration on NVIDIA hardware instead of being hardcoded to CPU.

## What Was Done

Replaced two hardcoded `if model_id == "large-v3-turbo"` GPU mode checks with `app.state::<CachedGpuMode>().0.clone()` lookups. Also updated the small-en model description to conditionally show GPU capability.

## Tasks Completed

| Task | Description | Commit |
|------|-------------|--------|
| 1 | Replace hardcoded model_id GPU check with CachedGpuMode lookup | da85907 |

## Changes Made

### `src-tauri/src/lib.rs`

**set_model() (~line 997):** Replaced:
```rust
// BEFORE:
let mode = if model_id == "large-v3-turbo" {
    crate::transcribe::ModelMode::Gpu
} else {
    crate::transcribe::ModelMode::Cpu
};

// AFTER:
let mode = app.state::<CachedGpuMode>().0.clone();
```

**Startup loader (~line 1438):** Same replacement for the saved-model loading branch.

**list_models() (~line 889):** Updated small-en description from static string to:
```rust
description: if gpu_mode {
    "Fast — 190 MB — GPU accelerated".to_string()
} else {
    "Fast — 190 MB — works on any CPU".to_string()
},
```

## Verification

- `cargo check --features whisper` passes with no errors
- No remaining `model_id == "large-v3-turbo"` GPU checks in lib.rs
- Fallback path at ~line 1465 (auto-detect on startup) already used CachedGpuMode and was left unchanged

## Deviations from Plan

None - plan executed exactly as written.

## Self-Check: PASSED

- File modified: `src-tauri/src/lib.rs` - confirmed
- Commit da85907 - confirmed
- No remaining hardcoded GPU checks - confirmed via grep
- cargo check passes - confirmed
