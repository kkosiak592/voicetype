---
phase: 26-quantize-distil-large-v3-5
plan: 01
subsystem: download/model-metadata
tags: [quantization, whisper, distil-large-v3.5, q5_0, model-hosting]
dependency_graph:
  requires: []
  provides: [distil-large-v3.5-q5_0-download]
  affects: [src-tauri/src/download.rs, src-tauri/src/lib.rs, src/components/FirstRun.tsx]
tech_stack:
  added: []
  patterns: [q5_0 quantization via whisper.cpp quantize tool, GitHub releases for model hosting]
key_files:
  modified:
    - src-tauri/src/download.rs
    - src-tauri/src/lib.rs
    - src/components/FirstRun.tsx
decisions:
  - "Host q5_0 model on GitHub releases (kkosiak592/voicetype v1.2-models) instead of HuggingFace — user controls hosting"
  - "Local filename kept as ggml-distil-large-v3.5.bin — download URL changes, disk path stays same"
  - "Display size set to 513 MB (537,819,875 bytes / 1,048,576) to match existing MB-format convention for sub-1GB models"
metrics:
  duration: "~5 minutes (Task 1 was human-action, Tasks 2-3 are automated/verify)"
  completed_date: "2026-03-03"
  tasks: 2
  files: 3
---

# Phase 26 Plan 01: Quantize Distil Large v3.5 (fp16 -> q5_0) Summary

**One-liner:** Replaced fp16 distil-large-v3.5 (1.52 GB) with q5_0 quantized variant (~513 MB) hosted on GitHub releases, updating URL, SHA256, and size display across all three source files.

## What Was Built

- **download.rs:** `model_info()` entry for `"distil-large-v3.5"` now points at the q5_0 binary on GitHub releases with correct SHA256 (`e1cd9d36...`) and byte size (`537,819,875`)
- **lib.rs:** `ModelInfo` description updated from `"1.52 GB"` to `"513 MB"`
- **FirstRun.tsx:** MODELS array entry updated from `size: '1.52 GB'` to `size: '513 MB'`

## Tasks

| Task | Name | Status | Commit |
|------|------|--------|--------|
| 1 | Quantize fp16 model to q5_0 and host | Complete (human-action) | N/A |
| 2 | Update download.rs, lib.rs, and FirstRun.tsx metadata | Complete | 7ac54c1 |
| 3 | Verify q5_0 download and transcription | Pending human-verify | — |

## Model Values Applied

| Field | Old (fp16) | New (q5_0) |
|-------|-----------|-----------|
| URL | `distil-whisper/distil-large-v3.5-ggml/.../ggml-model.bin` | `kkosiak592/voicetype/releases/download/v1.2-models/ggml-distil-large-v3.5-q5_0.bin` |
| SHA256 | `ec2498919b...` | `e1cd9d36ee...` |
| Size (bytes) | `1,519,521,155` | `537,819,875` |
| Display size | `1.52 GB` | `513 MB` |
| Local filename | `ggml-distil-large-v3.5.bin` | `ggml-distil-large-v3.5.bin` (unchanged) |

## Verification

- `cargo check`: Finished with no errors (8.47s)
- `tsc --noEmit`: No errors
- All three files show consistent `513 MB` size for distil-large-v3.5

## Deviations from Plan

None — plan executed exactly as written. Values provided by user matched the expected format (URL, SHA256, byte size).

## Self-Check: PASSED

- Commit `7ac54c1` exists in git log
- `src-tauri/src/download.rs` exists with q5_0 URL, SHA256, and byte size
- `src-tauri/src/lib.rs` exists with `513 MB` description
- `src/components/FirstRun.tsx` exists with `size: '513 MB'`
