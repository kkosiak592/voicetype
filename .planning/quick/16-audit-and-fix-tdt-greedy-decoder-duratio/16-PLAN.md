---
phase: quick-16
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/patches/parakeet-rs/src/model_tdt.rs
autonomous: false
requirements: [QUICK-16]

must_haves:
  truths:
    - "TDT greedy decoder advances frame pointer on non-blank tokens when duration_step > 0, matching onnx-asr reference"
    - "Blank token with duration_step == 0 always advances by 1 frame (no stall)"
    - "Duration step is always honored regardless of emitted_tokens count — no emitted_tokens > 0 guard on advancement"
    - "LSTM decoder state only updates on non-blank token emission, matching onnx-asr and sherpa-onnx reference"
    - "Both int8 and fp32 Parakeet variants produce output without dropped words"
  artifacts:
    - path: "src-tauri/patches/parakeet-rs/src/model_tdt.rs"
      provides: "Corrected TDT greedy decode loop"
      contains: "if duration_step > 0"
  key_links:
    - from: "src-tauri/patches/parakeet-rs/src/model_tdt.rs"
      to: "onnx-asr _AsrWithTransducerDecoding._decoding()"
      via: "Identical frame-advancement logic"
      pattern: "if duration_step > 0"
---

<objective>
Fix the TDT greedy decoder in parakeet-rs to match the onnx-asr reference implementation's frame-advancement logic. Three bugs cause word dropping:

1. Duration step is ignored on non-blank tokens — reference advances by `step` on ANY token when `step > 0`
2. `emitted_tokens > 0` guard prevents duration-based advancement at utterance start and after blank sequences
3. LSTM state updates on blank tokens corrupt the prediction network (reference only updates state on non-blank)

The fix restructures the decode loop to match the proven onnx-asr pattern exactly.

Purpose: Eliminate dropped/missing words in Parakeet TDT transcription
Output: Corrected `greedy_decode` function in model_tdt.rs
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src-tauri/patches/parakeet-rs/src/model_tdt.rs
@src-tauri/patches/parakeet-rs/src/decoder_tdt.rs

<interfaces>
<!-- Reference: onnx-asr _AsrWithTransducerDecoding._decoding() from asr.py -->
<!-- This is the CORRECT algorithm our Rust code must match -->

```python
# From https://github.com/istupakov/onnx-asr/blob/main/src/onnx_asr/asr.py
# class _AsrWithTransducerDecoding:

def _decoding(self, encoder_out, encoder_out_lens, /, **kwargs):
    for encodings, encodings_len in zip(encoder_out, encoder_out_lens):
        prev_state = self._create_state()
        tokens = []
        timestamps = []
        t = 0
        emitted_tokens = 0
        while t < encodings_len:
            logits, step, state = self._decode(tokens, prev_state, encodings[t])
            # step = duration_logits.argmax() (from NemoConformerTdt._decode)
            token = logits.argmax()

            if token != self._blank_idx:
                prev_state = state          # STATE ONLY UPDATES ON NON-BLANK
                tokens.append(int(token))
                timestamps.append(t)
                emitted_tokens += 1

            if step > 0:                    # DURATION APPLIES TO ANY TOKEN
                t += step
                emitted_tokens = 0
            elif token == self._blank_idx or emitted_tokens == self._max_tokens_per_step:
                t += 1                      # BLANK+step=0 OR MAX TOKENS: advance 1
                emitted_tokens = 0
            # else: non-blank + step=0 — stay on same frame, try more tokens
```

<!-- Reference: sherpa-onnx PR #2606 fix (C++) -->
<!-- Same algorithm, confirms onnx-asr is correct -->

```cpp
// After fix: skip = duration_logits.argmax() (can be 0)
// Non-blank: emit, update decoder, tokens_this_frame++
// if (skip > 0) { tokens_this_frame = 0; }  // will advance
// if (tokens_this_frame >= max_tokens_per_frame) { skip = 1; }
// if (y == blank_id && skip == 0) { skip = 1; }
// loop: t += skip
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Rewrite greedy_decode frame-advancement logic to match onnx-asr reference</name>
  <files>src-tauri/patches/parakeet-rs/src/model_tdt.rs</files>
  <action>
Rewrite the `greedy_decode` method (lines 173-284) to fix three bugs. The new decode loop must exactly mirror the onnx-asr `_AsrWithTransducerDecoding._decoding()` algorithm. Here are the specific changes:

**Bug 1 — Duration step must apply to ALL tokens, not just blank:**

Current (WRONG):
```rust
if token_id != blank_id {
    // emit token
    // Never advances — stays on same frame always
} else {
    // Only blank branch can advance
    if duration_step > 0 && emitted_tokens > 0 { t += duration_step; }
    else { t += 1; }
}
```

Fixed (match onnx-asr):
```rust
if token_id != blank_id {
    tokens.push(token_id);
    frame_indices.push(t);
    durations.push(duration_step);
    last_emitted_token = token_id as i32;
    emitted_tokens += 1;
}

