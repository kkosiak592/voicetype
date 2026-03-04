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
**Current focus:** Phase 19 — Include distil-large-v3.5 as download option and first-time run

## Position

**Milestone:** v1.2 Keyboard Hook
**Phase:** 19 — Include distil-large-v3.5 as download option and first-time run
**Plan:** 01 complete (2026-03-03) — awaiting human-verify checkpoint
**Status:** Plan 01 complete, checkpoint pending

[##########████████████████████░░░░░░░░░░░░░░░░░░░░░░] 57% (4/7 phases)

Last activity: 2026-03-04 - Completed quick task 35: Fix benchmark fairness - add VAD chunking to Whisper and incremental audio feed for streaming models

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

### Research Flags (from SUMMARY.md)

- Phase 15: Windows 11 Start menu suppression timing for VK_E8 injection (KEYDOWN only vs KEYDOWN+KEYUP) requires empirical validation during implementation — not resolvable from documentation alone
- Phase 18: Defender ML sensitivity for WH_KEYBOARD_LL + SendInput on unsigned vs signed binary cannot be determined pre-build — VirusTotal scan of actual v1.2 binary is the gate

### Pending Todos

1. Implement sub-500ms transcription latency improvements (area: backend)
2. Simplify profiles to shared dictionary and editable prompts (area: ui)

### Roadmap Evolution

- Phase 19 added: Include distil-large-v3.5 as download option and first-time run
- Phase 20 added: Implement dual CPU/GPU installers with variant-specific auto-updates
- Phase 21 added: Integration and Distribution (moved from Phase 18 — voided Phase 18 to allow 19-20 to execute first)

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

## Session Continuity

Last session: 2026-03-04
Stopped at: Quick task 35 complete — Whisper VAD chunking and streaming Moonshine incremental frame feeding
Resume file: None
