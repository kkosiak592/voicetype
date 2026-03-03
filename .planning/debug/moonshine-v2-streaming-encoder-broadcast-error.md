---
status: fixing
trigger: "Moonshine v2 streaming model fails on 60s audio clips with ORT broadcast dimension mismatch in encoder Add node"
created: 2026-03-03T00:00:00Z
updated: 2026-03-03T00:05:00Z
---

## Current Focus

hypothesis: encode() feeds all accumulated frames (~4182 for 83.6s audio) to the ONNX encoder in a single call. The encoder ONNX has an internal hard limit of 4096 frames (positional attention bias or mask table). When total frames > 4096, the Add node (node_add_10) fails with a broadcast error because it tries to add a [4096]-sized tensor to a [4182]-sized tensor.
test: Confirmed by math: 4182 / 83.6s = 50.02 fps, 4096 / 50 fps = 81.9s max duration before overflow. All failing clips are 83-98 seconds decoded (labeled "60s" but longer).
expecting: Fix by changing encode() to loop in frame_len (80) steps, feeding left_context + 80 + lookahead frames per encoder call.
next_action: Implement the fix in encode() in streaming_model.rs

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

## Resolution

root_cause: encode() method passes ALL accumulated feature frames to the ONNX encoder in a single call. The encoder ONNX model has an internal fixed-size tensor (attention bias/mask at node_add_10) of 4096 frames. Audio decoded to >81.9 seconds (50fps * 4096) causes a broadcast mismatch. The "60s" test clips actually decode to 83-98 seconds, crossing this threshold.

fix: Refactor encode() to loop through accumulated features in frame_len (80) steps. Each iteration passes a window of [left_context + frame_len + remaining_lookahead] frames to the encoder. This matches the streaming encoder's intended usage pattern.

verification: pending

files_changed:
  - src-tauri/patches/transcribe-rs/src/engines/moonshine/streaming_model.rs
