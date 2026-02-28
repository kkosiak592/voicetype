# Requirements: VoiceType

**Defined:** 2026-02-27
**Core Value:** Voice dictation must feel instant — sub-1500ms from end-of-speech to text appearing at the cursor, with zero internet dependency.

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Core Pipeline

- [x] **CORE-01**: User can activate voice recording via a system-wide global hotkey from any application
- [x] **CORE-02**: App captures microphone audio at 16kHz via cpal/WASAPI
- [x] **CORE-03**: App transcribes audio using whisper.cpp (whisper-rs) with GPU acceleration on CUDA 11.7
- [x] **CORE-04**: App falls back to CPU inference (whisper small model) when no NVIDIA GPU is detected
- [x] **CORE-05**: Transcribed text is injected at the active cursor position via clipboard paste (Ctrl+V)
- [x] **CORE-06**: App saves clipboard contents before injection and restores them after (with timing delays to avoid race conditions)

### Recording Modes

- [x] **REC-01**: User can hold the hotkey to record and release to transcribe (hold-to-talk mode)
- [ ] **REC-02**: User can tap the hotkey to start recording and tap again to stop (toggle mode)
- [ ] **REC-03**: In toggle mode, Silero VAD automatically detects silence and stops recording
- [ ] **REC-04**: User can switch between hold-to-talk and toggle mode in settings

### UI / Overlay

- [ ] **UI-01**: A floating pill-shaped overlay appears on screen during recording (always-on-top, transparent, frameless)
- [ ] **UI-02**: The pill overlay does not steal focus from the active application (Win32 WS_EX_NOACTIVATE)
- [ ] **UI-03**: The pill displays an audio visualizer with frequency bars showing mic input levels
- [ ] **UI-04**: The pill shows recording state (idle/recording/processing)
- [x] **UI-05**: App runs in the system tray with a context menu (Settings, Quit, version info)

### Vocabulary & Corrections

- [ ] **VOC-01**: App applies a user-editable word correction dictionary (JSON find-and-replace) after each transcription
- [ ] **VOC-02**: User can create and switch between vocabulary profiles (each profile bundles a whisper initial_prompt + correction dictionary + output formatting)
- [ ] **VOC-03**: App ships with a pre-configured "Structural Engineering" profile (I-beam, W-section, MPa, rebar, AISC, ACI 318, kips, PSI, prestressed)
- [ ] **VOC-04**: App ships with a "General" profile (no domain bias, default corrections only)
- [ ] **VOC-05**: User can enable ALL CAPS output mode per profile (for engineering drawing annotations and PDF markups)
- [ ] **VOC-06**: Whisper initial_prompt is set per profile to bias the model toward domain-specific vocabulary

### Settings & Configuration

- [ ] **SET-01**: App has a settings panel UI for configuring hotkeys, model, microphone, profiles, and corrections
- [x] **SET-02**: User can configure the global hotkey binding (choose any key or key combo)
- [ ] **SET-03**: User can select which whisper model to use (large-v3-turbo for GPU, small for CPU, medium as alternative)
- [ ] **SET-04**: User can select which microphone to use from available input devices
- [x] **SET-05**: Settings persist across app restarts (tauri-plugin-store)

### Distribution

- [ ] **DIST-01**: On first run, app downloads the selected whisper model with a progress indicator
- [ ] **DIST-02**: App auto-detects GPU capability and recommends appropriate model size
- [ ] **DIST-03**: App is packaged as a single Windows NSIS installer (models downloaded separately, not bundled)

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Enhanced Corrections

- **ECOR-01**: Regex-based post-processing corrections for phonetic mishearings ("why section" → "W-section")
- **ECOR-02**: Hotword support via whisper.cpp --hotwords parameter (experimental)
- **ECOR-03**: Quick-add to dictionary from system tray context menu

### History & Recovery

- **HIST-01**: App logs every transcription with timestamp, word count, and duration
- **HIST-02**: User can view transcription history in settings panel
- **HIST-03**: User can re-inject a previous transcription from history

### Additional Profiles

- **PROF-01**: Additional domain profiles (legal, medical, software development)
- **PROF-02**: Per-app profile auto-switching

### Updates

- **UPDT-01**: Auto-updater via tauri-plugin-updater

## Out of Scope

| Feature | Reason |
|---------|--------|
| Streaming/real-time partial transcription | whisper.cpp not designed for streaming; chunk-based achieves sub-500ms; dramatically increases complexity |
| LLM-based text cleanup | Breaks offline premise; adds 200-800ms latency; can hallucinate and change technical terms |
| Cloud/API transcription fallback | Undermines core privacy value; two code paths to maintain |
| Preview/confirmation before paste | Destroys workflow speed advantage; BridgeVoice deliberately chose instant injection |
| Context-aware app detection (screenshots) | Privacy violation risk; Wispr Flow's most controversial feature |
| Voice commands for editing | Requires separate command recognition layer; dramatically increases complexity |
| macOS/Linux support | Windows-first; Tauri enables cross-platform later but not v1 |
| Mobile app | Desktop-only tool |
| Multi-language UI | Whisper auto-detects language; UI in English only |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| CORE-01 | Phase 1 | Complete |
| CORE-02 | Phase 2 | Complete |
| CORE-03 | Phase 2 | Complete |
| CORE-04 | Phase 2 | Complete |
| CORE-05 | Phase 3 | Complete |
| CORE-06 | Phase 3 | Complete |
| REC-01 | Phase 3 | Complete |
| REC-02 | Phase 5 | Pending |
| REC-03 | Phase 5 | Pending |
| REC-04 | Phase 5 | Pending |
| UI-01 | Phase 4 | Pending |
| UI-02 | Phase 4 | Pending |
| UI-03 | Phase 4 | Pending |
| UI-04 | Phase 4 | Pending |
| UI-05 | Phase 1 | Complete |
| VOC-01 | Phase 6 | Pending |
| VOC-02 | Phase 6 | Pending |
| VOC-03 | Phase 6 | Pending |
| VOC-04 | Phase 6 | Pending |
| VOC-05 | Phase 6 | Pending |
| VOC-06 | Phase 6 | Pending |
| SET-01 | Phase 6 | Pending |
| SET-02 | Phase 1 | Complete |
| SET-03 | Phase 6 | Pending |
| SET-04 | Phase 6 | Pending |
| SET-05 | Phase 1 | Complete |
| DIST-01 | Phase 7 | Pending |
| DIST-02 | Phase 7 | Pending |
| DIST-03 | Phase 7 | Pending |

**Coverage:**
- v1 requirements: 29 total
- Mapped to phases: 29
- Unmapped: 0

---
*Requirements defined: 2026-02-27*
*Last updated: 2026-02-27 after roadmap creation*