// Frame advancement — separate from token emission (key insight from onnx-asr)
if duration_step > 0 {
    // Duration prediction says advance — honor it for ANY token type
    t += duration_step;
    emitted_tokens = 0;
} else if token_id == blank_id || emitted_tokens >= max_tokens_per_step {
    // Blank with step=0: advance by 1 (prevent stall)
    // OR max tokens hit: force advance
    t += 1;
    emitted_tokens = 0;
}
// else: non-blank + step=0 — stay on same frame, try more tokens
```

**Bug 2 — Remove `emitted_tokens > 0` guard:**
The old `duration_step > 0 && emitted_tokens > 0` condition is gone. The new code checks only `duration_step > 0`. This ensures duration predictions are honored at utterance start and after blank sequences.

**Bug 3 — LSTM state should only update on non-blank tokens:**

Current (WRONG — updates state on every step including blank):
```rust
// Always update LSTM states
if let Ok(...) = outputs["output_states_1"]... { state_h = ...; }
if let Ok(...) = outputs["output_states_2"]... { state_c = ...; }
```

Fixed (match onnx-asr where `prev_state = state` is inside `if token != blank_idx`):
Move the LSTM state extraction INSIDE the `if token_id != blank_id` block:
```rust
if token_id != blank_id {
    // Update LSTM states only on non-blank (matches onnx-asr: prev_state = state)
    if let Ok((h_shape, h_data)) = outputs["output_states_1"].try_extract_tensor::<f32>() {
        let dims = h_shape.as_ref();
        state_h = Array3::from_shape_vec(
            (dims[0] as usize, dims[1] as usize, dims[2] as usize),
            h_data.to_vec()
        ).map_err(|e| Error::Model(format!("Failed to update state_h: {e}")))?;
    }
    if let Ok((c_shape, c_data)) = outputs["output_states_2"].try_extract_tensor::<f32>() {
        let dims = c_shape.as_ref();
        state_c = Array3::from_shape_vec(
            (dims[0] as usize, dims[1] as usize, dims[2] as usize),
            c_data.to_vec()
        ).map_err(|e| Error::Model(format!("Failed to update state_c: {e}")))?;
    }

    tokens.push(token_id);
    frame_indices.push(t);
    durations.push(duration_step);
    last_emitted_token = token_id as i32;
    emitted_tokens += 1;
}
```

Remove the old comment about "prediction network must accumulate context from every step including blank" — that comment was wrong. The onnx-asr reference and sherpa-onnx both only update state on non-blank emission.

**Summary of the complete rewritten decode loop body (inside `while t < time_steps`):**

1. Extract encoder frame at position `t`
2. Build targets from `last_emitted_token`
3. Run `decoder_joint` session
4. Extract vocab logits and duration logits
5. `token_id = argmax(vocab_logits)`, `duration_step = argmax(duration_logits)`
6. If `token_id != blank_id`: update LSTM states from outputs, push token/frame/duration, update `last_emitted_token`, increment `emitted_tokens`
7. If `duration_step > 0`: `t += duration_step; emitted_tokens = 0;`
8. Else if `token_id == blank_id || emitted_tokens >= max_tokens_per_step`: `t += 1; emitted_tokens = 0;`
9. (else: non-blank + step=0 — stay on same frame for next iteration)

Do NOT change any other part of model_tdt.rs (encoder, model loading, etc). Only the decode loop body changes.
  </action>
  <verify>
    <automated>cd src-tauri && cargo check --features cuda 2>&1 | tail -5</automated>
  </verify>
  <done>
    - greedy_decode compiles without errors
    - Frame advancement uses `if duration_step > 0` without emitted_tokens guard
    - Duration step applies to both blank and non-blank tokens
    - LSTM state updates are inside the `if token_id != blank_id` block
    - max_tokens_per_step safety check is preserved
    - No other functions in model_tdt.rs are modified
  </done>
</task>

<task type="checkpoint:human-verify" gate="blocking">
  <name>Task 2: Verify transcription produces complete output without dropped words</name>
  <files>src-tauri/patches/parakeet-rs/src/model_tdt.rs</files>
  <action>Human verifies the TDT decoder fix produces complete transcriptions.</action>
  <what-built>
    Rewrote the TDT greedy decoder frame-advancement logic to match the onnx-asr reference implementation. Three bugs fixed:
    1. Duration step now applies to ALL tokens (not just blank) — prevents skipping frames that would cause word drops
    2. Removed `emitted_tokens > 0` guard that was suppressing duration predictions at utterance start
    3. LSTM state only updates on non-blank tokens — prevents prediction network state corruption
  </what-built>
  <how-to-verify>
    1. Build: `cd src-tauri && cargo build --features cuda`
    2. Launch the app and select either Parakeet model (int8 or fp32)
    3. Record a sentence with multiple words, e.g. "How is everybody doing today"
    4. Verify ALL words appear in the transcription — no dropped words
    5. Test with a longer sentence to confirm no regression: "The quick brown fox jumps over the lazy dog"
    6. If using fp32 variant, switch to int8 and repeat — both must produce complete output
  </how-to-verify>
  <verify>Human confirms no dropped words in transcription output</verify>
  <done>Both int8 and fp32 Parakeet variants produce complete transcription without dropped words</done>
  <resume-signal>Type "approved" if transcription is complete with no dropped words, or describe which words are still missing</resume-signal>
</task>

</tasks>

<verification>
- `cargo check --features cuda` passes in src-tauri
- The decode loop structure matches onnx-asr `_AsrWithTransducerDecoding._decoding()` algorithm
- No words dropped in manual transcription test
</verification>

<success_criteria>
- Parakeet TDT transcription produces complete output without dropped words
- Both int8 and fp32 model variants work correctly
- Frame advancement logic matches the proven onnx-asr reference implementation
</success_criteria>

<output>
After completion, create `.planning/quick/16-audit-and-fix-tdt-greedy-decoder-duratio/16-SUMMARY.md`
</output>
