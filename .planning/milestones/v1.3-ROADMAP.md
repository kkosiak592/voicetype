# Roadmap: VoiceType

## Milestones

- ✅ **v1.0 MVP** — Phases 1-8 + 4.1, 6.1 (shipped 2026-03-02)
- ✅ **v1.1 Auto-Updates & CI/CD** — Phases 11-14 (shipped 2026-03-02)
- ✅ **v1.2 Keyboard Hook** — Phases 15-20.1 (shipped 2026-03-07)
- **v1.3 Clipboard Simplification** — Phase 22 (in progress)

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

- [x] Phase 11: Signing & Repo Setup (1/1 plan) — completed 2026-03-02
- [x] Phase 12: Plugin Integration (1/1 plan) — completed 2026-03-02
- [x] Phase 13: CI/CD Pipeline (2/2 plans) — completed 2026-03-02
- [x] Phase 14: Release Workflow (1/1 plan) — completed 2026-03-02

</details>

<details>
<summary>✅ v1.2 Keyboard Hook (Phases 15-20.1) — SHIPPED 2026-03-07</summary>

- [x] Phase 15: Hook Module (3/3 plans) — completed 2026-03-03
- [x] Phase 16: Rebind and Coexistence (2/2 plans) — completed 2026-03-03
- [x] Phase 17: Frontend Capture UI (1/1 plan) — completed 2026-03-03
- [x] Phase 18: Integration and Distribution — VOIDED (moved to Phase 21, deferred)
- [x] Phase 19: Distil-large-v3.5 (1/1 plan) — completed 2026-03-03
- [x] Phase 19.1: Moonshine Tiny (2/2 plans) — completed 2026-03-04 (INSERTED)
- [x] Phase 19.2: Model Selection Revamp (1/1 plan) — completed 2026-03-04 (INSERTED)
- [x] Phase 19.3: UI Polish (3/3 plans) — completed 2026-03-04 (INSERTED)
- [x] Phase 20: Bundle CUDA DLLs (1/1 plan) — completed 2026-03-06
- [x] Phase 20.1: VAD Chunking (1/1 plan) — completed 2026-03-06 (INSERTED)

</details>

### v1.3 Clipboard Simplification (In Progress)

**Milestone Goal:** Remove clipboard save/restore logic from inject_text -- after transcription, clipboard simply contains the transcription text, matching standard dictation tool behavior.

- [x] **Phase 22: Clipboard Save/Restore Removal** - Remove save, restore, and post-paste sleep from inject_text (completed 2026-03-07)

## Phase Details

### Phase 22: Clipboard Save/Restore Removal
**Goal**: Transcription text stays on clipboard after injection, matching standard dictation tool behavior
**Depends on**: Nothing (standalone milestone)
**Requirements**: CLIP-01, CLIP-02, CLIP-03
**Success Criteria** (what must be TRUE):
  1. After dictating and pasting, the clipboard contains the transcription text (verifiable via Ctrl+V in any app)
  2. The 80ms post-paste delay is gone -- injection completes faster with no observable regression in paste reliability
  3. The inject_text doc comment accurately describes the simplified clipboard flow (set, verify, paste)
  4. Existing clipboard verification retry loop and 150ms pre-paste delay remain functional (no collateral removal)
**Plans**: 1 plan

Plans:
- [ ] 22-01-PLAN.md — Remove clipboard save/restore, post-paste sleep, and update doc comment

## Progress

**Execution Order:**
Phase 22

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1-8 | v1.0 | 26/26 | Complete | 2026-03-02 |
| 11-14 | v1.1 | 5/5 | Complete | 2026-03-02 |
| 15-20.1 | v1.2 | 15/15 | Complete | 2026-03-07 |
| 22. Clipboard Save/Restore Removal | 1/1 | Complete    | 2026-03-07 | - |

Full v1.0 milestone details: `.planning/milestones/v1.0-ROADMAP.md`
Full v1.1 milestone details: `.planning/milestones/v1.1-ROADMAP.md`
Full v1.2 milestone details: `.planning/milestones/v1.2-ROADMAP.md`
