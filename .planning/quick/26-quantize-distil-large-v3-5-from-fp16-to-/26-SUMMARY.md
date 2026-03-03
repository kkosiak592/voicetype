---
phase: 26-quantize-distil-large-v3-5
plan: 01
subsystem: model-downloads
tags: [whisper, quantization, q5_0, ggml, download, first-run]

dependency_graph:
  requires:
    - phase: 19-include-distil-large-v3-5
      provides: distil-large-v3.5 model entry in download.rs, lib.rs, and FirstRun.tsx
  provides:
    - distil-large-v3.5 downloads as q5_0 quantized (~513 MB) instead of fp16 (1.52 GB)
    - Correct SHA256 and byte-size metadata in download.rs
    - Consistent size display across lib.rs and FirstRun.tsx
    - set_model() correctly loads model after first-run download
  affects:
    - phase-20 (dual installer — model download metadata)
    - first-run UX (model size display)

tech-stack:
  added: []
  patterns:
    - "q5_0 quantization for medium-large Whisper models — consistent with large-v3-turbo"
    - "set_model() guards on WhisperContext presence (is_some()) not just settings.json model_id"

key-files:
  created: []
  modified:
    - src-tauri/src/download.rs
    - src-tauri/src/lib.rs
    - src/components/FirstRun.tsx
    - src-tauri/Cargo.toml

key-decisions:
  - "q5_0 for distil-large-v3.5 — matches quantization approach used for large-v3-turbo; 513 MB vs 1.52 GB fp16"
  - "Hosted on GitHub Releases (kkosiak592/voicetype v1.2-models) — same CDN as app updates"
  - "set_model() early-return requires is_some() guard on WhisperContext — settings.json model_id alone is insufficient after first-run"
  - "default-run = voice-to-text in Cargo.toml required when multiple [[bin]] targets exist"
  - "Display size 513 MB (537,819,875 bytes / 1,048,576) — sub-1GB models display in MB per existing convention"

patterns-established:
  - "Model skip-reload check: verify WhisperContext is in memory (is_some()), not just settings.json"

requirements-completed: [QUANT-01]

metrics:
  duration: "~45 min (including user-side quantization and upload)"
  completed_date: "2026-03-03"
  tasks: 3
  files: 4
---

# Quick Task 26: Quantize distil-large-v3.5 fp16 to q5_0 Summary

**distil-large-v3.5 switched from fp16 (1.52 GB) to q5_0 quantized (513 MB) hosted on GitHub Releases, with set_model() early-return bug fixed to ensure model loads correctly after first-run download**

## Performance

- **Duration:** ~45 min (including user-side quantization and upload to GitHub Releases)
- **Completed:** 2026-03-03
- **Tasks:** 3 (1 human-action, 1 auto, 1 human-verify) + 2 post-verify fixes
- **Files modified:** 4

## Accomplishments

- distil-large-v3.5 model downloads 513 MB (q5_0) instead of 1.52 GB (fp16) — 66% smaller
- Consistent size display: download.rs byte count, lib.rs description ("513 MB"), FirstRun.tsx model card all match
- SHA256 verification still enforced on the new quantized binary
- Fixed set_model() not loading model after first-run: settings.json had model_id but WhisperContext was None — added is_some() guard before early return
- Fixed cargo run failure when both voice-to-text and benchmark [[bin]] targets exist — added default-run to Cargo.toml

## Task Commits

1. **Task 1: Quantize fp16 model to q5_0 and host** — N/A (human-action: user built quantize tool, ran quantization, uploaded to GitHub Releases)
2. **Task 2: Update download.rs, lib.rs, and FirstRun.tsx** — `7ac54c1` (feat)
3. **Task 3: Verify q5_0 download and transcription** — N/A (human-verify: user approved)
4. **Post-verification fixes** — `d46e7ec` (fix: set_model early-return bug + Cargo.toml default-run)

## Model Values Applied

| Field | Old (fp16) | New (q5_0) |
|-------|-----------|-----------|
| URL | `distil-whisper/distil-large-v3.5-ggml/.../ggml-model.bin` | `kkosiak592/voicetype/releases/download/v1.2-models/ggml-distil-large-v3.5-q5_0.bin` |
| SHA256 | `ec2498919b...` | `e1cd9d36ee8628206fe0c8f9e067ee2679409b5845b4c4a14a7e2dd906fb9a19` |
| Size (bytes) | `1,519,521,155` | `537,819,875` |
| Display size | `1.52 GB` | `513 MB` |
| Local filename | `ggml-distil-large-v3.5.bin` | `ggml-distil-large-v3.5.bin` (unchanged) |

