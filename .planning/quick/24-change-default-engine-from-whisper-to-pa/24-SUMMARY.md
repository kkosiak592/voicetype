---
phase: quick-24
plan: 01
subsystem: backend
tags: [engine-selection, gpu-detection, first-launch, parakeet, whisper]
dependency_graph:
  requires: [CachedGpuMode state managed before setup() engine block]
  provides: [GPU-aware default engine on first launch]
  affects: [src-tauri/src/lib.rs read_saved_engine()]
tech_stack:
  added: []
  patterns: [GPU-aware default selection via cached state, explicit match arms for engine strings]
key_files:
  modified:
    - src-tauri/src/lib.rs
decisions:
  - "Use CachedGpuMode state (already available) rather than re-detecting GPU at read time"
  - "Add explicit Some(\"whisper\") arm alongside Some(\"parakeet\") for clarity and future-proofing"
  - "Keep builder default (line 1304) as Whisper — it is immediately overwritten by setup() so changing it adds no value"
metrics:
  duration_minutes: 5
  completed_date: "2026-03-02T18:27:39Z"
  tasks_completed: 1
  files_changed: 1
---

# Quick Task 24: Change Default Engine from Whisper to Parakeet (GPU-Aware)

**One-liner:** GPU-aware default engine selection — Parakeet on first launch when GPU detected, Whisper when CPU-only.

## What Was Done

Modified `read_saved_engine()` in `src-tauri/src/lib.rs` to accept a `gpu_mode: bool` parameter and select the default engine based on GPU availability. Updated the caller in `setup()` to extract `CachedGpuMode` state and pass the bool.

## Changes

### A. `read_saved_engine()` (lines 177-204)

- **Signature changed:** `fn read_saved_engine(app: &tauri::App, gpu_mode: bool) -> TranscriptionEngine`
- **New logic:** Compute `default_engine = if gpu_mode { Parakeet } else { Whisper }` at the top
- **Error paths:** All three `return TranscriptionEngine::Whisper` replaced with `return default_engine`
- **Match arm added:** Explicit `Some("whisper") => TranscriptionEngine::Whisper`
- **Catch-all changed:** `_ => TranscriptionEngine::Whisper` changed to `_ => default_engine`
- **Doc comment updated** to describe GPU-aware behavior

### B. `setup()` caller (lines 1380-1390)

- Extract `CachedGpuMode` from app state using existing pattern
- Pass `gpu_mode` bool to `read_saved_engine(app, gpu_mode)`

## Verification

- `cargo check --features whisper,parakeet` passes with no errors
- No hardcoded `return TranscriptionEngine::Whisper` remains in `read_saved_engine` body
- Explicit `Some("whisper")` arm present for saved-preference compatibility

## Deviations from Plan

None - plan executed exactly as written.

## Commits

| Hash | Message |
|------|---------|
| f4c22a3 | feat(quick-24): default to Parakeet on GPU, Whisper on CPU-only |

## Self-Check: PASSED

- `src-tauri/src/lib.rs` modified: confirmed
- Commit f4c22a3 exists: confirmed
- `cargo check` clean: confirmed
