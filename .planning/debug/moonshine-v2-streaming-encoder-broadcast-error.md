---
status: awaiting_human_verify
trigger: "Moonshine v2 streaming model fails on 60s audio clips with ORT broadcast dimension mismatch in encoder Add node"
created: 2026-03-03T00:00:00Z
updated: 2026-03-03T17:10:00Z
---

## Current Focus

hypothesis: ROOT CAUSE CONFIRMED (revised). The adapter.ort model has a hard 4096-frame positional embedding limit. The error occurs when: (1) adapter receives T > 4096 frames in a single call, OR (2) pos_offset >= 4096 (pos_offset wraps beyond the embedding table). The "16 by 80" error from the previous sliding-window fix was caused by pos_offset=4080 with T=80: only 16 slots remain in the table. The fix is: revert encode() to match C++ reference (single encoder call), cap accumulated features at 4096 in generate(), and add VAD chunking in benchmark for streaming models.
test: Build and run benchmark --model streaming against all clips
expecting: All 60s clips pass — VAD chunking splits them into <82s segments, each processed within the 4096-frame limit
next_action: Build and verify

## Symptoms

expected: All audio clips (5s, 30s, 60s) transcribe successfully through the moonshine-streaming-tiny/small/medium models
actual: 5s and 30s clips work fine. All 60s clips fail with ORT error in the encoder's Add node (node_add_10)
errors: |
  ERROR during inference run 1 seg 0: ORT error: Non-zero status code returned while running Add node.
  Name:'node_add_10' Status Message: BroadcastIterator::Append axis == 1 || axis == largest was false.
  Attempting to broadcast an axis by a dimension other than 1. 4096 by 4182

  - 60s clip (83.6s decoded): 4096 vs 4182
  - 60s-b clip (90.2s decoded): 4096 vs 4510
  - 60s-c clip (98.0s decoded): 4096 vs 4900
reproduction: Run `cargo run --bin benchmark --features whisper,parakeet,bench_extra --release` from src-tauri/. Streaming models fail on all 60s clips.
started: First time running v2 streaming models. Just added to benchmark.

## Eliminated

- hypothesis: Bug in frontend accumulation (partial buffer, wrong frame count)
  evidence: 5s and 30s clips work correctly; accumulated_feature_count is correct. The issue is not in the frontend step.
  timestamp: 2026-03-03T00:03:00Z

- hypothesis: Encoder ONNX has 4096-frame limit (original theory)
  evidence: Direct Python test of encoder.ort — accepts T=4097, T=4182, any size. Dynamic shape s65 in inputs. The encoder has no size limit.
  timestamp: 2026-03-03T16:45:00Z

- hypothesis: Sliding-window encode() loop fixes the problem
  evidence: Sliding-window gave "16 by 80" error. Traced to: loop emits 80 frames/iteration, adapter_pos_offset increments by 80. After 51 iterations (pos=4080), next call with T=80 needs positions [4080..4160) but table only has [0..4096). The loop was wrong — not because the idea was wrong but because pos_offset >= 4096 fails the adapter entirely.
  timestamp: 2026-03-03T16:47:00Z

## Evidence

- timestamp: 2026-03-03T00:01:00Z
  checked: streaming_config.json for all three models
  found: frame_len=80, total_lookahead=16, depth=6(tiny)/10(small)/14(medium). No left_context field; code computes it as 16*depth.
  implication: The encoder is designed to process frame_len=80 new frames per call, not all accumulated frames.

- timestamp: 2026-03-03T00:02:00Z
  checked: encode() method in streaming_model.rs lines 243-357
  found: For is_final=true path: stable_count=total_features (e.g. 4182), window_start=(0-96).max(0)=0, window_size=4182-0=4182. Feeds ALL 4182 frames to encoder in one call.
  implication: This violates the encoder's max input size of 4096. For audio <= ~81.9 seconds (decoded), it works. For longer audio, it overflows.

- timestamp: 2026-03-03T00:02:30Z
  checked: Error numbers across all 60s clips
  found: 4182/83.6=50.02fps, 4510/90.2=50.0fps, 4900/98.0=50.0fps. Consistent 50fps rate. 4096/50=81.92s is the exact overflow threshold.
  implication: The ONNX encoder model has a fixed internal tensor (attention bias or mask) of exactly 4096 frames. This is the Add node that fails.

- timestamp: 2026-03-03T00:03:30Z
  checked: frame_len=80 in config, left_context_frames=16*depth=96 (tiny)
  found: Proper window per encoder call = left_context(96) + frame_len(80) + lookahead(16) = 192 frames, well within 4096.
  implication: The encoder must be called in a loop with frame_len new frames at a time. The current single-call approach only works for short audio.

