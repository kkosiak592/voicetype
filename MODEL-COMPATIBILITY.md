# Model Hardware Compatibility

| Model | CPU | NVIDIA (CUDA) | AMD/Intel GPU (DirectML) |
|---|---|---|---|
| whisper-small-en | Yes | Yes | No |
| whisper-medium-en | Yes | Yes | No |
| whisper-large-v3-turbo | Yes | Yes | No |
| whisper-distil-large-v3.5 | Yes | Yes | No |
| parakeet-tdt-v2 | Yes | Yes | Yes |
| moonshine-tiny | Yes | Yes | Yes |
| moonshine-base | Yes | Yes | Yes |
| moonshine-streaming-tiny | Yes | Yes | Yes |
| moonshine-streaming-small | Yes | Yes | Yes |
| moonshine-streaming-medium | Yes | Yes | Yes |
| sensevoice-small | Yes | Yes | Yes |

Whisper uses whisper-rs (whisper.cpp) which only exposes a GPU on/off flag tied to CUDA. The ORT-based models (parakeet, moonshine, sensevoice) support pluggable execution providers including DirectML for AMD/Intel GPUs on Windows.
