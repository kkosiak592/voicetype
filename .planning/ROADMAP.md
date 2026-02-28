# Roadmap: VoiceType

## Overview

VoiceType is built in strict dependency order: the framework and hotkey system must exist before audio can be wired in, audio must be verified before whisper integration, and the full pipeline must be proven end-to-end before the overlay UI is added on top of it. This bottom-up order prevents UI and injection complexity from masking pipeline timing failures. Vocabulary profiles and settings are layered on after the core loop is proven, and distribution is last — it validates the full product as built.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Foundation** - Tauri scaffold, global hotkey, system tray, and settings persistence (completed 2026-02-27)
- [x] **Phase 2: Audio + Whisper** - Microphone capture pipeline and GPU-verified whisper transcription (completed 2026-02-28)
- [x] **Phase 3: Core Pipeline** - End-to-end hold-to-talk: hotkey to audio to transcription to text injection (completed 2026-02-28)
- [ ] **Phase 4: Pill Overlay** - Floating transparent overlay with visualizer and no-focus-steal guarantee
- [ ] **Phase 5: VAD + Toggle Mode** - Silero VAD silence detection and toggle recording mode
- [ ] **Phase 6: Vocabulary + Settings** - Word corrections, vocabulary profiles, and full settings UI
- [ ] **Phase 7: Distribution** - First-run model download, GPU auto-detection, and NSIS installer

## Phase Details

### Phase 1: Foundation
**Goal**: A working Tauri 2.0 app that registers a system-wide hotkey, shows in the system tray, and persists its configuration — the verified container for all future work
**Depends on**: Nothing (first phase)
**Requirements**: CORE-01, UI-05, SET-02, SET-05
**Success Criteria** (what must be TRUE):
  1. User can press the configured hotkey from any other application and the app responds (prints to console or emits a log event) without the other app losing focus
  2. App icon appears in the system tray with a right-click context menu showing at minimum Settings and Quit
  3. User can configure the global hotkey binding in settings; the new binding takes effect immediately without restarting the app
  4. Settings survive an app restart — hotkey binding and any other persisted config are restored from disk
**Plans**: 3 plans

Plans:
- [ ] 01-01-PLAN.md — Tauri scaffold with system tray, settings window, single-instance, and hide-to-tray
- [ ] 01-02-PLAN.md — Global hotkey registration with tauri-plugin-global-shortcut, verify no focus steal
- [ ] 01-03-PLAN.md — Settings persistence with tauri-plugin-store, hotkey rebinding UI, theme and autostart toggles

### Phase 2: Audio + Whisper
**Goal**: Microphone audio captured at 16kHz and transcribed by whisper.cpp with GPU acceleration confirmed on the development machine — the two highest-risk components verified in isolation before being wired together
**Depends on**: Phase 1
**Requirements**: CORE-02, CORE-03, CORE-04
**Success Criteria** (what must be TRUE):
  1. App captures microphone audio via WASAPI and saves a WAV file that plays back correctly at 16kHz
  2. App transcribes a test WAV file using whisper-rs and prints the result to console in under 1500ms on the NVIDIA P2000 (verified via Task Manager GPU utilization, not assumed)
  3. On a machine with no NVIDIA GPU, the app falls back to CPU inference using the small model and completes transcription (latency is acceptable; GPU speed is not required on CPU machines)
**Plans**: 3 plans

Plans:
- [ ] 02-01-PLAN.md — Persistent audio capture via cpal WASAPI with rubato resampling to 16kHz mono, WAV output for verification
- [ ] 02-02-PLAN.md — Whisper-rs with CUDA 11.7 Pascal build, model loading, GPU inference, Task Manager GPU verification
- [ ] 02-03-PLAN.md — GPU detection via nvml-wrapper, model size selection (large-v3-turbo vs small.en), CPU fallback verification

### Phase 3: Core Pipeline
**Goal**: A working end-to-end hold-to-talk dictation loop — hold hotkey, speak, release, see text appear at the cursor — proven before any UI or polish is added on top
**Depends on**: Phase 2
**Requirements**: CORE-05, CORE-06, REC-01
**Success Criteria** (what must be TRUE):
  1. User holds the hotkey, speaks a sentence, releases the hotkey, and the transcribed text appears in the active application (Notepad, VS Code, browser address bar) within 1500ms of release
  2. The clipboard contents that were present before dictation are fully restored after the transcribed text is pasted — the user's clipboard is not clobbered
  3. The injection works in at minimum Notepad, VS Code, Chrome, and Windows Terminal (the four baseline test targets)
**Plans**: TBD

Plans:
- [ ] 03-01: Text injection — implement clipboard save/restore via Win32 CF_UNICODETEXT, enigo Ctrl+V paste with 50-100ms pre-paste and 100-150ms pre-restore delays, test in all four target apps
- [ ] 03-02: Pipeline integration — wire hotkey hold/release events to audio start/stop to whisper inference to injection, confirm sub-1500ms latency end-to-end on GPU

### Phase 4: Pill Overlay
**Goal**: A floating transparent pill window that shows recording state and audio levels during dictation, without ever stealing focus from the application being dictated into
**Depends on**: Phase 3
**Requirements**: UI-01, UI-02, UI-03, UI-04
**Success Criteria** (what must be TRUE):
  1. A floating pill-shaped overlay appears while recording and disappears when idle — it is always on top of other windows
  2. Opening or dismissing the pill does not steal focus from the previously active application — text injected immediately after dictation lands in the correct app and correct field
  3. The pill displays animated frequency bars that respond to real microphone input levels during recording
  4. The pill displays distinct visual states for recording, processing (whisper running), and idle
