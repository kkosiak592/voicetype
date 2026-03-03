# Roadmap: VoiceType

## Milestones

- ✅ **v1.0 MVP** — Phases 1-8 + 4.1, 6.1 (shipped 2026-03-02)
- ✅ **v1.1 Auto-Updates & CI/CD** — Phases 11-14 (shipped 2026-03-02)
- 🚧 **v1.2 Keyboard Hook** — Phases 15-18 (in progress)

## Phases

<details>
<summary>✅ v1.0 MVP (Phases 1-8) — SHIPPED 2026-03-02</summary>

- [x] Phase 1: Foundation (3/3 plans) — completed 2026-02-27
- [x] Phase 2: Audio + Whisper (3/3 plans) — completed 2026-02-28
- [x] Phase 3: Core Pipeline (2/2 plans) — completed 2026-02-28
- [x] Phase 4: Pill Overlay (2/2 plans) — completed 2026-02-28
- [x] Phase 4.1: Premium Pill UI (2/2 plans) — completed 2026-02-28 (INSERTED)
- [x] Phase 5: VAD + Toggle Mode (2/2 plans) — completed 2026-03-01
- [x] Phase 6: Vocabulary + Settings (4/4 plans) — completed 2026-03-01
- [x] Phase 6.1: Fix Tray Icons (2/2 plans) — completed 2026-03-01 (INSERTED)
- [x] Phase 7: Distribution (3/3 plans) — completed 2026-03-01
- [x] Phase 8: Parakeet TDT + Latency (3/3 plans) — completed 2026-03-02

</details>

<details>
<summary>✅ v1.1 Auto-Updates & CI/CD (Phases 11-14) — SHIPPED 2026-03-02</summary>

- [x] **Phase 11: Signing & Repo Setup** - Generate Ed25519 keypair, push source to public GitHub repo, configure secrets (completed 2026-03-02)
- [x] **Phase 12: Plugin Integration** - Add updater plugin to app, implement update check UI with progress and relaunch (completed 2026-03-02)
- [x] **Phase 13: CI/CD Pipeline** - GitHub Actions workflow that builds, signs, and publishes releases on tag push (completed 2026-03-02)
- [x] **Phase 14: Release Workflow** - Document release process and changelog template; verify end-to-end update delivery (completed 2026-03-02)

</details>

### 🚧 v1.2 Keyboard Hook (In Progress)

**Milestone Goal:** Replace tauri-plugin-global-shortcut with a custom WH_KEYBOARD_LL low-level keyboard hook, enabling Ctrl+Win modifier-only hotkey activation with debounce for reliable key ordering.

- [x] **Phase 15: Hook Module** - Install WH_KEYBOARD_LL on a dedicated thread, implement modifier state machine with debounce and Start menu suppression, wire hold-to-talk end-to-end (completed 2026-03-03)
- [x] **Phase 16: Rebind and Coexistence** - Route modifier-only combos through the hook and standard combos through tauri-plugin-global-shortcut; surface hook failure to user (completed 2026-03-03)
- [x] **Phase 17: Frontend Capture UI** - Accept and display modifier-only combos in the hotkey capture dialog and settings panel (completed 2026-03-03)
- [ ] **Phase 18: Integration and Distribution** - Verify behavior across all critical runtime conditions and confirm VirusTotal clean on signed v1.2 binary

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

### Phase 18: Integration and Distribution
**Goal**: All v1.2 behavior is verified under real runtime conditions that unit tests cannot cover, and the signed binary is confirmed safe for distribution
**Depends on**: Phase 16, Phase 17
**Requirements**: DIST-01
**Success Criteria** (what must be TRUE):
  1. Alt+Tab away from VoiceType while holding Ctrl, then release Ctrl — no phantom recording session starts on return, confirming modifier state desync recovery
  2. The signed v1.2 binary submitted to VirusTotal shows no new detections relative to the v1.1 baseline — any new detection is a blocking issue before distribution
  3. On both Windows 10 and Windows 11 machines, Ctrl+Win activates dictation without opening the Start menu, and Win key alone continues to open the Start menu
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 15 → 16 → 17 (parallelizable with 16 after Phase 15) → 18

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 15. Hook Module | v1.2 | Complete    | 2026-03-03 | 2026-03-03 |
| 16. Rebind and Coexistence | 2/2 | Complete    | 2026-03-03 | - |
| 17. Frontend Capture UI | 1/1 | Complete    | 2026-03-03 | - |
| 18. Integration and Distribution | v1.2 | 0/? | Not started | - |

Full v1.0 milestone details: `.planning/milestones/v1.0-ROADMAP.md`
Full v1.1 milestone details: `.planning/milestones/v1.1-ROADMAP.md`

### Phase 19: Include distil-large-v3.5 as download option and first-time run

**Goal:** [To be planned]
**Requirements**: TBD
**Depends on:** Phase 18
**Plans:** 1/1 plans complete

Plans:
- [ ] TBD (run /gsd:plan-phase 19 to break down)

### Phase 20: Implement dual CPU/GPU installers with variant-specific auto-updates

**Goal:** [To be planned]
**Requirements**: TBD
**Depends on:** Phase 19
**Plans:** 0 plans

Plans:
- [ ] TBD (run /gsd:plan-phase 20 to break down)
