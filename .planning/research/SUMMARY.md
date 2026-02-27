# Project Research Summary

**Project:** VoiceType — Local Voice-to-Text Desktop Tool
**Domain:** Offline desktop dictation tool (Windows, GPU-accelerated, hotkey-activated)
**Researched:** 2026-02-27
**Confidence:** HIGH (core stack and architecture validated against BridgeVoice production app; features grounded in competitor analysis; pitfalls verified against official GitHub issue trackers)

## Executive Summary

VoiceType is a local-first Windows desktop dictation tool that captures microphone audio via a global hotkey, transcribes it offline using whisper.cpp running on the local GPU, applies post-processing corrections, and injects the result as text into the active application via clipboard paste. The validated reference implementation for this architecture is BridgeVoice (macOS, Tauri 2.0 + whisper.cpp), and the open-source Voquill and Keyless projects demonstrate the same pattern on Windows. The recommended stack — Tauri 2.0, whisper-rs 0.15.1, cpal 0.16.0, silero-vad-rust, and enigo 0.5.0 — is the only viable combination for the target hardware: a Pascal-architecture GPU (NVIDIA P2000) with CUDA Toolkit 11.7. The primary alternative, faster-whisper/CTranslate2, hard-requires CUDA 12 and is incompatible.

The product's primary differentiator is domain-specific vocabulary profiles — bundles of a Whisper `initial_prompt`, a regex correction dictionary, and per-profile output settings (e.g., ALL CAPS mode). No competitor offers this: BridgeVoice has a dictionary but no profiles; Wispr Flow uses cloud LLM cleanup that is incompatible with the offline premise; Superwhisper's custom modes are macOS-only and LLM-based. The structural engineering profile ships out of the box and directly targets the primary user's workflow. The v1 feature set is larger than typical MVPs because vocabulary profiles and toggle mode (which requires Silero VAD) are core to the value proposition, not optional additions.

The three highest-risk implementation areas are: (1) the floating pill overlay stealing focus from the target application — a confirmed Tauri 2.0 Windows bug that requires Win32 `WS_EX_NOACTIVATE` as the authoritative fix, not config alone; (2) the CUDA build silently falling back to CPU-only inference — detectable via Task Manager GPU utilization and must be verified as an acceptance criterion; and (3) Windows Defender flagging the binary as malware due to `SendInput` usage — a distribution-phase concern that requires code signing for any distribution beyond personal use. Each of these must be addressed in the phase that introduces the relevant component, not retrofitted.

## Key Findings

### Recommended Stack

