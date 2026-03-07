# Roadmap: VoiceType

## Milestones

- ✅ **v1.0 MVP** — Phases 1-8 + 4.1, 6.1 (shipped 2021-03-02)
- ✅ **v1.1 Auto-Updates & CI/CD** — Phases 11-14 (shipped 2021-03-02)
- 🚧 **v1.2 Keyboard Hook** — Phases 15-21 (in progress)

## Phases

<details>
<summary>✅ v1.0 MVP (Phases 1-8) — SHIPPED 2021-03-02</summary>

- [x] Phase 1: Foundation (3/3 plans) — completed 2021-02-27
- [x] Phase 2: Audio + Whisper (3/3 plans) — completed 2021-02-28
- [x] Phase 3: Core Pipeline (2/2 plans) — completed 2021-02-28
- [x] Phase 4: Pill Overlay (2/2 plans) — completed 2021-02-28
- [x] Phase 4.1: Premium Pill UI (2/2 plans) — completed 2021-02-28 (INSERTED)
- [x] Phase 5: VAD + Toggle Mode (2/2 plans) — completed 2021-03-01
- [x] Phase 6: Vocabulary + Settings (4/4 plans) — completed 2021-03-01
- [x] Phase 6.1: Fix Tray Icons (2/2 plans) — completed 2021-03-01 (INSERTED)
- [x] Phase 7: Distribution (3/3 plans) — completed 2021-03-01
- [x] Phase 8: Parakeet TDT + Latency (3/3 plans) — completed 2021-03-02

</details>

<details>
<summary>✅ v1.1 Auto-Updates & CI/CD (Phases 11-14) — SHIPPED 2021-03-02</summary>

- [x] **Phase 11: Signing & Repo Setup** - Generate Ed25519 keypair, push source to public GitHub repo, configure secrets (completed 2021-03-02)
- [x] **Phase 12: Plugin Integration** - Add updater plugin to app, implement update check UI with progress and relaunch (completed 2021-03-02)
- [x] **Phase 13: CI/CD Pipeline** - GitHub Actions workflow that builds, signs, and publishes releases on tag push (completed 2021-03-02)
- [x] **Phase 14: Release Workflow** - Document release process and changelog template; verify end-to-end update delivery (completed 2021-03-02)

</details>

### 🚧 v1.2 Keyboard Hook (In Progress)

**Milestone Goal:** Replace tauri-plugin-global-shortcut with a custom WH_KEYBOARD_LL low-level keyboard hook, enabling Ctrl+Win modifier-only hotkey activation with debounce for reliable key ordering.

- [x] **Phase 15: Hook Module** - Install WH_KEYBOARD_LL on a dedicated thread, implement modifier state machine with debounce and Start menu suppression, wire hold-to-talk end-to-end (completed 2021-03-03)
- [x] **Phase 16: Rebind and Coexistence** - Route modifier-only combos through the hook and standard combos through tauri-plugin-global-shortcut; surface hook failure to user (completed 2021-03-03)
- [x] **Phase 17: Frontend Capture UI** - Accept and display modifier-only combos in the hotkey capture dialog and settings panel (completed 2021-03-03)

## Phase Details

### Phase 15: Hook Module
**Goal**: A working WH_KEYBOARD_LL keyboard hook runs on a dedicated thread, detects Ctrl+Win with 50ms debounce, suppresses the Start menu, drives hold-to-talk end-to-end, and shuts down cleanly — all five critical pitfalls addressed from the first commit
**Depends on**: Phase 14 (v1.1 complete)
**Requirements**: HOOK-01, HOOK-02, HOOK-03, HOOK-04, MOD-01, MOD-02, MOD-03, MOD-04, MOD-05, INT-01
**Success Criteria** (what must be TRUE):
  1. With the VoiceType settings window focused, pressing Ctrl+Win starts recording and releasing it triggers transcription — the callback fires even when the Tauri window has focus
  2. Pressing Win before Ctrl (reversed order) within normal typing speed still activates dictation, confirming the 50ms debounce handles press-order variation
  3. After Ctrl+Win activates dictation, the Start menu does not open on Win key release; pressing Win alone (without Ctrl) continues to open the Start menu normally
  4. Closing VoiceType while the hook is installed leaves no dangling hook — subsequent keyboard input is unaffected and no error appears in Event Viewer
  5. Rapid Ctrl+Win activations (20 in sequence) produce exactly 20 recording sessions with no dropped or duplicate events, confirming the non-blocking callback stays under the 5ms budget