## Files Created/Modified

- `src-tauri/src/download.rs` — distil-large-v3.5 match arm: new GitHub Releases URL, SHA256, and 537,819,875 bytes
- `src-tauri/src/lib.rs` — ModelInfo description updated to "513 MB"; set_model() guard added (is_some() check before early return)
- `src/components/FirstRun.tsx` — MODELS array size changed from '1.52 GB' to '513 MB'
- `src-tauri/Cargo.toml` — Added `default-run = "voice-to-text"` to resolve ambiguous cargo run with multiple [[bin]] targets

## Decisions Made

- q5_0 quantization for distil-large-v3.5: matches the approach already used for large-v3-turbo (q5_0). 513 MB vs 1.52 GB — 66% reduction with negligible quality loss for CPU inference.
- Hosted on GitHub Releases under tag v1.2-models (kkosiak592/voicetype) — same CDN already used for app auto-updates.
- set_model() early-return requires checking WhisperContext is_some() in addition to settings.json model_id. The settings.json check alone is wrong: after first-run download, settings has the model_id but the app started with no model in memory (None), so the early return silently skipped the load.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] set_model() early-return skips model load after first-run download**
- **Found during:** Task 3 (human-verify — transcription failed after first-run download)
- **Issue:** set_model() checked settings.json model_id to decide if model was already loaded. After first-run download, settings.json had the model_id written, yet WhisperContext (WhisperStateMutex) was None — the app starts with no model in memory. The early return fired, skipping the actual model load, leaving transcription silently broken.
- **Fix:** Added WhisperStateMutex lock and is_some() check inside the model_id match block. Early return only fires when settings model_id matches AND context is already in memory. Added informational log line when settings match but context is None.
- **Files modified:** src-tauri/src/lib.rs
- **Verification:** Transcription worked after first-run download with the fix applied (user approved in Task 3)
- **Committed in:** d46e7ec

**2. [Rule 3 - Blocking] Missing default-run in Cargo.toml causes ambiguous cargo run failure**
- **Found during:** Task 3 (verification — cargo run failed with ambiguity error)
- **Issue:** Cargo.toml had two [[bin]] targets: voice-to-text and benchmark. Without default-run, `cargo run` errors with "error: could not determine which binary to run". The benchmark bin requires --features; plain cargo run used in development failed.
- **Fix:** Added `default-run = "voice-to-text"` to [package] section.
- **Files modified:** src-tauri/Cargo.toml
- **Verification:** cargo run resolves to voice-to-text binary without explicit --bin flag
- **Committed in:** d46e7ec

---

**Total deviations:** 2 auto-fixed (1 bug, 1 blocking)
**Impact on plan:** Both fixes are correctness requirements — without them the model is un-loadable after first-run and the dev build workflow is broken. No scope creep.

## Issues Encountered

- Task 1 required user to build whisper.cpp quantize tool from source, run quantization, and upload to GitHub Releases — correctly modeled as human-action checkpoint.
- The q5_0 binary resolved to 537,819,875 bytes (513 MB displayed), not the ~600 MB estimated in the plan objective. Actual size used in all three files.

## User Setup Required

None — the quantized model is hosted at a public GitHub Releases URL. No credentials or external service configuration required.

## Next Phase Readiness

- distil-large-v3.5 is available as a 513 MB download option, consistent with other quantized models in the app.
- Phase 20 (dual CPU/GPU installers) can reference this model without size concerns.
- The set_model() is_some() guard pattern should be considered canonical for any future model-switching logic.

## Self-Check: PASSED

- Commit `7ac54c1` exists: `feat(26): update distil-large-v3.5 to q5_0 quantized model`
- Commit `d46e7ec` exists: `fix(26): set_model early-return bug and Cargo.toml default-run`
- `src-tauri/src/download.rs` contains q5_0 GitHub Releases URL, SHA256 e1cd9d36..., 537,819,875 bytes
- `src-tauri/src/lib.rs` contains "513 MB" description and is_some() guard in set_model()
- `src/components/FirstRun.tsx` contains `size: '513 MB'`
- `src-tauri/Cargo.toml` contains `default-run = "voice-to-text"`

---
*Quick Task: 26-quantize-distil-large-v3-5*
*Completed: 2026-03-03*
