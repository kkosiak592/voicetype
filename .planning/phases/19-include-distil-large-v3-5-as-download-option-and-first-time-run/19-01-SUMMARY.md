---
phase: 19-include-distil-large-v3-5-as-download-option-and-first-time-run
plan: 01
subsystem: ui, download
tags: [whisper, ggml, download, first-run, model-management, tauri, rust, react]

# Dependency graph
requires:
  - phase: 17-frontend-capture-ui
    provides: FirstRun.tsx component and model download infrastructure
provides:
  - distil-large-v3.5 as 4th downloadable model option (download.rs, lib.rs, FirstRun.tsx)
  - Per-model URL embedding in model_info() 4-tuple
  - SHA256-verified download of distil-large-v3.5 from distil-whisper HuggingFace repo
affects:
  - 20-implement-dual-cpu-gpu-installers-with-variant-specific-auto-updates

# Tech tracking
tech-stack:
  added: []
  patterns:
    - model_info() 4-tuple (filename, url, sha256, size) — all model metadata in one place,
      supports models from different HuggingFace repos without a separate routing function

key-files:
  created: []
  modified:
    - src-tauri/src/download.rs
    - src-tauri/src/lib.rs
    - src/components/FirstRun.tsx

key-decisions:
  - "SHA256 ec2498919b498c5f6b00041adb45650124b3cd9f26f545fffa8f5d11c28dcf26 obtained from
    LFS pointer file (huggingface.co/distil-whisper/distil-large-v3.5-ggml/raw/main/ggml-model.bin)
    — not hardcoded placeholder; verified exact file size 1519521155 bytes"
  - "model_info() refactored to 4-tuple embedding URL — download_url() removed; cleaner than
    per-repo routing function, single source of truth for all model metadata"
  - "xl:grid-cols-4 breakpoint added to FirstRun.tsx gridClass for 4-model layout"
  - "distil-large-v3.5 shown to all users (gpuOnly: false) — works on CPU, recommended for GPU"

patterns-established:
  - "Per-model 4-tuple in model_info(): each new Whisper model only needs one entry here +
    model_id_to_path() + list_models() + check_first_run() + FirstRun.tsx MODELS array"

requirements-completed: []

# Metrics
duration: 4min
completed: 2026-03-03
---

# Phase 19 Plan 01: distil-large-v3.5 Integration Summary

**distil-large-v3.5 (fp16 GGML, 1.52 GB) added as 4th downloadable model with SHA256-verified streaming download from the distil-whisper HuggingFace repo, model_info() refactored to 4-tuple with embedded URLs**

## Performance

- **Duration:** 4 min
- **Started:** 2026-03-03T15:17:24Z
- **Completed:** 2026-03-03T15:21:11Z
- **Tasks:** 2 (Task 3 is human-verify checkpoint)
- **Files modified:** 3

## Accomplishments

- Refactored `model_info()` from 3-tuple to 4-tuple `(filename, url, sha256, size)` — each model now embeds its own HuggingFace URL, enabling models from different repos without a routing function
- Removed `download_url()` function — URL is now per-model in `model_info()`
- Added `distil-large-v3.5` entry across all 5 required locations: `model_info()`, `model_id_to_path()`, `list_models()`, `check_first_run()` needs_setup predicate, and `FirstRun.tsx` MODELS array
- SHA256 hash obtained from LFS pointer file (not a placeholder): `ec2498919b498c5f6b00041adb45650124b3cd9f26f545fffa8f5d11c28dcf26`, file size corrected to 1,519,521,155 bytes
- Added `xl:grid-cols-4` breakpoint for clean 4-card layout on extra-large screens

## Task Commits

Each task was committed atomically:

1. **Task 1: Compute SHA256 and add distil-large-v3.5 to backend** - `a4dec26` (feat)
2. **Task 2: Add distil-large-v3.5 card to FirstRun.tsx** - `a95688e` (feat)

**Plan metadata:** (docs commit below)

## Files Created/Modified

- `src-tauri/src/download.rs` - model_info() 4-tuple, download_url() removed, download_model() updated
- `src-tauri/src/lib.rs` - model_id_to_path() new arm, list_models() new entry, check_first_run() distil_v35_exists
- `src/components/FirstRun.tsx` - distil-large-v3.5 card, xl:grid-cols-4 breakpoint

## Decisions Made

- SHA256 obtained from LFS pointer file (`/raw/main/ggml-model.bin`) rather than downloading 1.52 GB file — LFS pointer contains the exact SHA256 that git-lfs uses for content-addressed storage, identical to `sha256sum` of the file
- File size corrected from plan estimate (1,519,525,364) to actual LFS value (1,519,521,155) — difference was 4,209 bytes; the corrected value matches what HuggingFace headers report
- `xl:grid-cols-4` added to gridClass (plan marked this as "Claude's discretion") — improves layout when all 4 models are visible on GPU systems

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed `url` type after 4-tuple refactor**
- **Found during:** Task 1 (backend refactor)
- **Issue:** After changing `model_info()` to 4-tuple, `url` became `&'static str` instead of `String`. Two call sites required adjustment: `url.clone()` → `url.to_string()` for the Started event, and `.get(&url)` → `.get(url)` for the reqwest call (compiler caught `&&str` coercion error)
- **Fix:** Changed `url.clone()` to `url.to_string()` and `.get(&url)` to `.get(url)`
- **Files modified:** `src-tauri/src/download.rs`
- **Verification:** `cargo check --features whisper` passes
- **Committed in:** `a4dec26` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - type correction after planned refactor)
**Impact on plan:** Necessary compiler fix, expected consequence of the refactor. No scope creep.

## Issues Encountered

None - cargo check and TypeScript compilation both pass cleanly.

## Next Phase Readiness

- distil-large-v3.5 is downloadable from first-run and settings model selector
- Awaiting human verification (Task 3 checkpoint): first-run shows 4 cards, download + SHA256 validate + model activation, settings lists distil, re-launch skips first-run
- Phase 20 (dual CPU/GPU installers) can proceed after verification

---
*Phase: 19-include-distil-large-v3-5-as-download-option-and-first-time-run*
*Completed: 2026-03-03*

## Self-Check: PASSED

- FOUND: src-tauri/src/download.rs
- FOUND: src-tauri/src/lib.rs
- FOUND: src/components/FirstRun.tsx
- FOUND: .planning/phases/19-include-distil-large-v3-5-as-download-option-and-first-time-run/19-01-SUMMARY.md
- FOUND: a4dec26 (Task 1 commit)
- FOUND: a95688e (Task 2 commit)
