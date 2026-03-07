# Changelog

All notable changes to VoiceType will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.3.0] - 2026-03-07

### Changed

- Clipboard save/restore removed from inject_text — transcription text stays on clipboard after paste, matching standard dictation tool behavior (Dragon, Superwhisper, OpenWhispr)

### Removed

- Pre-paste clipboard save and post-paste clipboard restore logic
- 80ms post-paste sleep (only existed for restore timing)

## [1.2.0] - 2026-03-07

### Added

- WH_KEYBOARD_LL keyboard hook module on dedicated thread for modifier-only hotkeys
- Ctrl+Win modifier-only hotkey with 50ms debounce and press-order independence
- Start menu suppression when Ctrl+Win combo is active
- Fallback to standard RegisterHotKey if hook installation fails
- Frontend modifier-only combo capture with progressive key display
- Moonshine Tiny ONNX engine as third transcription backend
- Engine-agnostic VAD chunking for 60s+ recordings across all engines
- Data-driven model selection with benchmark stats and descriptive labels
- Transcription history panel with click-to-copy and live refresh
- CUDA DLLs bundled in single NSIS installer with runtime GPU fallback
- Filler word removal toggle in Output settings
- Always-listen mode with persistent mic stream
- Pill drag reposition with move mode, green glow, and multi-monitor support
- Inline correction dictionary editor with auto-promote from history
- System tab with inference status display
- Standalone benchmark binary with WER scoring and model rankings
- UI redesign: emerald design system, card-based layouts, page transitions, Inter font

### Changed

- Vocabulary system simplified from multi-profile to single vocabulary section
- Model selection revamped with Parakeet as universal recommendation
- Settings window increased to 800x650
- Pill UI redesigned with simplified CSS and subtle glow

### Fixed

- Clipboard verify-and-retry loop for Outlook paste races
- Stuck Win key and paste failure in Ctrl+Win hotkey
- Startup race condition in hook-status event emission
- Moonshine download URLs pointing to correct onnx/ subdir
- Default hotkey display synced with actual registered shortcut
- Settings store consolidated on single Mutex (removed tauri-plugin-store)
- Rust-side download cancellation and file size validation
- Unsafe Sync on AudioCapture replaced with scoped SendStream newtype

### Removed

- distil-large-v3.5 model (replaced by Moonshine Tiny for low-resource use)
- Multi-profile vocabulary system (simplified to single vocabulary)

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

[Unreleased]: https://github.com/kkosiak592/voicetype/compare/v1.3.0...HEAD
[1.3.0]: https://github.com/kkosiak592/voicetype/compare/v1.2.0...v1.3.0
[1.2.0]: https://github.com/kkosiak592/voicetype/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/kkosiak592/voicetype/releases/tag/v1.1.0
