---
phase: quick-19
verified: 2026-03-02T00:00:00Z
status: passed
score: 5/5 must-haves verified
---

# Quick Task 19: Remove Herakeet int8 Model Verification Report

**Task Goal:** Remove Herakeet int8 model from entire codebase
**Verified:** 2026-03-02
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | No int8 Parakeet model entry appears in list_models output | VERIFIED | `list_models` in lib.rs (lines 875-905) contains only `large-v3-turbo`, `small-en`, and `parakeet-tdt-v2-fp32`. No int8 entry exists. |
| 2 | No int8 Parakeet model card appears in FirstRun or settings ModelSelector | VERIFIED | `MODELS` array in FirstRun.tsx has exactly 3 entries: large-v3-turbo, parakeet-tdt-v2-fp32, small-en. ModelSelector props interface contains only `onFp32Download`, `fp32Downloading`, `fp32Percent`, `fp32Error` — no int8 props. |
| 3 | download_parakeet_model Tauri command no longer exists | VERIFIED | download.rs contains no `download_parakeet_model` function. `PARAKEET_FILES` const is absent. `parakeet_model_dir()` and `parakeet_model_exists()` are absent. The invoke_handler in lib.rs (line 1229) registers only `download_parakeet_fp32_model`. |
| 4 | All Parakeet defaults point to parakeet-tdt-v2-fp32 | VERIFIED | `read_saved_parakeet_model` defaults to `"parakeet-tdt-v2-fp32"` (lib.rs line 200). All four fallback paths in `read_saved_parakeet_model_startup` return `"parakeet-tdt-v2-fp32"` (lines 209, 214, 218, 223). `resolve_parakeet_dir` calls `download::parakeet_fp32_model_dir()` (line 229). App.tsx uses `parakeet-tdt-v2-fp32` at lines 68, 69, 74. |
| 5 | App compiles without errors and FirstRun shows 3 GPU model cards | VERIFIED | FirstRun.tsx MODELS array has 3 entries; `gridClass` is `lg:grid-cols-3` for GPU users (line 164). All TypeScript types are consistent — no stale props or dangling references found. Code structure is coherent across all modified files. |

**Score:** 5/5 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src-tauri/src/download.rs` | Parakeet fp32 download only — no int8 download function or file list | VERIFIED | Contains `PARAKEET_FP32_FILES`, `parakeet_fp32_model_dir()`, `parakeet_fp32_model_exists()`, `download_parakeet_fp32_model`. No PARAKEET_FILES (int8), no `parakeet_model_dir`, no `download_parakeet_model`. |
| `src-tauri/src/lib.rs` | list_models with no int8 entry, defaults changed to fp32 | VERIFIED | `list_models` emits 3 models only. All 5 fallback/default sites return `parakeet-tdt-v2-fp32`. `download_parakeet_model` absent from invoke_handler. |
| `src/components/FirstRun.tsx` | 3-card GPU layout with no int8 card | VERIFIED | MODELS array has 3 entries; gridClass uses `lg:grid-cols-3`; handleDownload only has an `if (modelId === 'parakeet-tdt-v2-fp32')` branch. |
| `src/components/ModelSelector.tsx` | Simplified Parakeet handling — fp32 only | VERIFIED | Props interface has fp32 props only. `isParakeet = model.id === 'parakeet-tdt-v2-fp32'`. Download state resolution uses fp32 state vars directly. No isParakeetInt8 variable. |
| `src/App.tsx` | Engine reconciliation using fp32 model ID | VERIFIED | Lines 68, 69, 74 all reference `parakeet-tdt-v2-fp32`. No bare `parakeet-tdt-v2` string literal. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src-tauri/src/lib.rs` | `src-tauri/src/download.rs` | `download::parakeet_fp32_model_dir()` and `parakeet_fp32_model_exists()` | WIRED | lib.rs line 229 calls `download::parakeet_fp32_model_dir()`; line 902 calls `crate::download::parakeet_fp32_model_exists()`; line 933 calls same. Pattern `parakeet_fp32` present. |
| `src/components/sections/ModelSection.tsx` | `src/components/ModelSelector.tsx` | fp32 download props only — no int8 parakeet props | WIRED | ModelSection passes `onFp32Download`, `fp32Downloading`, `fp32Percent`, `fp32Error` to ModelSelector. No int8 props in the JSX call (lines 129-139). |
| `src/App.tsx` | `invoke('get_engine')` | Engine reconciliation defaults to fp32 | WIRED | App.tsx lines 66-76: invokes `get_engine`, and when result is `'parakeet'` sets `parakeet-tdt-v2-fp32`. Fallback filter at line 74 excludes `parakeet-tdt-v2-fp32`. |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| QUICK-19 | 19-PLAN.md | Remove Herakeet int8 model from entire codebase | SATISFIED | All int8 code paths removed from Rust backend (download.rs, lib.rs) and frontend (FirstRun.tsx, ModelSelector.tsx, ModelSection.tsx, App.tsx). Zero active-code references to bare `parakeet-tdt-v2` (without -fp32 suffix) found. |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src-tauri/src/lib.rs` | 271 | Stale comment: "which Parakeet variant (int8 or fp32) to load" | Info | No runtime effect — comment in `set_engine` doc block is outdated. Does not affect behavior. |
| `src-tauri/src/lib.rs` | 273 | Stale comment: "(int8 -> fp32 or fp32 -> int8) to take effect" | Info | No runtime effect — same doc block. Does not affect behavior. |

### Human Verification Required

None — all goal truths are fully verifiable from static code inspection.

### Gaps Summary

No gaps. All 5 observable truths are verified. The goal "Remove Herakeet int8 model from entire codebase" is achieved:

- The int8 download infrastructure (PARAKEET_FILES const, parakeet_model_dir, parakeet_model_exists, download_parakeet_model) is fully removed from download.rs.
- The int8 model entry is absent from list_models output in lib.rs.
- All default/fallback values have been changed to `parakeet-tdt-v2-fp32`.
- FirstRun shows a 3-card GPU layout with no int8 card.
- ModelSelector and ModelSection handle only fp32 Parakeet downloads.
- App.tsx engine reconciliation references only fp32.

Two stale doc comments in lib.rs mention "int8 or fp32" and "int8 -> fp32" — these are cosmetic and have zero behavioral impact.

---

_Verified: 2026-03-02_
_Verifier: Claude (gsd-verifier)_
