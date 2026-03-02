# Changelog

All notable changes to VoiceType will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.1.0] - 2026-03-02

### Added

- Local voice-to-text transcription with zero internet dependency
- Dual engine support: Whisper (CUDA) and Parakeet TDT (CUDA/DirectML)
- Hold-to-talk and toggle recording modes with global hotkey (Ctrl+Shift+Space default)
- Silero VAD silence detection for automatic stop in toggle mode
- Glassmorphism pill overlay with real-time frequency visualization
- Vocabulary profiles with custom word dictionaries
- Structural engineering domain vocabulary profile
- First-run setup wizard with GPU auto-detection and model download
- System tray with microphone state indicator
- Settings: hotkey rebinding, theme toggle, autostart, model selection
- NSIS installer for Windows distribution
- Auto-update infrastructure (updater plugin, Ed25519 signing)

[Unreleased]: https://github.com/kkosiak592/voicetype/compare/v1.1.0...HEAD
[1.1.0]: https://github.com/kkosiak592/voicetype/releases/tag/v1.1.0
