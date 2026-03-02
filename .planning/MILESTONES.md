# Milestones

## v1.0 MVP (Shipped: 2026-03-02)

**Phases completed:** 10 phases, 26 plans, 16 quick tasks
**Commits:** 237
**Lines of code:** 9,474 (Rust + TypeScript + CSS)
**Timeline:** 4 days (2026-02-27 → 2026-03-02)

**Delivered:** Full local voice-to-text desktop tool with dual transcription engines, glassmorphism pill overlay, vocabulary profiles, and Windows installer.

**Key accomplishments:**
- Tauri 2.0 app with global hotkey, system tray, and settings persistence
- Dual transcription engines: Whisper (whisper-rs/CUDA) and Parakeet TDT (ONNX/CUDA/DirectML)
- End-to-end dictation pipeline: hold-to-talk and toggle mode with Silero VAD
- Glassmorphism pill overlay with sinusoidal frequency bars, animated state transitions, no-focus-steal
- Vocabulary profiles with structural engineering domain support, word corrections, ALL CAPS mode
- First-run setup with GPU auto-detection, model download with progress, NSIS installer

---