**Plans**: 3 plans
- [x] 15-01-PLAN.md — Hook thread infrastructure, Cargo.toml features, DeviceEventFilter fix
- [x] 15-02-PLAN.md — Modifier state machine with debounce, exact-match, VK_E8 suppression
- [x] 15-03-PLAN.md — Wire into setup(), conditional routing, default hotkey change, end-to-end verification

### Phase 16: Rebind and Coexistence
**Goal**: Changing the hotkey in settings correctly switches between the hook backend and tauri-plugin-global-shortcut at runtime, with no double-firing, and surfaces hook installation failure as a visible status in settings
**Depends on**: Phase 15
**Requirements**: INT-02, INT-03
**Success Criteria** (what must be TRUE):
  1. Changing the hotkey from Ctrl+Win to a standard combo (e.g. Ctrl+Shift+V) in settings stops the hook for that combo and registers it via tauri-plugin-global-shortcut — both activate dictation correctly with no double-firing
  2. Changing back from a standard hotkey to Ctrl+Win unregisters the standard hotkey and re-enables the hook path — no overlap period where both fire
  3. If WH_KEYBOARD_LL installation fails at startup, the settings panel shows a "hook unavailable, using fallback" status and the standard hotkey continues to work
**Plans**: 2 plans
  - [ ] 16-01-PLAN.md — Backend routing logic (is_modifier_only, routed IPC commands, startup routing, hook-failure fallback, get_hook_status IPC)
  - [ ] 16-02-PLAN.md — Frontend hook-failure warning (load hook status on mount, inline warning in GeneralSection)

### Phase 17: Frontend Capture UI
**Goal**: The hotkey capture dialog and settings panel fully support modifier-only combos — users can configure Ctrl+Win without needing to press a letter key, and the display reads naturally
**Depends on**: Phase 15
**Requirements**: UI-01, UI-02
**Success Criteria** (what must be TRUE):
  1. Opening the hotkey capture dialog and pressing only Ctrl+Win (no additional key) registers as a valid hotkey and saves without error
  2. The settings panel displays the saved modifier-only combo as "Ctrl + Win" — not an error state, empty field, or raw key code
  3. The existing capture flow for standard hotkeys (letter key + modifiers) continues to work unchanged alongside the new modifier-only path
**Plans**: 1 plan
- [ ] 17-01-PLAN.md — Dual keydown/keyup capture with modifier-only support, progressive display, canonical token ordering, human verification

### Phase 18: Integration and Distribution — VOIDED
Moved to Phase 21 to allow phases 19-20 to complete first.

## Progress

**Execution Order:**
Phases execute in numeric order: 15 → 16 → 17 → 19 → 19.1 → 19.2 → 19.3 → 20 → 20.1 → 21

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 15. Hook Module | v1.2 | Complete | Complete | 2021-03-03 |
| 16. Rebind and Coexistence | v1.2 | 2/2 | Complete | 2021-03-03 |
| 17. Frontend Capture UI | v1.2 | 1/1 | Complete | 2021-03-03 |
| 18. Integration and Distribution | v1.2 | — | Voided | 2021-03-03 |
| 19. Distil-large-v3.5 | 1/1 | Complete   | 2021-03-03 | - |
| 19.1. Moonshine Tiny | 2/2 | Complete    | 2021-03-04 | - |
| 19.2. Model Selection Revamp | 1/1 | Complete | 2021-03-04 | - |
| 19.3. UI Polish | 3/3 | Complete    | 2021-03-04 | - |
| 20. Bundle CUDA DLLs | 1/1 | Complete   | 2026-03-06 | ✅ |
| 20.1. VAD Chunking | 1/1 | Complete    | 2026-03-06 | - |

