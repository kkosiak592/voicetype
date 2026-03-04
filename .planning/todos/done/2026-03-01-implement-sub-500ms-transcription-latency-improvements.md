---
created: 2026-03-01T15:18:26.790Z
title: Implement sub-500ms transcription latency improvements
area: backend
files:
  - artifacts/research/2026-03-01-sub-500ms-transcription-latency-technical.md
  - src-tauri/src/transcribe.rs
  - src-tauri/src/pipeline.rs
  - src-tauri/src/inject.rs
  - src-tauri/src/lib.rs
  - src-tauri/Cargo.toml
---

## Problem

Current transcription latency is ~1-2s after releasing the shortcut key on a Quadro P2000 GPU. The dominant bottleneck is Whisper's autoregressive decoder (large-v3-turbo via whisper-rs), which takes ~800-1500ms for a 5s clip on this hardware. Target is <500ms end-to-end from key release to text injection.

## Solution

Two-pronged approach from research (see artifacts/research/2026-03-01-sub-500ms-transcription-latency-technical.md):

**Primary: Switch to NVIDIA Parakeet TDT via parakeet-rs**
- Add `parakeet-rs = { version = "0.3", features = ["cuda"] }` to Cargo.toml
- Create transcribe_parakeet.rs (Parakeet inference wrapper)
- Non-autoregressive architecture: ~100-400ms on P2000 vs ~800-1500ms Whisper
- Download parakeet-tdt-0.6b-v2 ONNX model (~600MB)
- No initial_prompt equivalent — corrections engine must compensate for vocabulary biasing

**Complementary: Pipeline micro-optimizations (~50-85ms saved)**
- Skip post-hoc VAD gate → simple length check (save ~20-30ms)
- Reduce injection sleeps: 30ms→15ms clipboard, 50ms→25ms paste (save ~35ms)
- Reuse WhisperState / pre-warm clipboard (save ~10-20ms)

**Fallback: distil-large-v3 model swap**
- One-line change in resolve_model_path() if Parakeet integration is problematic
- ~1.5-2x faster than turbo, same whisper-rs API

Implementation order: instrument current pipeline timing → micro-optimizations → parakeet-rs integration → benchmark A/B comparison.
