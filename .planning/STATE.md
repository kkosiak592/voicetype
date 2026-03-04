---
gsd_state_version: 1.0
milestone: v1.2
milestone_name: Keyboard Hook
status: planning
stopped_at: Completed 19.3-03-PLAN.md
last_updated: "2026-03-04T21:51:55.112Z"
last_activity: "2026-03-04 - Completed Phase 19.1 Plan 02: Moonshine Tiny frontend integration verified end-to-end"
progress:
  total_phases: 10
  completed_phases: 7
  total_plans: 16
  completed_plans: 13
---

---
gsd_state_version: 1.0
milestone: v1.2
milestone_name: Keyboard Hook
status: planning
last_updated: "2026-03-04T19:34:51.550Z"
progress:
  total_phases: 9
  completed_phases: 6
  total_plans: 13
  completed_plans: 10
---

---
gsd_state_version: 1.0
milestone: v1.2
milestone_name: Keyboard Hook
status: planning
last_updated: "2026-03-04T14:02:49.549Z"
progress:
  total_phases: 8
  completed_phases: 5
  total_plans: 12
  completed_plans: 9
---

---
gsd_state_version: 1.0
milestone: v1.2
milestone_name: Keyboard Hook
status: completed
last_updated: "2026-03-04T13:56:06.761Z"
progress:
  total_phases: 8
  completed_phases: 5
  total_plans: 12
  completed_plans: 9
---

---
gsd_state_version: 1.0
milestone: v1.2
milestone_name: Keyboard Hook
status: completed
last_updated: "2026-03-04T13:06:53.061Z"
progress:
  total_phases: 8
  completed_phases: 4
  total_plans: 11
  completed_plans: 8
---

---
gsd_state_version: 1.0
milestone: v1.2
milestone_name: Keyboard Hook
status: planning
last_updated: "2026-03-03T15:22:44.378Z"
progress:
  total_phases: 7
  completed_phases: 4
  total_plans: 12
  completed_plans: 7
---

---
gsd_state_version: 1.0
milestone: v1.2
milestone_name: Keyboard Hook
status: planning
last_updated: "2026-03-03T15:06:28.550Z"
progress:
  total_phases: 6
  completed_phases: 3
  total_plans: 7
  completed_plans: 6
---

# Session State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-02)

**Core value:** Voice dictation must feel instant — sub-1500ms from end-of-speech to text appearing at the cursor, with zero internet dependency.
**Current focus:** Phase 19.1 — Integrate Moonshine Tiny model into main app with VAD chunking and GPU support

## Position

**Milestone:** v1.2 Keyboard Hook
**Phase:** 19.1 — Integrate Moonshine Tiny model into main app with VAD chunking and GPU support
**Plan:** 02 complete (2026-03-04)
**Status:** Ready to plan

