# VoiceType

## What This Is

A local, low-latency voice-to-text desktop tool for Windows that runs entirely on-device using whisper.cpp. Built with Tauri 2.0, it provides a BridgeVoice-style UX — floating pill overlay with audio visualizer, global hotkey activation, and instant text injection into any active application. Designed for personal use and sharing with friends across mixed hardware (NVIDIA GPU and CPU-only machines).

## Core Value

Voice dictation must feel instant — sub-500ms from end-of-speech to text appearing at the cursor, with zero internet dependency.

## Requirements

### Validated

<!-- Shipped and confirmed valuable. -->

(None yet — ship to validate)

### Active

<!-- Current scope. Building toward these. -->

- [ ] Local-only transcription using whisper.cpp (large-v3-turbo on GPU, small on CPU)
- [ ] Hold-to-talk mode (hold hotkey while speaking, release to transcribe)
- [ ] Toggle mode (tap to start recording, tap again to stop)
- [ ] Floating pill overlay with audio visualizer (frequency bars) showing recording state
- [ ] Instant text injection via clipboard paste into any active application
- [ ] Configurable global hotkey for activation (user picks key/combo in settings)
- [ ] Silero VAD for speech endpoint detection in toggle mode
- [ ] Vocabulary profiles — switchable bundles of whisper initial prompt + post-processing dictionary
- [ ] Structural engineering profile (I-beam, W-section, MPa, rebar, AISC terms, etc.)
- [ ] General-purpose profile (default, no domain bias)
- [ ] Caps lock output mode for engineering drawings and PDF markup annotations
- [ ] Post-processing word correction dictionary (find-and-replace after transcription)
- [ ] Settings panel for configuring hotkeys, profiles, corrections, formatting
- [ ] System tray presence with context menu
- [ ] GPU inference with CUDA 11.7 (P2000) with automatic CPU fallback for non-NVIDIA machines
- [ ] Model download on first run (not bundled in installer)
- [ ] Single Windows installer (NSIS) for easy distribution

### Out of Scope

<!-- Explicit boundaries. Includes reasoning to prevent re-adding. -->

- Streaming/real-time partial transcription — chunk-based is sufficient for v1, streaming adds complexity
- Cloud/API-based transcription — core premise is local-only
- macOS/Linux support — Windows-first, cross-platform potential exists with Tauri but not a v1 goal
- LLM-based text cleanup (like Wispr Flow) — raw whisper output + dictionary corrections is sufficient
- Preview/confirmation step before paste — instant injection is the desired workflow
- Mobile app — desktop-only tool

## Context

**Technical environment:**
- Windows 10 Pro, NVIDIA Quadro P2000 (5GB VRAM, Pascal arch, CUDA 11.7)
- Target apps: VS Code, terminals, Chrome/Edge, Outlook/Word/Excel/Teams, AutoCAD, Revit, Bluebeam
- Distribution: personal use + friends/colleagues with mixed hardware (some NVIDIA, some CPU-only)

**Prior research:**
- Detailed technical research completed — see `artifacts/research/` for full analysis
- BridgeVoice validates the Tauri 2.0 + whisper.cpp + Silero VAD stack in production
- whisper.cpp chosen over faster-whisper due to CUDA 11.7 compatibility (faster-whisper requires CUDA 12)
- Multiple reference projects exist: Keyless, Handy, Voquill, Whispering (all Tauri-based)

**Reference apps:**
- BridgeVoice — primary UX reference (speed, pill UI, shortcut activation, hold-to-talk)
- Wispr Flow — settings/features reference (though cloud-based, different architecture)

**Domain context:**
- Structural engineering workflow — dictating into drawing annotations (all caps), PDF markups, emails, code
- Engineering terminology needs special handling (I-beam, W-section, rebar, prestressed, MPa, kips, AISC, ACI 318)

## Constraints

- **GPU**: Must work on CUDA 11.7 (P2000) — eliminates faster-whisper, mandates whisper.cpp
- **CPU Fallback**: Must run on laptops without NVIDIA GPU using smaller models
- **Installer Size**: App installer should be small (~2.5 MB); models downloaded separately on first run
- **Privacy**: Zero telemetry, zero cloud calls, fully offline capable
- **Tech Stack**: Tauri 2.0 (Rust backend + React frontend) — validated by BridgeVoice and multiple reference projects

## Key Decisions

<!-- Decisions that constrain future work. Add throughout project lifecycle. -->

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Tauri 2.0 over Electron/Python | Smallest binary, native Rust FFI to whisper.cpp, BridgeVoice-proven, 20-40MB RAM vs 800MB | — Pending |
| whisper.cpp over faster-whisper | CUDA 11.7 compatible (faster-whisper needs CUDA 12), Rust bindings via whisper-rs | — Pending |
| Clipboard paste as primary text injection | BridgeVoice-proven, fast regardless of text length, works in 95% of apps | — Pending |
| Profiles system for vocabulary | Combines whisper initial prompt + post-processing dictionary per domain, cleanly extensible | — Pending |
| Chunk-based over streaming transcription | Simpler architecture, sub-500ms latency achievable, streaming deferred | — Pending |
| React for frontend | Widely known, good Tauri ecosystem support, Tailwind CSS for styling | — Pending |

---
*Last updated: 2026-02-27 after initialization*
