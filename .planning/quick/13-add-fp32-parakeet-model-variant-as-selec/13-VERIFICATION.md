---
phase: quick-13
verified: 2026-03-01T00:00:00Z
status: passed
score: 5/5 must-haves verified
---

# Quick Task 13: Add fp32 Parakeet Model Variant Verification Report

**Task Goal:** Add fp32 Parakeet model variant as selectable option in settings UI alongside existing int8 model
**Verified:** 2026-03-01
**Status:** PASSED
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | User can see both 'Parakeet TDT (int8)' and 'Parakeet TDT (fp32)' as separate entries in settings model list and first-run flow | VERIFIED | `list_models` in lib.rs pushes `parakeet-tdt-v2-fp32` entry (line 907); FirstRun.tsx MODELS array has id `parakeet-tdt-v2-fp32` (line 36); int8 renamed confirmed via summary |
| 2 | User can download the fp32 model independently of the int8 model | VERIFIED | `PARAKEET_FP32_FILES` const at download.rs line 222; `download_parakeet_fp32_model` command at line 406 uses `parakeet_fp32_model_dir()` (separate dir); registered in invoke_handler at lib.rs line 1244 |
| 3 | Selecting fp32 variant sets engine to parakeet and loads from the fp32 model directory | VERIFIED | ModelSection.tsx line 62: `invoke('set_engine', { engine: 'parakeet', parakeetModel: modelId })`; lib.rs `resolve_parakeet_dir` maps `parakeet-tdt-v2-fp32` to `parakeet_fp32_model_dir()` (line 230) |
| 4 | Switching between int8 and fp32 reloads the Parakeet model from the correct directory | VERIFIED | `set_engine` has no `is_none` guard — always reloads (lib.rs line 297–298: unconditional `resolve_parakeet_dir` + load); variant persisted to settings.json at line 332 |
| 5 | Both variants can be downloaded and exist on disk simultaneously | VERIFIED | int8 dir: `parakeet_model_dir()` → `models/parakeet-tdt-v2`; fp32 dir: `parakeet_fp32_model_dir()` → `models/parakeet-tdt-v2-fp32` — fully separate paths |

**Score:** 5/5 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src-tauri/src/download.rs` | fp32 file list, download command, dir helper, exists check | VERIFIED | `PARAKEET_FP32_FILES` (line 222), `parakeet_fp32_model_dir()` (line 237), `parakeet_fp32_model_exists()` (line 255), `download_parakeet_fp32_model` command (line 406) |
| `src-tauri/src/lib.rs` | fp32 ModelInfo entry in list_models, parakeet_model setting, variant-aware set_engine and startup | VERIFIED | `parakeet-tdt-v2-fp32` entry in list_models (line 907); `set_engine` with `parakeet_model: Option<String>` (line 280); `read_saved_parakeet_model` (line 195); `resolve_parakeet_dir` (line 228); startup loading uses variant-aware dir (lines 1311–1312) |
| `src/components/sections/ModelSection.tsx` | fp32 download state management and handler | VERIFIED | `fp32Downloading` state (line 24); `handleFp32Download` (line 126) invokes `download_parakeet_fp32_model` (line 157); fp32 props passed to ModelSelector (lines 183–184) |
| `src/components/ModelSelector.tsx` | fp32-aware Parakeet card rendering with download buttons | VERIFIED | `onFp32Download`, `fp32Downloading` props (lines 28–29, 48–49); `isParakeetFp32` (line 120); per-variant state resolution (lines 124–127); `disabled` includes `fp32Downloading` (line 137) |
| `src/components/FirstRun.tsx` | fp32 entry in MODELS array | VERIFIED | `id: 'parakeet-tdt-v2-fp32'` in MODELS (line 36); `download_parakeet_fp32_model` dispatch for fp32 (lines 138–139); `set_engine` with `parakeetModel: downloadingId` for both variants (lines 81–83) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `ModelSection.tsx` | `download::download_parakeet_fp32_model` | `invoke('download_parakeet_fp32_model')` | WIRED | Line 157 in ModelSection.tsx; command registered in lib.rs invoke_handler line 1244 |
| `ModelSection.tsx` | `lib.rs set_engine` | `handleModelSelect always calls set_engine with parakeetModel for parakeet variants` | WIRED | Lines 55–66: both `parakeet-tdt-v2` and `parakeet-tdt-v2-fp32` always call `set_engine` with `parakeetModel: modelId`, skipping the `engine !== currentEngine` guard |
| `lib.rs set_engine` | `download::parakeet_fp32_model_dir` | `reads parakeet_model from settings to resolve correct model directory` | WIRED | lib.rs line 297: `unwrap_or_else(|| read_saved_parakeet_model(&app))`; line 298: `resolve_parakeet_dir(&parakeet_model_id)`; line 230: maps `parakeet-tdt-v2-fp32` to `parakeet_fp32_model_dir()` |

### Anti-Patterns Found

None detected. No TODO/FIXME/placeholder comments, no empty implementations, no stub handlers in any of the 5 modified files.

### Human Verification Required

**1. Visual — Two Parakeet Cards in Settings UI**
Test: Open Settings > Models tab. Verify two separate cards appear: "Parakeet TDT (int8)" and "Parakeet TDT (fp32)".
Expected: Both cards visible with correct names, sizes, and independent download buttons.
Why human: Card rendering depends on runtime `list_models` response and React state — not statically verifiable.

**2. Visual — Two Parakeet Cards in First Run Flow**
Test: Clear settings and launch app in first-run mode with GPU detected. Verify fp32 card appears alongside int8 with "2.56 GB" size and "Full precision (GPU)" quality badge.
Expected: 4-column grid (lg) with fp32 card present, "Fastest" badge on int8 only.
Why human: GPU detection and grid layout are runtime-dependent.

**3. Behavioral — Variant Switch Reloads Model**
Test: Download both variants. Select int8, confirm transcription works. Switch to fp32, confirm model reloads without app restart and transcription works from fp32 directory.
Expected: Switching triggers reload; no restart required; transcription uses correct model.
Why human: Model loading is a runtime process requiring actual ONNX runtime.

### Gaps Summary

No gaps. All 5 must-have truths verified, all 5 artifacts substantive and wired, all 3 key links confirmed in code. Build confirmed passing per SUMMARY.md (cargo build + tsc both passed). Implementation matches plan exactly with no deviations reported.

---

_Verified: 2026-03-01_
_Verifier: Claude (gsd-verifier)_