**Plans**: TBD

Plans:
- [ ] 04-01: Pill window setup — create frameless transparent always-on-top Tauri window, apply Win32 WS_EX_NOACTIVATE extended style via Rust window builder, verify no focus steal against all four target apps
- [ ] 04-02: Visualizer and state display — wire audio level data via channel to React frequency bar component, implement recording/processing/idle state transitions in Pill.tsx

### Phase 5: VAD + Toggle Mode
**Goal**: Silero VAD silence detection enables toggle mode — tap to start, auto-stop on silence — and gates whisper against hallucination on empty audio buffers
**Depends on**: Phase 4
**Requirements**: REC-02, REC-03, REC-04
**Success Criteria** (what must be TRUE):
  1. User taps the hotkey to start recording, speaks, pauses for ~1.5 seconds, and the app stops recording and transcribes automatically — no second tap required
  2. User can tap the hotkey a second time to stop recording early in toggle mode (manual override of auto-stop)
  3. If the user activates dictation and says nothing, whisper does not run and no text is injected — the silence gate discards the buffer
  4. User can switch between hold-to-talk and toggle mode in settings, and the selected mode persists across restarts
**Plans**: TBD

Plans:
- [ ] 05-01: Silero VAD integration — integrate silero-vad-rust, process 30ms chunks from audio mpsc channel, detect speech start/end with tuned silence threshold, discard buffers with less than 300ms detected speech
- [ ] 05-02: Toggle mode — implement tap-to-start/auto-stop state machine using VAD events, add manual stop on second tap, wire mode selection (hold-to-talk vs toggle) to settings persistence

### Phase 6: Vocabulary + Settings
**Goal**: Word correction dictionary, vocabulary profiles with engineering and general presets, and a full settings panel — the differentiating layer that makes VoiceType accurate for structural engineering work
**Depends on**: Phase 5
**Requirements**: VOC-01, VOC-02, VOC-03, VOC-04, VOC-05, VOC-06, SET-01, SET-03, SET-04
**Success Criteria** (what must be TRUE):
  1. After dictation, common mishearings defined in the user's correction dictionary are replaced before text is injected — the user can add and edit entries in the settings panel
  2. Switching to the Structural Engineering profile causes whisper to recognize engineering terms (I-beam, W-section, MPa, rebar, AISC, ACI 318, kips) more accurately and applies engineering-specific corrections
  3. With ALL CAPS mode enabled on a profile, all injected text is uppercased — enabling engineering drawing annotation and PDF markup workflows
  4. User can select which microphone and which whisper model to use from the settings panel; selections persist across restarts
  5. Settings panel opens from the system tray context menu and provides access to all configurable options: hotkey, profile, model, microphone, and correction dictionary editor
**Plans**: TBD

Plans:
- [ ] 06-01: Corrections engine — implement HashMap-based post-processing dictionary in corrections.rs, load from JSON, apply after each transcription
- [ ] 06-02: Vocabulary profiles — implement profiles.rs with initial_prompt selection, corrections dictionary per profile, ALL CAPS normalization flag; ship Structural Engineering and General profiles
- [ ] 06-03: Settings panel UI — build Settings.tsx Tauri window with hotkey rebind, model selector, microphone selector, profile switcher, and inline corrections dictionary editor

### Phase 7: Distribution
**Goal**: First-run model download with progress UI, GPU auto-detection with model recommendation, and a single NSIS installer — making the app installable on any Windows machine regardless of hardware
**Depends on**: Phase 6
**Requirements**: DIST-01, DIST-02, DIST-03
**Success Criteria** (what must be TRUE):
  1. On a fresh install with no model file present, the app detects the missing model, shows a download progress indicator, downloads the appropriate model file, validates its SHA256 checksum, and starts normally — without any manual steps from the user
  2. On a machine with an NVIDIA GPU, the app auto-detects CUDA capability and recommends large-v3-turbo-q5_0; on a CPU-only machine it recommends small — the recommendation is shown before the download begins
  3. The NSIS installer is under 5 MB (models excluded), installs without errors on a fresh Windows 10 machine, and the installed binary passes Windows Defender scan
**Plans**: TBD

Plans:
- [ ] 07-01: GPU detection + model recommendation — implement CUDA/GPU capability detection at startup, surface recommendation in first-run UI before download
- [ ] 07-02: Model download + validation — implement HTTP download with progress events to frontend, SHA256 checksum validation, error recovery on failure
- [ ] 07-03: NSIS packaging + signing — configure Tauri NSIS builder with models excluded, document code signing process, verify clean Defender scan on clean Windows 10 VM

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation | 3/3 | Complete   | 2026-02-27 |
| 2. Audio + Whisper | 3/3 | Complete   | 2026-02-28 |
| 3. Core Pipeline | 2/2 | Complete   | 2026-02-28 |
| 4. Pill Overlay | 1/2 | In Progress|  |
| 5. VAD + Toggle Mode | 0/2 | Not started | - |
| 6. Vocabulary + Settings | 0/3 | Not started | - |
| 7. Distribution | 0/3 | Not started | - |
