---
phase: quick-16
plan: "01"
subsystem: parakeet-rs / TDT decoder
tags: [bug-fix, tdt, parakeet, decoder, frame-advancement, lstm]
dependency_graph:
  requires: []
  provides: [correct-tdt-greedy-decode]
  affects: [parakeet-int8-transcription, parakeet-fp32-transcription]
tech_stack:
  added: []
  patterns: [onnx-asr-reference-algorithm, rnn-t-greedy-decoding]
key_files:
  modified:
    - src-tauri/patches/parakeet-rs/src/model_tdt.rs
decisions:
  - "LSTM state updates only on non-blank token emission — matches onnx-asr prev_state = state inside if token != blank_idx block"
  - "Frame advancement check is unconditional on duration_step > 0 with no emitted_tokens guard — required for utterance start and post-blank sequences"
  - "max_tokens_per_step safety valve folded into else-if branch — no longer a separate post-loop check"
metrics:
  duration: "~5 minutes"
  completed: "2026-03-02"
  tasks_completed: 1
  files_modified: 1
---

# Phase quick-16 Plan 01: Audit and Fix TDT Greedy Decoder Duration Logic Summary

Corrected TDT greedy decoder in parakeet-rs to match onnx-asr reference: duration step now applies to all tokens (not just blank), no emitted_tokens guard on advancement, and LSTM state updates only on non-blank emission.

## What Was Built

Rewrote the decode loop body in `src-tauri/patches/parakeet-rs/src/model_tdt.rs` `greedy_decode()` (lines 243-281) to fix three bugs that caused word dropping in Parakeet TDT transcription.

### Bugs Fixed

**Bug 1 — Duration step must apply to ALL tokens, not just blank**

Old code advanced frames only in the blank branch: `if duration_step > 0 && emitted_tokens > 0 { t += duration_step; }`. Non-blank tokens never advanced the frame pointer via duration.

Fixed: frame advancement is now a separate block after the non-blank/blank emission block. `if duration_step > 0 { t += duration_step; emitted_tokens = 0; }` applies regardless of token type.

**Bug 2 — Remove `emitted_tokens > 0` guard**

The `&& emitted_tokens > 0` condition suppressed duration-based advancement at utterance start (where `emitted_tokens` is 0 before any token is seen) and after blank sequences. The onnx-asr reference has no such guard.

Fixed: removed entirely. The check is now simply `if duration_step > 0`.

**Bug 3 — LSTM state only updates on non-blank tokens**

Old code updated `state_h` and `state_c` unconditionally after every decoder_joint call, including blank tokens. The onnx-asr reference does `prev_state = state` only inside `if token != self._blank_idx`.

Fixed: state extraction moved inside `if token_id != blank_id` block.

## Algorithm Comparison

| Step | onnx-asr reference | old parakeet-rs | fixed parakeet-rs |
|------|-------------------|-----------------|-------------------|
| State update | Non-blank only | Every step | Non-blank only |
| Duration advance | Any token, step>0 | Blank only, step>0, emitted>0 | Any token, step>0 |
| Blank + step=0 | t+=1 | t+=1 | t+=1 |
| Non-blank + step=0 | Stay on frame | Stay on frame | Stay on frame |
| Max tokens | emitted >= max -> t+=1 | emitted >= max -> t+=1 (separate check) | emitted >= max -> t+=1 (in else-if) |

## Deviations from Plan

None - plan executed exactly as written.

## Commits

| Task | Description | Commit |
|------|-------------|--------|
| 1 | Rewrite greedy_decode frame-advancement to match onnx-asr | 4ee5b69 |

## Self-Check

- [x] `src-tauri/patches/parakeet-rs/src/model_tdt.rs` modified
- [x] `cargo check` passes (verified during execution)
- [x] Commit 4ee5b69 exists
- [x] `if duration_step > 0` without `emitted_tokens > 0` guard present
- [x] LSTM state update inside `if token_id != blank_id` block
- [x] `max_tokens_per_step` safety check preserved

## Self-Check: PASSED
