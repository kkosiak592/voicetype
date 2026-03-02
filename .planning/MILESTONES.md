# Milestones

## v1.1 Auto-Updates & CI/CD (Shipped: 2026-03-02)

**Phases completed:** 4 phases, 5 plans, 12 tasks
**Commits:** 31
**Lines changed:** +7,107 / -1,109
**Timeline:** 1 day (2026-03-02)
**Git range:** feat(11-01)..release: v1.1.0

**Delivered:** Complete auto-update pipeline — Ed25519 signing, in-app update UX with progress, GitHub Actions CI/CD, and documented release workflow.

**Key accomplishments:**
- Ed25519 signing keypair with public key in tauri.conf.json and private key in GitHub Actions secrets
- Public GitHub repo kkosiak592/voicetype with full source code
- tauri-plugin-updater + tauri-plugin-process with check/download/install/relaunch lifecycle
- UpdateBanner component with download progress, release notes, tray indicator, and periodic 4-hour checks
- GitHub Actions CI/CD: v* tag push triggers CUDA+LLVM build, Ed25519 signing, and GitHub Release publishing
- RELEASING.md runbook + CHANGELOG.md template for repeatable release process

---

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