Full v1.0 milestone details: `.planning/milestones/v1.0-ROADMAP.md`
Full v1.1 milestone details: `.planning/milestones/v1.1-ROADMAP.md`

### Phase 19: Include distil-large-v3.5 as download option and first-time run

**Goal:** [To be planned]
**Requirements**: TBD
**Depends on:** Phase 17
**Plans:** 1/1 plans complete

Plans:
- [x] TBD (run /gsd:plan-phase 19 to break down) (completed 2021-03-03)

### Phase 19.3: UI polish — tray icons, profile simplification, history panel, double-click settings (INSERTED)

**Goal:** Fix duplicate tray icons, add double-click-to-open-settings, simplify multi-profile system to single vocabulary prompt, and add transcription history panel with click-to-copy
**Requirements**: TRAY-FIX, TRAY-DBLCLICK, PROFILE-SIMPLIFY, HISTORY-PANEL
**Depends on:** Phase 19.2
**Plans:** 3/3 plans complete

Plans:
- [ ] 19.3-01-PLAN.md — Tray duplicate fix (destroy before relaunch) and double-click handler
- [ ] 19.3-02-PLAN.md — Profile simplification (remove multi-profile, add vocabulary prompt field)
- [ ] 19.3-03-PLAN.md — Transcription history backend + frontend panel

### Phase 19.1: Integrate Moonshine Tiny model into main app with VAD chunking and GPU support (INSERTED)

**Goal:** Moonshine Tiny works as a selectable transcription engine in VoiceType — users can download it, select it, and dictate with it, including on recordings longer than 30 seconds via VAD chunking, with GPU acceleration when available
**Requirements**: MOON-01, MOON-02, MOON-03, MOON-04, MOON-05, MOON-06, MOON-07
**Depends on:** Phase 19
**Plans:** 2/2 plans complete

Plans:
- [ ] 19.1-01-PLAN.md — Backend engine core, VAD chunking, pipeline dispatch, download command, IPC wiring, startup loading
- [ ] 19.1-02-PLAN.md — Frontend model selection, first-run card, download handling, human verification

### Phase 19.2: Revamp model selection with benchmark stats, recommend parakeet, remove distil-large-v3.5 (INSERTED)

**Goal:** Remove distil-large-v3.5 from all model surfaces, embed benchmark stats in model descriptions, and make parakeet-tdt-v2-fp32 the universal recommended model — data-driven model selection based on Quick 21-37 benchmark results
**Requirements**: N/A (no formal requirement IDs — cleanup/UX improvement)
**Depends on:** Phase 19.1
**Plans:** 1/1 plans complete

Plans:
- [ ] 19.2-01-PLAN.md — Backend distil removal, benchmark stat descriptions, universal parakeet recommendation, FirstRun cleanup

### Phase 20: Bundle CUDA DLLs in single installer with runtime GPU fallback

**Goal:** Bundle redistributable CUDA DLLs (cudart, cublas, cublasLt) in the single installer so GGML/whisper-rs falls back to CPU at runtime on non-NVIDIA machines. No installer split, no updater changes — one installer works for all users.
**Requirements**: N/A (no formal requirement IDs — scope is CI/config only)
**Depends on:** Phase 19
**Plans:** 1/1 plans complete

Plans:
- [x] 20-01-PLAN.md — CI DLL staging, Tauri bundle.resources map, gitignore, human verification of installer output (completed 2026-03-06)

### Phase 20.1: Generalize VAD chunking for all transcription engines (INSERTED)

**Goal:** Generalize the Moonshine-specific VAD chunking algorithm into an engine-agnostic function and wire it into Whisper and Parakeet engine dispatch, so all three engines handle 60s+ recordings via VAD-based chunk splitting at silence boundaries
**Requirements**: VAD-01, VAD-02, VAD-03, VAD-04, VAD-05, VAD-06
**Depends on:** Phase 20
**Plans:** 1/1 plans complete

Plans:
- [ ] 20.1-01-PLAN.md — Generalize vad_chunk_audio, refactor Moonshine caller, add chunk dispatch to Whisper and Parakeet in pipeline.rs
