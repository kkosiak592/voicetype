# Phase 2: Audio + Whisper - Context

**Gathered:** 2026-02-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Microphone audio captured at 16kHz and transcribed by whisper.cpp with GPU acceleration confirmed on the development machine. The two highest-risk components (audio capture and whisper inference) verified in isolation before being wired into the full pipeline. Auto-download, vocabulary profiles, and streaming are out of scope.

</domain>

<decisions>
## Implementation Decisions

### Audio capture behavior
- Fall back to system default microphone with a brief notification if configured mic is not found (matches Discord/OBS/Teams pattern)
- On mic disconnect mid-recording, transcribe whatever audio was captured so far (don't discard)
- Target ~10ms audio buffers for low-latency capture
- Always downmix to mono regardless of input device channels (whisper needs mono 16kHz)
- Keep audio stream persistent (always-on mic, just not saving) so recording starts instantly on hotkey press — no device initialization delay

### Model management
- Store models in %APPDATA%/VoiceType/models/ (standard Windows app data directory)
- Phase 2 uses manual model placement — developer downloads model files to the expected path
- Prefer English-only model variants (.en) — no multi-language support needed
- Research phase should re-evaluate model choices (large-v3-turbo-q5_0 for GPU, small for CPU) to see if better English-only alternatives exist
- User has prior research in the artifacts/ folder that may inform model selection

### Transcription tuning
- Force language='en' — English-only mode, no auto-detection
- No initial_prompt for Phase 2 (vocabulary tuning is Phase 6)
- Hardcode whisper defaults (beam_size=5, temperature=0) — no exposed tuning parameters
- Simple batch API: record all audio, then transcribe as one batch after recording stops
- Speed comes from persistent audio stream + fast GPU inference, not streaming

### Verification & logging
- Log CUDA initialization success/failure at startup
- GPU verification: manual Task Manager check during test runs (per success criteria)
- Keep 2-3 reference WAV recordings in a test fixtures directory for regression testing
- Verbose logging for Phase 2: audio device selection, sample rate, buffer sizes, CUDA init, inference time, model load time
- Dial back logging verbosity in later phases

### Claude's Discretion
- Persistent audio stream implementation details
- Exact resampling configuration (rubato parameters)
- Error message content when model file is missing (must include download instructions and expected path)
- Test WAV file naming and directory structure
- Exact whisper-rs parameter values beyond beam_size and temperature

</decisions>

<specifics>
## Specific Ideas

- BridgeVoice (Tauri 2.0 + Rust + local whisper) achieves <1s end-to-end by keeping audio stream persistent and using batch transcription — use this as the reference architecture
- User wants to minimize transcription latency above all else
- Model setup guidance needed before Phase 2 execution (document manual download steps as part of the plan)

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 02-audio-whisper*
*Context gathered: 2026-02-27*
