# VoiceType

## What This Is

A local, low-latency voice-to-text desktop tool for Windows with three transcription engines (Whisper, Parakeet TDT, Moonshine Tiny). Built with Tauri 2.0, it provides a glassmorphism pill overlay with audio visualizer, WH_KEYBOARD_LL keyboard hook for Ctrl+Win activation, Silero VAD silence detection with engine-agnostic chunking for long recordings, transcription history with click-to-copy, and instant text injection into any active application. Runs entirely on-device with zero internet dependency.

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
- ✓ tauri-plugin-updater with Ed25519 signing and GitHub Releases backend — v1.1
- ✓ GitHub Actions CI/CD for automated builds, signing, and release publishing — v1.1
- ✓ In-app update check on launch with download/install/relaunch UX — v1.1
- ✓ Release workflow with documented runbook and changelog format — v1.1
- ✓ WH_KEYBOARD_LL keyboard hook module on dedicated thread — v1.2
- ✓ Ctrl+Win modifier-only hotkey with 50ms debounce — v1.2
- ✓ Start menu suppression when Ctrl+Win combo is active — v1.2
- ✓ Fallback to standard hotkey if hook installation fails — v1.2
- ✓ Frontend modifier-only combo capture and display — v1.2
- ✓ Moonshine Tiny ONNX engine as third transcription backend — v1.2
- ✓ Engine-agnostic VAD chunking for 60s+ recordings — v1.2
- ✓ Data-driven model selection with benchmark stats — v1.2
- ✓ Transcription history panel with click-to-copy — v1.2
- ✓ CUDA DLLs bundled in single installer with runtime GPU fallback — v1.2
- ✓ Filler word removal, always-listen mode, pill drag reposition — v1.2

- ✓ Simplified clipboard flow: save/restore removed, transcription replaces clipboard content — v1.3

### Active

- [ ] Per-app settings sidebar page with auto-detect foreground window
- [ ] ALL CAPS per-app override (global toggle remains as default for unlisted apps)
- [ ] Add apps via "Detect Active App" button + searchable dropdown of running processes
- [ ] Win32 foreground window detection at injection time (GetForegroundWindow + process name)

## Current Milestone: v1.4 Per-App Settings

**Goal:** Enable per-application setting overrides, starting with ALL CAPS, detected automatically based on the foreground window at injection time.

**Target features:**
- New "App Rules" sidebar page for managing per-app overrides
- Auto-detect foreground application using Win32 APIs at text injection time
- Per-app ALL CAPS toggle that overrides the global default
- "Detect Active App" button + searchable dropdown of running processes for adding apps
- Global ALL CAPS toggle stays on General page as default for unlisted apps

## Current State

v1.3 shipped 2026-03-07. Four milestones complete (v1.0 MVP, v1.1 Auto-Updates, v1.2 Keyboard Hook, v1.3 Clipboard Simplification). v1.4 in progress.

### Out of Scope

- Streaming/real-time partial transcription — chunk-based achieves acceptable latency
- Cloud/API-based transcription — core premise is local-only
- macOS/Linux support — Windows-first, Tauri enables cross-platform later
- LLM-based text cleanup — raw whisper/parakeet output + dictionary corrections is sufficient
- Preview/confirmation step before paste — instant injection is the desired workflow
- Mobile app — desktop-only tool
- Offline mode for updates — models require initial internet download, all inference is offline

## Context

**Current state (v1.3 shipped 2026-03-07):**
- 23,533 LOC across Rust backend + React/TypeScript frontend
- Tech stack: Tauri 2.0, whisper-rs, parakeet-rs, ort (Moonshine ONNX), cpal/WASAPI, Silero VAD, React, Tailwind CSS, tauri-plugin-updater, tauri-plugin-process
- Three engines: Whisper (CUDA), Parakeet TDT (CUDA/DirectML), Moonshine Tiny (ONNX)
- WH_KEYBOARD_LL keyboard hook for Ctrl+Win modifier-only activation
- Engine-agnostic VAD chunking for 60s+ recordings
- 521 commits over 9 days of development
- NSIS installer with bundled CUDA DLLs, models downloaded on first run
- Auto-update pipeline: Ed25519 signing, GitHub Actions CI/CD, in-app update UX
- Public repo: https://github.com/kkosiak592/voicetype

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
| Remove clipboard save/restore | Standard dictation tools leave transcription on clipboard; save/restore adds complexity and 80ms latency for no user benefit | ✓ Good — -24 lines, faster injection |
| Profiles system for vocabulary | Combines whisper initial prompt + post-processing dictionary per domain, cleanly extensible | ✓ Good — engineering terms recognized accurately |
| Chunk-based over streaming transcription | Simpler architecture, sub-500ms latency achievable, streaming deferred | ✓ Good — acceptable latency achieved |
| React for frontend | Widely known, good Tauri ecosystem support, Tailwind CSS for styling | ✓ Good — fast development |
| Parakeet TDT as second engine | ONNX-based, supports CUDA + DirectML, faster inference for GPU users | ✓ Good — broader GPU support |
| DirectML for non-NVIDIA GPUs | ort DirectML EP enables Parakeet on Intel/AMD integrated GPUs | ✓ Good — widens hardware support |
| VAD gate bypass for hold-to-talk | Saves 20-30ms by skipping Silero scan when user explicitly controls recording | ✓ Good — noticeable latency reduction |
| WH_KEYBOARD_LL on dedicated thread | Enables modifier-only combos (Ctrl+Win) impossible with RegisterHotKey; 50ms debounce for press-order independence | ✓ Good — reliable activation |
| Moonshine Tiny as third engine | ONNX-based, smallest model (~70MB), fastest inference for quick dictation | ✓ Good — broadens hardware support |
| Engine-agnostic VAD chunking | Single vad_chunk_audio function handles 60s+ recordings for all engines | ✓ Good — eliminated per-engine chunking duplication |
| CUDA DLLs bundled in installer | Single installer for all users; runtime GPU fallback on non-NVIDIA | ✓ Good — no installer split needed |
| Parakeet as universal recommendation | Benchmark data shows best accuracy/latency balance across hardware | ✓ Good — simplified model selection |

---
| tauri-plugin-updater + GitHub Releases | Zero cost, excellent UX, official Tauri approach, simplest for <20 users | ✓ Good — first CI release published |
| Public GitHub repo | Updater needs unauthenticated access to release assets, source visibility acceptable | ✓ Good |
| Ed25519 signing over RSA | Tauri default, small signatures, fast verification | ✓ Good |
| bundle.createUpdaterArtifacts v1Compatible | Backward-compatible signature format for Tauri 2 | ✓ Good |
| JS plugin API for download (not Rust IPC) | Progress callbacks in frontend without custom channel piping | ✓ Good |
| CUDA minimal sub-packages in CI | Avoids 4 GB full toolkit; installs only nvcc/cudart/cublas needed for whisper-rs | ✓ Good |
| CMAKE_CUDA_ARCHITECTURES=61;75;86;89 | Single binary supports Pascal through Ada Lovelace GPUs | ✓ Good |
| Annotated git tags for releases | Store tagger info, work with git describe, better practice | ✓ Good |

| Per-app settings with auto-detect foreground window | Per-app ALL CAPS is the first use case; extensible for future per-app settings | — Pending |

---
*Last updated: 2026-03-07 after v1.4 milestone started*
