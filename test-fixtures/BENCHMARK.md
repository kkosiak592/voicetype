# Model Benchmark

Measures transcription latency for each downloaded model using two test clips (5s and 60s). Each model is run 5 times per clip to produce avg/min/max timing.

## Prerequisites

- Models downloaded via the VoiceType app (stored in `%APPDATA%\VoiceType\models\`)
- Rust toolchain installed

## Steps

### 1. Generate test WAV files (one-time)

From the project root:

```powershell
powershell -ExecutionPolicy Bypass -File test-fixtures/generate-benchmark-wavs.ps1
```

This creates two 16kHz/16-bit/mono WAV files using Windows TTS:
- `test-fixtures/benchmark-5s.wav`
- `test-fixtures/benchmark-60s.wav`

### 2. Run the benchmark

```powershell
cd src-tauri
cargo run --bin benchmark --features whisper,parakeet --release
```

> First build takes 10-30 minutes (CUDA compilation). Subsequent runs are fast.

### 3. Read results

The benchmark prints an ASCII table to the terminal:

```
Model                     | Clip        | Avg (ms) | Min (ms) | Max (ms)
--------------------------+-------------+----------+----------+---------
whisper-small-en          | 5s          |      502 |      489 |      521
whisper-small-en          | 60s         |     2815 |     2790 |     2856
whisper-distil-large-v3.5 | 5s          |     1324 |     1301 |     1350
...
```

Models that aren't downloaded are skipped automatically.