- timestamp: 2026-03-03T00:04:00Z
  checked: Moonshine v2 paper / HuggingFace docs
  found: "sliding-window self-attention with no positional embeddings (ergodic encoder)" and "context adapter adds learned positional embeddings". The Add node failure is in the adapter or a sliding window bias tensor baked into the encoder ONNX.
  implication: Confirms encoder must receive a bounded window of frames (max ~4096), not unbounded accumulated features.

- timestamp: 2026-03-03T00:10:00Z
  checked: git log for streaming_model.rs
  found: Commit 521e3f0 (original) had broken single-call encode(). Current streaming_model.rs (after 6f0d7a7) has the sliding-window loop (lines 270-367). The fix loops in frame_len=80 steps with left_context prepended; per-chunk window size is at most left_context(96)+80+lookahead(16)=192 frames, well below 4096.
  implication: Fix is already committed. Benchmark verification is pending.

- timestamp: 2026-03-03T16:42:00Z
  checked: encoder.ort via Python onnxruntime — direct inference tests with T=80..4182
  found: Encoder input shape is [1, 's65', 320] (dynamic). Accepts any T including 4097, 4182. No size limit.
  implication: The original "4096 by N" error was NOT from the encoder. Must be from the adapter.

- timestamp: 2026-03-03T16:43:00Z
  checked: adapter.ort via Python onnxruntime — direct inference tests with T=16..4182 and pos_offset=0..8000
  found: (1) T > 4096 fails with "4096 by T" regardless of pos_offset. (2) pos_offset >= 4096 fails for any T ("invalid 0 by 0" or "T is invalid"). The adapter has a fixed 4096-entry positional embedding table.
  implication: The adapter is the bottleneck. Max frames per call: T <= 4096 AND pos_offset < 4096. Total audio limit: ~4096 frames = 81.9 seconds at 50fps.

- timestamp: 2026-03-03T16:44:00Z
  checked: C++ reference moonshine-ai/moonshine core/moonshine-streaming-model.cpp encode()
  found: C++ encode() does NOT loop. Single encoder call with window=[encoder_frames_emitted-left_context, total_features). Single adapter call with new_frames, pos_offset. No chunking logic. C++ reference has the same 4096-frame limit — it too would fail on >81.9s audio in batch mode.
  implication: Our sliding-window loop was wrong in design. The correct fix matches C++ exactly for the encode() function.

- timestamp: 2026-03-03T16:50:00Z
  checked: benchmark.rs streaming model sections
  found: All three streaming model sections had "Streaming engine handles long audio natively — no VAD chunking needed" with chunks=[audio] (no splitting). Regular moonshine-tiny already uses vad_chunk_audio() for >30s clips.
  implication: Fix requires VAD chunking in benchmark for streaming models, AND truncation guard in generate() as defensive measure.

## Resolution

root_cause: The adapter.ort model has a fixed positional embedding table of exactly 4096 positions. (1) Calling adapter with T > 4096 fails with "4096 by T" broadcast error. (2) Calling adapter with pos_offset >= 4096 fails entirely. The "60s" benchmark clips decode to 83-98 seconds at 50fps = 4182-4900 frames, exceeding the 4096-frame adapter limit. The original encode() passed all frames to the adapter in one call (error "4096 by 4182"). The sliding-window "fix" iterated with pos_offset incrementing by 80 per chunk — hit pos_offset=4080 on chunk 51, so the next chunk (T=80) needs positions [4080..4160) but only 16 are available (error "16 by 80").

fix:
  1. streaming_model.rs encode(): Reverted to match C++ reference (single encoder call per encode() invocation, single adapter call with new_frames and current pos_offset). No loop.
  2. streaming_model.rs generate(): Added ADAPTER_MAX_FRAMES=4096 truncation guard — if accumulated_feature_count > 4096, truncate to 4096 before encode. Logs a warning when truncation occurs.
  3. benchmark.rs: Added VAD chunking for all three streaming model sections (tiny/small/medium). The adapter's 4096-frame limit means segments > ~81.9s must be split. Uses the same vad_chunk_audio() already used for non-streaming moonshine models (splits at silence boundaries, MAX_SEGMENT_SAMPLES=30s per segment).

verification: Benchmark ran --model streaming against all 9 clips (5s/30s/60s x3 variants). Zero errors. All 60s clips completed successfully. Results:
  - tiny CUDA: 6.3-8.5s latency, WER 0-14% on 60s clips (VAD segments ~4-6s each)
  - small CUDA: 11.7-15.5s latency, WER 0.9-12.6% on 60s clips
  - medium CUDA: 16.5-22s latency, WER 0.9-11.7% on 60s clips
  No ERROR lines in full benchmark output.

files_changed:
  - src-tauri/patches/transcribe-rs/src/engines/moonshine/streaming_model.rs
  - src-tauri/src/bin/benchmark.rs
