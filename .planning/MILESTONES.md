# Milestones
## v1.5 Prefix Text (Shipped: 2026-03-08)

**Phases completed:** 1 phase, 1 plan, 2 tasks
**Timeline:** 1 day (2026-03-08)
**Git range:** feat(27-01)

**Delivered:** Toggleable prefix string prepended to all dictated output, for annotation use cases like shop drawing review.

**Key accomplishments:**
- Profile struct extended with prefix_enabled and prefix_text fields, with 4 IPC commands (Phase 27)
- Pipeline prefix prepend step after ALL CAPS formatting so prefix string is not uppercased (Phase 27)
- PrefixTextInput component with toggle switch and conditional text input in General Settings Output card (Phase 27)
- Settings persist across restarts via settings.json (Phase 27)

---


## v1.4 Per-App Settings (Shipped: 2026-03-07)

**Phases completed:** 4 phases, 5 plans, 10 tasks
**Execution time:** 33 min
**Timeline:** 5 days (2026-03-03 -> 2026-03-07)
**Git range:** feat(23-01)..feat(26-01)

**Delivered:** Per-application ALL CAPS overrides with Win32 foreground detection, three-state toggle UI, and searchable process dropdown.

**Key accomplishments:**
- Win32 foreground detection with GetForegroundWindow chain, UWP resolution via EnumChildWindows, and PROCESS_QUERY_LIMITED_INFORMATION for elevated process safety (Phase 23)
- Pure resolve_all_caps() override resolution with 8 unit tests and safe lock ordering in pipeline (Phase 24)
- App Rules settings page with color-coded three-state dropdown (Inherit/Force ON/Force OFF) and 3-second detect-app countdown flow (Phase 25)
- Browse Running Apps searchable dropdown with CreateToolhelp32Snapshot + EnumWindows two-phase process enumeration (Phase 26)
- Per-app rules persistence via settings.json with startup hydration and case-normalized exe name keys (Phases 23-24)

---

## v1.3 Clipboard Simplification (Shipped: 2026-03-07)

**Phases completed:** 1 phase, 1 plan, 1 task
**Commits:** 2
**Lines changed:** +7 / -31 (net -24 lines)
**Timeline:** 1 day (2026-03-07)
**Git range:** v1.2.0..v1.3.0

**Delivered:** Simplified clipboard injection flow — removed save/restore logic and 80ms post-paste sleep, matching standard dictation tool behavior where transcription text stays on clipboard.

**Key accomplishments:**
- Removed clipboard save/restore from inject_text — transcription stays on clipboard after paste (Phase 22)
- Eliminated 80ms post-paste sleep that only existed for restore timing (Phase 22)
- Updated doc comment to reflect simplified 3-step flow: set clipboard → verify → paste (Phase 22)

---

## v1.2 Keyboard Hook (Shipped: 2026-03-07)

**Phases completed:** 10 phases (1 voided), 15 plans, 21 quick tasks
**Commits:** 253
**Lines changed:** +42,468 / -2,176
**Lines of code:** 23,557 (Rust + TypeScript)
**Timeline:** 5 days (2026-03-02 -> 2026-03-07)
**Git range:** v1.1.0..v1.2.0

**Delivered:** Custom WH_KEYBOARD_LL keyboard hook with Ctrl+Win modifier-only activation, three transcription engines (Whisper + Parakeet + Moonshine), engine-agnostic VAD chunking, bundled CUDA DLLs, and comprehensive UI polish.

**Key accomplishments:**
- WH_KEYBOARD_LL low-level keyboard hook on dedicated thread with 50ms debounce, Start menu suppression, and clean shutdown — replacing RegisterHotKey for modifier-only combos (Phases 15-17)
- Moonshine Tiny ONNX engine integrated as third transcription backend with VAD chunking and GPU support (Phase 19.1)
- Data-driven model selection with benchmark stats, parakeet-tdt-v2-fp32 as universal recommendation (Phase 19.2)
- UI polish: tray icon fixes, profile simplification to single vocabulary prompt, transcription history panel with click-to-copy (Phase 19.3)
- CUDA DLLs bundled in single installer — one installer works for all users with runtime GPU fallback (Phase 20)
- Engine-agnostic VAD chunking for 60s+ recordings across all three engines (Phase 20.1)

### Known Gaps
- DIST-01: Signed v1.2 binary passes VirusTotal scan — Phase 18/21 voided, deferred to future milestone

---

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

