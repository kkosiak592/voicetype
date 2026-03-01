# Voice-to-Text Desktop Tool - Brain Dump

## Vision

Local, low-latency voice-to-text desktop tool for Windows that mimics **BridgeVoice** and **Wispr Flow** functionality. Not for public distribution — personal use and sharing with friends/other machines.

## Core Requirements

### 1. Low Latency (Top Priority)
- Must be fast — comparable to BridgeVoice speed
- Local model only, no cloud/API calls
- Model downloaded and running entirely on-device

### 2. Local/Offline Model
- Open-source speech-to-text model (e.g., Whisper or similar)
- No internet dependency for transcription
- Model bundled or downloadable as part of setup

### 3. UI — Pill Icon Overlay
- Small floating pill/indicator appears on screen when activated via keyboard shortcut
- Command shortcut triggers recording mode
- Mimics the BridgeVoice/Wispr Flow interaction pattern

### 4. Custom Word Correction / Domain Vocabulary
- Structural engineering terminology support
- Ability to add custom correction rules (e.g., "if model outputs X, replace with Y")
- Custom vocabulary/prompt tuning for domain-specific terms
- Possibly a user-editable dictionary or correction list

### 5. Settings Panel
- Dedicated settings UI for configuring the app
- Extensible — will be adding features over time
- Settings to include at minimum:
  - Custom word corrections / dictionary
  - Keyboard shortcut configuration
  - Output formatting options

### 6. Output Formatting Options
- Caps lock mode — all output text in uppercase
- Other formatting features TBD

### 7. Packaging & Distribution
- Eventually package everything into a single installer/bundle
- Easy to send to someone and have them install it
- Windows-focused initially

## Reference Apps
- **BridgeVoice** — primary UX reference (speed, pill UI, shortcut activation)
- **Wispr Flow** — settings/features reference

## Open Questions
- Which local model? (Whisper, faster-whisper, whisper.cpp, etc.)
- Tech stack for the desktop app? (Electron, Tauri, native, etc.)
- How to handle real-time streaming vs. chunk-based transcription?
- GPU acceleration requirements/support?
- Exact keyboard shortcut for activation?
- How to inject text into the active application? (clipboard paste, simulated keystrokes, etc.)