[##########████████████████████░░░░░░░░░░░░░░░░░░░░░░] 57% (4/7 phases)

Last activity: 2026-03-04 - Completed quick task 39: Add ALL CAPS toggle to the frontend UI

## Performance Metrics

| Metric | Value |
|--------|-------|
| Phases defined | 4 |
| Requirements mapped | 15/15 |
| Plans complete | 2 |
| Blockers | 0 |
| Phase 15-hook-module P02 | 2 | 2 tasks | 1 files |
| Phase 15-hook-module P03 | 9 | 1 tasks | 3 files |
| Phase 16-rebind-and-coexistence P01 | 5 | 2 tasks | 2 files |
| Phase 16-rebind-and-coexistence P02 | 8 | 2 tasks | 2 files |
| Phase 17-frontend-capture-ui P01 | 5 | 2 tasks | 4 files |
| Phase 19 P01 | 4 | 2 tasks | 3 files |
| Phase 19.1 P01 | 35 | 2 tasks | 6 files |
| Phase 19.1 P02 | 0 | 2 tasks | 3 files |
| Phase 19.2 P01 | 4 | 2 tasks | 3 files |
| Phase 19.3 P01 | 8 | 2 tasks | 3 files |
| Phase 19.3 P02 | 7 | 2 tasks | 7 files |
| Phase 19.3 P03 | 377 | 2 tasks | 5 files |

## Accumulated Context

### Decisions

- v1.2: WH_KEYBOARD_LL on dedicated thread — no Tokio task, no main thread; Win32 GetMessage loop required
- v1.2: AtomicBool + mpsc::try_send only in hook callback — never lock Mutex, never allocate, never async
- v1.2: DeviceEventFilter::Always applied before build() — mandatory fix for Tauri issue #13919
- v1.2: VK_E8 mask-key injection for Start menu suppression — VK_07 reserved by Xbox Game Bar on Win10 1909+
- v1.2: tauri-plugin-global-shortcut kept as fallback — hook path for modifier-only combos, plugin for standard combos
- v1.2: No new Cargo dependencies — windows v0.58 + 3 feature flags only
- 15-01: std::thread::spawn for hook thread (not tokio) — WH_KEYBOARD_LL requires stable OS thread with Win32 message pump
- 15-01: hmod=None in SetWindowsHookExW (dwThreadId=0) — correct for global hooks; using GetModuleHandle causes silent removal
- 15-01: LLKHF_INJECTED guard in hook_proc prevents recursion from Plan 02 VK_E8 injection
- [Phase 15-02]: Tasks 1+2 combined into single commit — inject_mask_key is called inline from hook_proc, atomically correct; separating would require intermediate broken state
- [Phase 15-02]: Repeated Win keydown during active combo suppressed with inject+LRESULT(1) to prevent Start menu mid-recording
- [Phase 15-hook-module]: handle_hotkey_event(pressed: bool) avoids constructing private ShortcutEvent — both code paths converge on bool
- [Phase 15-hook-module]: 15-03: Tauri v2 Builder.run() takes only Context — hook cleanup moved to tray quit handler; HookHandle::Drop is safety net
- [Phase 15-hook-module]: 15-03: Default hotkey changed to ctrl+win for fresh installs; existing users keep saved hotkey
- 16-01: is_modifier_only predicate replaces hardcoded is_hook_hotkey — handles any modifier-only combo, not just ctrl+win
- 16-01: Global-shortcut plugin always registered (with or without shortcuts) for runtime rebind support
- 16-01: rebind_hotkey checks PipelineState::current() before switching backends
- 16-01: Hook-failure fallback persists ctrl+shift+space to settings.json; frontend reads from settings.json for displayed hotkey
- 16-02: hookAvailable defaults to true — silent-catch IPC pattern prevents warning flicker and maintains pre-v1.2 compatibility
- 16-02: Tasks 1+2 committed atomically — splitting would create intermediate TypeScript type error
- 16-02: Amber color for hook warning (not red) — app still functions with fallback shortcut, warning severity not error severity
- 17-01: modifierToken uses e.code (not e.ctrlKey/e.metaKey flags) on keyup — flags already false for released key
- 17-01: comboRef tracks full modifier set (not depleted heldRef) for combo on all-released
- 17-01: MODIFIER_ORDER sorts tokens deterministically (ctrl < alt < shift < meta) regardless of press order
- 17-01: HookAvailable registered on Builder (not setup closure) — webview2 COM pumps messages before setup() runs
- 17-01: Hook warning gated on modifier-only hotkey; status refreshed after rebind
- [Phase 19]: model_info() 4-tuple embeds URL per-model; download_url() removed — supports multi-repo Whisper models
- [Phase 19]: SHA256 for distil-large-v3.5 obtained from LFS pointer file; actual size 1,519,521,155 bytes
- [Quick 26]: q5_0 for distil-large-v3.5 — 513 MB vs 1.52 GB fp16; hosted on GitHub Releases v1.2-models
- [Quick 26]: set_model() early-return requires is_some() guard on WhisperContext — settings.json model_id alone insufficient after first-run
- [Quick 26]: default-run = voice-to-text in Cargo.toml required when multiple [[bin]] targets exist
- [Quick 29]: ort added as direct optional dep for bench_extra — Rust 2021 edition requires explicit dep to use ort types in benchmark binary
- [Phase 19.1]: 19.1-01: moonshine feature adds dep:ort directly — transcribe_moonshine.rs uses ort::execution_providers types directly
- [Phase 19.1]: 19.1-01: TranscribeRsEngine trait alias used to avoid collision with crate-level TranscriptionEngine enum
- [Phase 19.1]: 19.1-01: DirectML EP for Moonshine falls back to CPU — ort DirectML EP availability for ONNX Moonshine not validated
- [Phase 19.1]: 19.1-01: decoder_model_merged.onnx as sentinel file for Moonshine download existence check (largest file, last to complete)
- [Phase 19.1]: 19.1-02: Moonshine download props mirror the Parakeet fp32 pattern — onMoonshineDownload/moonshineDownloading/moonshinePercent/moonshineError passed through ModelSelector
- [Phase 19.1]: 19.1-02: Moonshine Tiny placed first in FirstRun MODELS array — smallest/fastest, best default for first-time users
- [Phase 19.1]: 19.1-02: currentEngine === 'moonshine' guard renders note that vocabulary prompting is unsupported (corrections dictionary still applies)
- [Phase 19.2]: 19.2-01: parakeet-tdt-v2-fp32 recommended unconditionally — benchmark data shows best accuracy/latency balance on all hardware
- [Phase 19.2]: 19.2-01: distil-large-v3.5 removal from model_id_to_path() causes graceful auto-fallback for existing users via startup error chain — no migration code needed
- [Phase 19.2]: 19.2-01: list_models() gpu_mode variable removed entirely — all recommended flags are now compile-time constants
- [Phase 19.3]: set_visible(false) used for destroy_tray — TrayIcon type in Tauri v2 does not expose public destroy(); set_visible(false) removes icon from Windows tray
- [Phase 19.3]: MouseButton::Left guard on DoubleClick handler prevents right-double-click from opening settings unintentionally
- [Phase 19.3]: try/catch around invoke('destroy_tray') in updater.ts ensures relaunch proceeds even if IPC fails (non-critical cleanup)
- [Phase 19.3]: corrections.default flat key replaces corrections.{profile_id} scoping — single profile needs no per-profile scoping
- [Phase 19.3]: history SectionId added to Sidebar in Plan 02 (not 03) — prevents file conflict between wave 2 and wave 3 plans
- [Phase 19.3]: history.rs uses tauri::Manager import for app.state() — Manager trait must be in scope for method resolution
- [Phase 19.3]: HistoryState loaded in setup() before build_tray — manage() call precedes all IPC access

### Research Flags (from SUMMARY.md)

- Phase 15: Windows 11 Start menu suppression timing for VK_E8 injection (KEYDOWN only vs KEYDOWN+KEYUP) requires empirical validation during implementation — not resolvable from documentation alone
- Phase 18: Defender ML sensitivity for WH_KEYBOARD_LL + SendInput on unsigned vs signed binary cannot be determined pre-build — VirusTotal scan of actual v1.2 binary is the gate

### Pending Todos

1. Re-integrate Caps Lock toggle feature (area: ui)
2. Remove stale Parakeet vocabulary prompting warning from model page (area: ui)
3. Add filler word removal to transcription output (area: general)
4. Add per-application profiles with auto-switch on focused window (area: general)
5. Learn from user corrections to auto-improve dictionary (area: general)
6. Long-press pill to drag reposition and double-click to reset home (area: ui)
7. Add always-listen mode to reduce activation latency (area: general)

### Roadmap Evolution

- Phase 19 added: Include distil-large-v3.5 as download option and first-time run
- Phase 20 added: Implement dual CPU/GPU installers with variant-specific auto-updates
- Phase 21 added: Integration and Distribution (moved from Phase 18 — voided Phase 18 to allow 19-20 to execute first)
- Phase 19.1 inserted after Phase 19: Integrate Moonshine Tiny model into main app with VAD chunking and GPU support (URGENT)
- Phase 19.2 inserted after Phase 19.1: Revamp model selection with benchmark stats, recommend parakeet, remove distil-large-v3.5 (URGENT)
- Phase 19.3 inserted after Phase 19.2: UI polish — tray icons, profile simplification, history panel, double-click settings (URGENT)

### Blockers/Concerns

None active.

### Quick Tasks Completed

| # | Description | Date | Commit | Status | Directory |
|---|-------------|------|--------|--------|-----------|
| 26 | Quantize distil-large-v3.5 from fp16 (1.52 GB) to q5_0 (513 MB); fix set_model() early-return bug and Cargo.toml default-run | 2026-03-03 | d46e7ec | Verified | [26-quantize-distil-large-v3-5-from-fp16-to-](./quick/26-quantize-distil-large-v3-5-from-fp16-to-/) |
| 27 | Create standalone benchmark script with TTS test WAV generation and multi-model latency measurement | 2026-03-03 | 76669d4 | Verified | [27-create-standalone-benchmark-script-with-](./quick/27-create-standalone-benchmark-script-with-/) |
| 28 | Add Moonshine tiny/base and SenseVoice to benchmark binary via transcribe-rs behind bench_extra feature flag | 2026-03-03 | bc32349 | Verified | [28-add-moonshine-and-sensevoice-to-benchmar](./quick/28-add-moonshine-and-sensevoice-to-benchmar/) |
| 29 | Patch transcribe-rs locally for configurable CUDA/DirectML execution providers in Moonshine/SenseVoice benchmarks | 2026-03-03 | 222db00 | Verified | [29-patch-transcribe-rs-for-gpu-execution-pr](./quick/29-patch-transcribe-rs-for-gpu-execution-pr/) |
| 30 | Switch audio capture from persistent stream to on-demand (open on record start, drop after pipeline) — removes Windows mic tray icon when idle | 2026-03-03 | db6fb28 | Verified | [30-switch-audio-capture-from-persistent-str](./quick/30-switch-audio-capture-from-persistent-str/) |
| 31 | Add VAD-based chunking to benchmark for Moonshine/SenseVoice on clips >30s | 2026-03-03 | 2dfaeef | Verified | [31-add-vad-based-chunking-to-benchmark-for-](./quick/31-add-vad-based-chunking-to-benchmark-for-/) |
| 32 | Add 2 more phrase variants per clip duration (9 total WAVs) + markdown report output | 2026-03-03 | 220b0bf | Complete | [32-add-2-more-phrase-variants-per-clip-dura](./quick/32-add-2-more-phrase-variants-per-clip-dura/) |
| 33 | Add GPU execution_providers to streaming Moonshine engine; wire moonshine-streaming-tiny/small/medium into benchmark | 2026-03-03 | 61171c7 | Complete | [33-add-moonshine-v2-streaming-models-to-ben](./quick/33-add-moonshine-v2-streaming-models-to-ben/) |
| 34 | Add VAD-based chunking to parakeet benchmark section for clips >30s | 2026-03-04 | 21d4441 | Complete | [34-add-vad-based-chunking-to-parakeet-model](./quick/34-add-vad-based-chunking-to-parakeet-model/) |
| 35 | Fix benchmark fairness: add VAD chunking to Whisper and switch streaming Moonshine to incremental frame feeding | 2026-03-04 | bdb3c14 | Complete | [35-fix-benchmark-fairness-add-vad-chunking-](./quick/35-fix-benchmark-fairness-add-vad-chunking-/) |
| 36 | Add 90s benchmark clips (deep-sea oceanography, aviation history, renewable energy) to PS1 script and benchmark.rs with 4-column pivot tables | 2026-03-04 | e8bea88 | Complete | [36-add-90s-benchmark-clips-to-benchmark-bin](./quick/36-add-90s-benchmark-clips-to-benchmark-bin/) |
| 37 | Add --progressive flag to benchmark binary: VAD-driven real-time chunk dispatch with post-release latency metric across all 7 engine sections | 2026-03-04 | 0bf1333 | Complete | [37-add-progressive-flag-to-benchmark-binary](./quick/37-add-progressive-flag-to-benchmark-binary/) |
| 38 | Remove entire Vocabulary section from settings UI and all vocabulary_prompt/initial_prompt plumbing from Rust backend | 2026-03-04 | 6c3616b | Complete | [38-remove-entire-vocabulary-section-from-se](./quick/38-remove-entire-vocabulary-section-from-se/) |
| 39 | Add ALL CAPS toggle to the frontend UI | 2026-03-04 | 090526e | Complete | [39-add-all-caps-toggle-to-the-frontend-ui](./quick/39-add-all-caps-toggle-to-the-frontend-ui/) |

## Session Continuity

Last session: 2026-03-04T22:04:02Z
Stopped at: Completed 19.3-03-PLAN.md
Resume file: None
