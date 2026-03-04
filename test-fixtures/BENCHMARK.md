# Model Benchmark

Measures transcription latency and Word Error Rate (WER) for each downloaded model using three test clips (5s, 30s, 60s). Each model is run 5 times per clip to produce avg/min/max timing.

## Prerequisites

- Models downloaded via the VoiceType app (stored in `%APPDATA%\VoiceType\models\`)
- Rust toolchain installed

## Steps

### 1. Generate test WAV files (one-time)

From the project root:

```powershell
powershell -ExecutionPolicy Bypass -File test-fixtures/generate-benchmark-wavs.ps1
```

This creates three 16kHz/16-bit/mono WAV files using Windows TTS:
- `test-fixtures/benchmark-5s.wav`
- `test-fixtures/benchmark-30s.wav`
- `test-fixtures/benchmark-60s.wav`

### 2. Run the benchmark

**Whisper + Parakeet only (default):**

```powershell
cd src-tauri
cargo run --bin benchmark --features whisper,parakeet --release
```

**With Moonshine + SenseVoice (extended):**

```powershell
cd src-tauri
cargo run --bin benchmark --features whisper,parakeet,bench_extra --release
```

> First build takes 10-30 minutes (CUDA compilation). The `bench_extra` feature adds transcribe-rs for Moonshine and SenseVoice inference.

**Filter by model (substring match):**

```powershell
# Only streaming models
cargo run --bin benchmark --features whisper,parakeet,bench_extra --release -- --model streaming

# Only a specific model
cargo run --bin benchmark --features whisper,parakeet,bench_extra --release -- -m moonshine-streaming-tiny

# Multiple filters
cargo run --bin benchmark --features whisper,parakeet,bench_extra --release -- -m streaming -m sensevoice
```

> Without `--model`, all models run. Filters use substring matching.

### 3. Read results

The benchmark prints per-clip results and a ranked summary table:

```
Model                          | Clip | Avg (ms) | Min (ms) | Max (ms) | WER %
-------------------------------+------+----------+----------+----------+------
whisper-small-en               | 5s   |      502 |      489 |      521 |  12.0%
whisper-large-v3-turbo         | 5s   |      120 |      115 |      130 |   7.8%
moonshine-tiny                 | 5s   |       50 |       45 |       55 |    ...
...
```

Models that aren't downloaded are skipped automatically.

## Models

### Default (`--features whisper,parakeet`)

| Model | File/Directory | Source |
|---|---|---|
| whisper-small-en | `ggml-small.en-q5_1.bin` | Built-in (VoiceType download) |
| whisper-medium-en | `ggml-medium.en-q5_0.bin` | Built-in (VoiceType download) |
| whisper-large-v3-turbo | `ggml-large-v3-turbo-q5_0.bin` | Built-in (VoiceType download) |
| whisper-distil-large-v3.5 | `ggml-distil-large-v3.5.bin` | Built-in (VoiceType download) |
| parakeet-tdt-v2 | `parakeet-tdt-v2-fp32/` | Built-in (VoiceType download) |

### Extended (`--features bench_extra`)

| Model | Directory | Size | Source |
|---|---|---|---|
| moonshine-tiny | `moonshine-tiny-ONNX/` | ~108 MB | [onnx-community/moonshine-tiny-ONNX](https://huggingface.co/onnx-community/moonshine-tiny-ONNX) |
| moonshine-base | `moonshine-base-ONNX/` | ~240 MB | [onnx-community/moonshine-base-ONNX](https://huggingface.co/onnx-community/moonshine-base-ONNX) |
| sensevoice-small | `sensevoice-small/` | ~229 MB | [sherpa-onnx SenseVoice](https://huggingface.co/csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17) |
| moonshine-streaming-tiny   | `moonshine-streaming-tiny/`   | ~? MB | [usefulsensors/moonshine](https://huggingface.co/usefulsensors/moonshine) streaming ONNX |
| moonshine-streaming-small  | `moonshine-streaming-small/`  | ~? MB | [usefulsensors/moonshine](https://huggingface.co/usefulsensors/moonshine) streaming ONNX |
| moonshine-streaming-medium | `moonshine-streaming-medium/` | ~? MB | [usefulsensors/moonshine](https://huggingface.co/usefulsensors/moonshine) streaming ONNX |

> Moonshine streaming models use the 5-session streaming ONNX pipeline (frontend, encoder, adapter, cross_kv, decoder_kv). The adapter has a 4096-frame positional limit (~82s at 50fps), so the benchmark applies VAD chunking for clips >30s.

All models are stored in `%APPDATA%\VoiceType\models\`.