The stack is determined primarily by hardware constraints (CUDA 11.7, Pascal sm_61) and the offline-first requirement. Tauri 2.0 provides a minimal runtime (20-40 MB RAM, ~2.5 MB installer) with native Rust FFI that makes whisper-rs integration straightforward. The GPU model default should be `ggml-large-v3-turbo-q5_0.bin` (547 MB, fits in the P2000's 5 GB VRAM) rather than the full-precision `large-v3-turbo` (1.5 GB) — 45% smaller, 19% faster, same accuracy. Models must not be bundled in the installer; Tauri's NSIS builder hard-fails for bundles over 2 GB.

**Core technologies:**
- **Tauri 2.0 (2.10.2):** Desktop framework — smallest footprint, native Rust FFI, BridgeVoice production-validated
- **whisper-rs 0.15.1 (wrapping whisper.cpp):** STT engine — only GPU option compatible with CUDA 11.7; faster-whisper requires CUDA 12
- **cpal 0.16.0:** Audio capture — WASAPI on Windows, callback-based audio thread, resamples to 16kHz
- **silero-vad-rust:** Voice activity detection — bundles ONNX model internally, enables auto-stop in toggle mode
- **enigo 0.5.0:** Text injection — clipboard paste (Ctrl+V) as primary; char-by-char SendInput as fallback only
- **tauri-plugin-global-shortcut 2.3.1:** System-wide hotkey registration — works when Tauri window is not focused
- **tauri-plugin-store 2.4.2:** Settings persistence — JSON-based, writes to %APPDATA%
- **React 18 + Vite 5 + TypeScript 5:** Frontend — official Tauri template, good TypeScript safety for IPC boundaries
- **Tailwind CSS 4.2.1:** Styling — v4 first-party Vite plugin, no config file required

### Expected Features

**Must have (table stakes) — all targeted for v1:**
- Global hotkey activation — tool is unusable without it
- Hold-to-talk mode — simpler mode; validates core pipeline without VAD dependency
- Toggle mode with Silero VAD auto-stop — toggle without VAD degrades to manual-stop only
- Floating pill overlay with recording state and frequency bar visualizer — universal pattern in this category
- Clipboard paste text injection with save/restore — core value proposition
- System tray with Quit and Settings — Windows convention for background tools
- Configurable hotkey — hardcoded keys conflict with existing application shortcuts
- Settings panel — hotkey, model, microphone, profile selection
- Multiple Whisper model sizes with GPU auto-detection — CPU fallback required for non-NVIDIA machines
- Model download on first run — installer cannot bundle 1.5 GB+ model files
- Word correction dictionary — domain accuracy without corrections is unacceptable for engineering use
- Clipboard restoration after paste — well-known UX bug if omitted

**Should have (differentiators) — targeted for v1:**
- Vocabulary profiles (structural engineering + general) — the primary competitive differentiator
- Caps lock output mode as profile property — engineering drawing and PDF markup use case
- Whisper `initial_prompt` support per profile — dramatically improves domain accuracy without fine-tuning
- Local-only privacy guarantee — explicit differentiator vs. Wispr Flow (cloud) and Superwhisper (cloud-optional)
- CUDA 11.x (Pascal) compatibility — targets enterprise engineering workstations excluded by competitors

**Defer (v1.x after validation):**
- Transcription history log with re-inject
- Regex-based post-processing corrections (level 2 phonetic matching)
- Quick-add to dictionary from system tray
- NSIS installer with auto-update

**Defer (v2+):**
- Additional domain profiles (legal, medical, software)
- Moonshine CPU streaming model
- Per-app profile auto-switching

**Anti-features to avoid:**
- Streaming/real-time partial transcription — whisper.cpp is chunk-based; streaming degrades accuracy and adds architecture complexity
- LLM-based text cleanup — breaks offline premise; Whisper initial_prompt + regex covers the engineering case
- Preview/confirmation before paste — destroys workflow speed advantage
- Cloud/API transcription fallback — contradicts privacy value proposition

### Architecture Approach

The architecture is a Tauri 2.0 Rust backend with three concurrent processing threads: an OS-managed WASAPI audio thread (cpal), a VAD thread consuming 30ms chunks via mpsc channel, and a tokio blocking thread pool for whisper inference. The key architectural constraint is that the cpal audio callback must be kept microsecond-fast — no locking, no I/O, only mpsc sends. whisper-rs inference (300ms-4s) must always run inside `tokio::task::spawn_blocking` to avoid freezing the async Tauri command executor. The React frontend communicates with the Rust backend via Tauri's typed invoke/emit IPC; high-frequency audio level data uses channels, not events. The pill overlay and settings panel are two separate Tauri windows with distinct properties (transparent/frameless/no-activate vs. standard chrome).

**Major components:**
1. **hotkeys.rs** — Global shortcut registration and event routing; triggers recording state changes
2. **audio.rs** — cpal WASAPI capture at device native rate; resamples to 16kHz; sends via mpsc
3. **vad.rs** — Silero VAD on 30ms chunks; detects speech start/end for toggle mode auto-stop
4. **transcribe.rs** — whisper-rs inference in spawn_blocking; returns raw text string
5. **corrections.rs** — Post-processing dictionary (HashMap lookup); applies profile's regex corrections
6. **injector.rs** — Clipboard save/restore + enigo Ctrl+V with 50-100ms delay
7. **profiles.rs** — Profile loading; initial_prompt selection; case normalization settings
8. **Pill.tsx** — Floating transparent always-on-top overlay; frequency bar visualizer; recording state
9. **Settings.tsx** — Full config panel in separate Tauri window; hotkey, model, profile, corrections editor
10. **AppState (state.rs)** — Single Arc<Mutex<T>> per field; managed by Tauri; accessed by all commands and background threads

### Critical Pitfalls

1. **Overlay window steals focus** — Use Win32 `WS_EX_NOACTIVATE` extended style via Tauri's Rust window builder; `focus: false` in config is insufficient on Windows (confirmed bug #11566). Record foreground HWND before hotkey fires and verify before injection. Must be addressed and verified before any text injection work.

2. **CUDA build silently falls back to CPU** — Set `CMAKE_CUDA_ARCHITECTURES="61"` explicitly for Pascal P2000; set `CUDA_PATH` and `LIBCLANG_PATH` env vars; verify GPU usage in Task Manager after every CUDA-enabled build. Latency under 500ms is the acceptance criterion for the transcription phase.

3. **Clipboard paste race condition** — Insert 50-100ms delay between `SetClipboardData` and Ctrl+V send; insert 100-150ms delay before clipboard restore; use `CF_UNICODETEXT` not `CF_TEXT`; check `GetOpenClipboardWindow` before writing. Never retrofit this — build in from day one.

4. **Windows Defender malware flag** — `SendInput` and clipboard APIs match keylogger heuristics. An OV/EV code signing certificate is required for any distribution. For personal use: document manual Defender exclusion. Default to clipboard paste (fewer SendInput calls) over char-by-char injection.

5. **Whisper hallucinations on silence** — Gate whisper.cpp with Silero VAD; discard buffers with less than 300ms of detected speech; implement hallucination detection heuristics (output length ratio, repetition patterns). VAD gate must be in place before end-to-end testing.

## Implications for Roadmap

The architecture research provides a clear build order with explicit dependency relationships. The phase structure maps directly to the component build order validated by the reference implementations.

### Phase 1: Foundation — Tauri Scaffold + Global Hotkey
**Rationale:** Zero dependencies; validates the framework setup, IPC, and window creation before any audio or ML work. Catches Tauri 2.0 configuration issues early.
**Delivers:** Working Tauri app with global hotkey that prints to console; two-window architecture (pill + settings); system tray presence.
**Addresses:** Configurable hotkey, system tray (table stakes).
**Avoids:** Prematurely wiring audio or ML before the framework is confirmed working.
**Research flag:** Standard pattern — no additional research needed.

### Phase 2: Audio Capture Pipeline
**Rationale:** Audio is the prerequisite for both transcription and VAD. Must validate WASAPI access, sample rate handling, and the mpsc channel threading pattern before adding whisper.
**Delivers:** Microphone capture at device native rate, resampled to 16kHz; WAV file output for verification; confirmed audio thread isolation via mpsc.
**Addresses:** Audio level visualizer data source; VAD input stream.
**Avoids:** cpal sample rate mismatch (WASAPI does not natively output 16kHz — resampling with `rubato` or `dasp` required).
**Research flag:** Standard pattern — cpal WASAPI docs are thorough; resampling is well-documented.

### Phase 3: Whisper Integration (GPU-Verified)
**Rationale:** Transcription is the highest-complexity and highest-risk component. Must be isolated from audio and UI during initial integration to verify CUDA build flags, model loading, and spawn_blocking pattern independently.
**Delivers:** Transcription of a test WAV file via whisper-rs with GPU acceleration verified; latency under 500ms on GPU confirmed as acceptance criterion.
**Addresses:** whisper.cpp transcription (P1 table stakes); GPU/CPU fallback detection.
**Avoids:** CUDA silent fallback to CPU (Pitfall 2) — GPU verification must be acceptance criterion for this phase, not a later check.
**Research flag:** Standard pattern for whisper-rs integration. CUDA build flags for Windows/MSVC are documented in BUILDING.md. No additional research needed.

### Phase 4: End-to-End Core Pipeline
**Rationale:** Wire audio capture to whisper transcription (hotkey → audio → whisper → console output). No UI, no injection. Validates the full data flow timing. This is the proof-of-concept gate — if latency exceeds 500ms here, the architecture needs adjustment before more is built on top of it.
**Delivers:** Hold-to-talk dictation printing to console; confirmed sub-500ms latency on GPU.
**Addresses:** Hold-to-talk mode, audio capture integration.
**Avoids:** Overlay and injection complexity masking pipeline timing issues.
**Research flag:** No additional research needed — all components validated in Phases 1-3.

### Phase 5: Text Injection
**Rationale:** Clipboard paste with save/restore is a non-trivial integration that must be built correctly once and not retrofitted. Testing against multiple target apps (Chrome, VS Code, Windows Terminal, Notepad) before adding UI catches injection failures early.
**Delivers:** Transcription injected into active application via clipboard paste; original clipboard restored; 50-100ms delays built in.
**Addresses:** Automatic text injection at cursor, clipboard restoration (table stakes).
**Avoids:** Clipboard race condition (Pitfall 3) — delays must be built in here, not added later after user reports.
**Research flag:** Standard pattern — enigo and Win32 clipboard docs are clear. No additional research needed.

### Phase 6: Pill Overlay UI
**Rationale:** The overlay window has two confirmed Tauri 2.0 Windows bugs (transparent window issues #8308, #13270; focus stealing #11566) that require Win32-level fixes. This must be built and verified against multiple target apps before any injection work relies on focus being preserved.
**Delivers:** Floating transparent always-on-top pill window; recording state indicator; frequency bar visualizer; Win32 WS_EX_NOACTIVATE applied.
**Addresses:** Floating visual indicator, audio level visualizer (P1 table stakes).
**Avoids:** Overlay focus steal (Pitfall 1) — the Win32 fix must be in place and manually tested before this phase is considered done.
**Research flag:** May need research during planning. The Win32 `WS_EX_NOACTIVATE` integration via Tauri's window builder has sparse documentation — reference the GitHub issues (#11566, #13070) and Tauri v2 win32 window attribute API.

### Phase 7: Silero VAD + Toggle Mode
**Rationale:** VAD is a dependency for toggle mode. Building toggle mode before VAD is working produces broken UX (no auto-stop). VAD also gates whisper against hallucination on silence.
**Delivers:** Silero VAD silence detection on 30ms chunks; toggle mode with auto-stop; hallucination gate (discard buffers with <300ms speech).
**Addresses:** Toggle mode, VAD auto-stop (P1 table stakes); whisper hallucination prevention (Pitfall 5).
**Avoids:** Toggle mode without VAD requires manual stop — degrades to hold-to-talk equivalent.
**Research flag:** May need research during planning. silero-vad-rust crate version is not pinned (MEDIUM confidence) — confirm current version on crates.io. Silence threshold tuning requires testing in real acoustic environments.

### Phase 8: Corrections + Vocabulary Profiles
**Rationale:** No new infrastructure needed — pure Rust logic layered on top of the working transcription pipeline. Build after the pipeline is validated so corrections can be tested against real transcriptions.
**Delivers:** Word correction dictionary (HashMap lookup); structural engineering profile with initial_prompt and regex corrections; general profile; ALL CAPS output mode as profile property; profile switching.
**Addresses:** Word correction dictionary, vocabulary profiles, caps lock mode, structural engineering profile, Whisper initial_prompt per profile (primary differentiators).
**Avoids:** Long initial_prompt consuming model context — keep to 50-100 words per profile.
**Research flag:** Standard pattern — no additional research needed. Correction logic is straightforward Rust string processing.

### Phase 9: Settings Panel
**Rationale:** Settings require the two-window Tauri architecture to be exercised. All features that settings configure (hotkey, profile, model) must already work before building the UI around them.
**Delivers:** Full settings Tauri window; hotkey configuration with runtime apply (no restart); model selection; microphone selection; correction dictionary editor UI.
**Addresses:** Settings panel, configurable hotkey (table stakes).
**Avoids:** Mixing pill window and settings window properties in one window (Architecture Anti-Pattern 4).
**Research flag:** Standard pattern — tauri-plugin-store docs are thorough. Nested JSON settings structure must be established from day one (flat key-value is a known debt trap from PITFALLS.md).

### Phase 10: Model Download + First-Run UX
**Rationale:** Model download must be built last because it requires all the infrastructure it depends on (GPU detection, model selection, progress events) to already be working.
**Delivers:** First-run detection of missing model; progress UI; SHA256 checksum validation; GPU auto-detection with model recommendation (large-v3-turbo-q5_0 vs. small-q5_1).
**Addresses:** Model download on first run (P1 table stakes); multiple model sizes.
**Avoids:** Bundling model in installer (NSIS hard-fails above 2 GB — Pitfall confirmed via issue #7372).
**Research flag:** Standard pattern — Hugging Face CDN download is straightforward. SHA256 validation for model files needs checksums sourced from whisper.cpp models README.

### Phase 11: Distribution + Code Signing
**Rationale:** Distribution is the final phase because code signing and NSIS packaging require a stable, tested binary. The Defender false-positive risk makes this a hard blocker for sharing with any colleagues.
**Delivers:** NSIS installer (~2.5 MB, models excluded); code signing with OV/EV certificate; verified clean Defender scan on fresh Windows 10 VM.
**Addresses:** Distribution; Windows Defender false positive prevention (Pitfall 4).
**Avoids:** Shipping unsigned binary — OV certificate required; EV provides immediate SmartScreen reputation.
**Research flag:** May need research during planning. OV vs EV certificate comparison (cost, turnaround, SmartScreen reputation difference) and Tauri NSIS signing configuration should be researched. Certificate costs $300-500/year minimum.

### Phase Ordering Rationale

- Phases 1-4 establish the core pipeline in strict dependency order: framework → audio → transcription → integration. Each phase must be verified before the next is started.
- Phase 5 (injection) and Phase 6 (overlay) are ordered to test injection before the overlay is introduced, preventing focus-steal bugs from hiding injection failures during testing.
- Phase 7 (VAD) comes after the full injection pipeline is working because VAD is a refinement that adds auto-stop; hold-to-talk without VAD is a valid v1 entry point for early testing.
- Phases 8-10 add the differentiating features and UX polish after the core loop is proven.
- Phase 11 (distribution) is always last — it validates the full product as built, not individual components.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 6 (Pill Overlay):** Win32 `WS_EX_NOACTIVATE` integration in Tauri 2.0 window builder is sparsely documented; reference issues #11566 and #13070; may need to examine Keyless or Voquill source code for working examples.
- **Phase 7 (Silero VAD):** silero-vad-rust crate version is unverified (MEDIUM confidence); threshold tuning is empirical and environment-dependent; confirm crate API stability before planning implementation details.
- **Phase 11 (Distribution):** OV vs EV certificate selection and Tauri NSIS code signing configuration need concrete documentation review; costs and CA selection need decision.

Phases with standard, well-documented patterns:
- **Phase 1 (Tauri scaffold):** Official Tauri 2.0 docs are comprehensive.
- **Phase 2 (Audio capture):** cpal WASAPI docs plus resampling crates are well-documented.
- **Phase 3 (Whisper integration):** whisper-rs BUILDING.md covers Windows CUDA build exactly.
- **Phase 4 (Pipeline integration):** No new components; wiring known pieces.
- **Phase 5 (Text injection):** enigo and Win32 clipboard docs are clear.
- **Phase 8 (Corrections + Profiles):** Pure Rust string processing; no external integrations.
- **Phase 9 (Settings panel):** tauri-plugin-store docs are thorough.
- **Phase 10 (Model download):** HTTP download + SHA256 + progress events are standard patterns.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Core stack (Tauri, whisper-rs, cpal, enigo) verified against official crate docs and BridgeVoice production use. CUDA 11.7/Pascal compatibility confirmed via community reports and whisper.cpp CUDA feature docs. ort (RC) and silero-vad-rust (unpinned version) are MEDIUM. |
| Features | HIGH | Grounded in BridgeVoice documentation, Wispr Flow feature page, VoiceTypr/OpenWhispr source, and competitor analysis. Feature prioritization is based on multiple reference implementations. |
| Architecture | HIGH | Threading patterns (cpal mpsc, spawn_blocking) verified against official Tauri and tokio docs. Two-window pattern validated against known Tauri behavior. Build order confirmed by reference project structure. |
| Pitfalls | HIGH | Five critical pitfalls verified against official Tauri GitHub issues (with issue numbers), whisper.cpp issue tracker, and cpal known limitations. Clipboard race condition timing from BridgeVoice validation. |

**Overall confidence:** HIGH

### Gaps to Address

- **silero-vad-rust version:** Crate version not pinned in research. Confirm current stable version on crates.io before writing Cargo.toml. Verify the bundled ONNX model opset is compatible with the ort version pulled transitively.
- **cpal resampling implementation:** WASAPI outputs 44.1kHz or 48kHz; Whisper requires 16kHz. The resampling crate choice (`rubato` vs. `dasp`) and configuration (quality vs. latency tradeoff) needs validation with real audio before treating as solved. `AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM` has known quality issues per PITFALLS.md.
- **Win32 WS_EX_NOACTIVATE in Tauri 2.0:** The exact Rust API call to set this extended window style via Tauri 2.0's window builder needs to be identified from Tauri source or working open-source examples. Config alone is insufficient.
- **Initial_prompt length limits:** Research notes 50-100 word limit to avoid consuming model context. The specific token budget should be validated against whisper.cpp's context window size before writing the structural engineering profile.
- **Code signing certificate:** OV vs EV decision, CA selection, and Tauri NSIS signing integration are unresolved. Budget must be confirmed before Phase 11.

## Sources

### Primary (HIGH confidence)
- [BridgeVoice Documentation](https://docs.bridgemind.ai/docs/bridgevoice) — recording modes, widget states, dictionary, history, model sizes
- [Tauri 2.0 official docs](https://v2.tauri.app/) — IPC, state management, window config, plugin system
- [whisper-rs BUILDING.md](https://github.com/tazz4843/whisper-rs/blob/master/BUILDING.md) — Windows CUDA build requirements
- [cpal docs.rs](https://docs.rs/cpal) — WASAPI audio capture API
- [tokio spawn_blocking docs](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html) — blocking thread pool pattern
- [enigo 0.5.0 docs.rs](https://docs.rs/enigo/latest/enigo/) — keyboard/clipboard injection API
- [tauri-plugin-store docs.rs](https://docs.rs/crate/tauri-plugin-store/latest) — settings persistence
- [tauri-plugin-global-shortcut docs.rs](https://docs.rs/crate/tauri-plugin-global-shortcut/latest) — system-wide hotkeys
- [whisper.cpp models README](https://github.com/ggml-org/whisper.cpp/blob/master/models/README.md) — model sizes and quantization
- [Tauri GitHub issue #7372](https://github.com/tauri-apps/tauri/issues/7372) — NSIS 2GB installer limit confirmed
- [Tauri GitHub issue #11566](https://github.com/tauri-apps/tauri/issues/11566) — focus: false config broken on Windows
- Artifacts: `artifacts/research/2026-02-27-voice-to-text-desktop-tool-technical.md` — prior deep technical research

### Secondary (MEDIUM confidence)
- [Wispr Flow Features Page](https://wisprflow.ai/features) — competitor feature analysis
- [VoiceTypr GitHub](https://github.com/moinulmoin/voicetypr) — open-source reference implementation
- [Keyless reference project](https://github.com/hate/keyless) — Tauri v2 voice-to-text reference
- [Voquill reference project](https://github.com/josiahsrc/voquill) — Tauri + React voice-to-text reference
- [silero-vad-rust crates.io](https://crates.io/crates/silero-vad-rust) — VAD crate (version unverified)
- [ort 2.0.0-rc.11 docs.rs](https://docs.rs/crate/ort/latest) — ONNX Runtime (still RC)
- [Tauri transparent window issue #8308](https://github.com/tauri-apps/tauri/issues/8308) — transparent window Windows bugs
- [Tauri transparent window issue #13270](https://github.com/tauri-apps/tauri/issues/13270) — overlay window workaround
- [cpal sample rate issue #593](https://github.com/RustAudio/cpal/issues/593) — WASAPI 16kHz limitation
- [Whisper hallucination on empty audio — OpenAI community](https://community.openai.com/t/whisper-api-hallucinating-on-empty-sections/93646) — confirmed reproducible

### Tertiary (LOW confidence)
- Windows Defender HackTool false positives (multiple forum reports 2024-2025) — pattern documented for enigo-adjacent tools; enigo specifically needs validation

---
*Research completed: 2026-02-27*
*Ready for roadmap: yes*
