# VoiceType

## What This Is

A local, low-latency voice-to-text desktop tool for Windows with dual transcription engines (Whisper + Parakeet TDT). Built with Tauri 2.0, it provides a glassmorphism pill overlay with audio visualizer, global hotkey activation, Silero VAD silence detection, vocabulary profiles for domain-specific dictation, and instant text injection into any active application. Runs entirely on-device with zero internet dependency.

## Core Value

Voice dictation must feel instant — sub-1500ms from end-of-speech to text appearing at the cursor, with zero internet dependency.

## Requirements

### Validated

- ✓ Local-only transcription using whisper.cpp (large-v3-turbo on GPU, small on CPU) — v1.0
- ✓ Parakeet TDT as second GPU engine with CUDA + DirectML support — v1.0
- ✓ Hold-to-talk mode (hold hotkey while speaking, release to transcribe) — v1.0
- ✓ Toggle mode (tap to start recording, tap again to stop) — v1.0
- ✓ Floating pill overlay with glassmorphism, sinusoidal frequency bars, animated state transitions — v1.0
- ✓ Instant text injection via clipboard paste into any active application — v1.0
- ✓ Configurable global hotkey for activation — v1.0
- ✓ Silero VAD for speech endpoint detection in toggle mode — v1.0
- ✓ Vocabulary profiles with whisper initial prompt + post-processing dictionary — v1.0
- ✓ Structural engineering profile (I-beam, W-section, MPa, rebar, AISC terms) — v1.0
- ✓ General-purpose profile (default, no domain bias) — v1.0
- ✓ ALL CAPS output mode for engineering drawings and PDF markup annotations — v1.0
- ✓ Post-processing word correction dictionary (find-and-replace after transcription) — v1.0
- ✓ Settings panel for configuring hotkeys, profiles, corrections, model, microphone — v1.0
- ✓ System tray with state-colored microphone icon and tooltip — v1.0
- ✓ GPU inference with CUDA 11.7 + automatic CPU fallback — v1.0
- ✓ First-run model download with GPU auto-detection and progress indicator — v1.0
- ✓ Single Windows NSIS installer (~9 MB, models downloaded separately) — v1.0

### Active

(None — planning next milestone)

### Out of Scope

- Streaming/real-time partial transcription — chunk-based achieves acceptable latency
- Cloud/API-based transcription — core premise is local-only
- macOS/Linux support — Windows-first, Tauri enables cross-platform later
- LLM-based text cleanup — raw whisper/parakeet output + dictionary corrections is sufficient
- Preview/confirmation step before paste — instant injection is the desired workflow
- Mobile app — desktop-only tool
- Offline mode for updates — models require initial internet download, all inference is offline

## Context

**Current state (v1.0 shipped 2026-03-02):**
- 9,474 LOC across Rust backend + React/TypeScript frontend
- Tech stack: Tauri 2.0, whisper-rs, parakeet-rs, cpal/WASAPI, Silero VAD, React, Tailwind CSS
- Dual engine: Whisper (CUDA) for broad compatibility, Parakeet TDT (CUDA/DirectML) for GPU users
- 237 commits over 4 days of development
- NSIS installer ~9 MB, models downloaded on first run (300MB-1.3GB depending on selection)

**Technical environment:**
- Windows 10 Pro, NVIDIA Quadro P2000 (5GB VRAM, Pascal arch, CUDA 11.7)
- Target apps: VS Code, terminals, Chrome/Edge, Outlook/Word/Excel/Teams, AutoCAD, Revit, Bluebeam
- Distribution: personal use + friends/colleagues with mixed hardware (NVIDIA, Intel/AMD, CPU-only)

**Domain context:**
- Structural engineering workflow — dictating into drawing annotations (all caps), PDF markups, emails, code
- Engineering terminology needs special handling (I-beam, W-section, rebar, prestressed, MPa, kips, AISC, ACI 318)

## Constraints

- **GPU**: Must work on CUDA 11.7 (P2000) — eliminates faster-whisper, mandates whisper.cpp
- **DirectML**: Parakeet TDT supports non-NVIDIA GPUs via DirectML EP
- **CPU Fallback**: Must run on laptops without any GPU using Whisper small model
- **Installer Size**: ~9 MB (models downloaded separately on first run)
- **Privacy**: Zero telemetry, zero cloud calls, fully offline inference
- **Tech Stack**: Tauri 2.0 (Rust backend + React frontend)

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Tauri 2.0 over Electron/Python | Smallest binary, native Rust FFI to whisper.cpp, BridgeVoice-proven, 20-40MB RAM vs 800MB | ✓ Good — ~9 MB installer, fast startup |
| whisper.cpp over faster-whisper | CUDA 11.7 compatible (faster-whisper needs CUDA 12), Rust bindings via whisper-rs | ✓ Good — works on P2000 |
| Clipboard paste as primary text injection | BridgeVoice-proven, fast regardless of text length, works in 95% of apps | ✓ Good — works across all target apps |
| Profiles system for vocabulary | Combines whisper initial prompt + post-processing dictionary per domain, cleanly extensible | ✓ Good — engineering terms recognized accurately |
| Chunk-based over streaming transcription | Simpler architecture, sub-500ms latency achievable, streaming deferred | ✓ Good — acceptable latency achieved |
| React for frontend | Widely known, good Tauri ecosystem support, Tailwind CSS for styling | ✓ Good — fast development |
| Parakeet TDT as second engine | ONNX-based, supports CUDA + DirectML, faster inference for GPU users | ✓ Good — broader GPU support |
| DirectML for non-NVIDIA GPUs | ort DirectML EP enables Parakeet on Intel/AMD integrated GPUs | ✓ Good — widens hardware support |
| VAD gate bypass for hold-to-talk | Saves 20-30ms by skipping Silero scan when user explicitly controls recording | ✓ Good — noticeable latency reduction |

---
*Last updated: 2026-03-02 after v1.0 milestone*
