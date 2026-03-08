# Roadmap: VoiceType

## Milestones

- ✅ **v1.0 MVP** — Phases 1-8 + 4.1, 6.1 (shipped 2026-03-02)
- ✅ **v1.1 Auto-Updates & CI/CD** — Phases 11-14 (shipped 2026-03-02)
- ✅ **v1.2 Keyboard Hook** — Phases 15-20.1 (shipped 2026-03-07)
- ✅ **v1.3 Clipboard Simplification** — Phase 22 (shipped 2026-03-07)
- ✅ **v1.4 Per-App Settings** — Phases 23-26 (shipped 2026-03-07)
- 🚧 **v1.5 Prefix Text** — Phase 27 (in progress)

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

<details>
<summary>✅ v1.3 Clipboard Simplification (Phase 22) — SHIPPED 2026-03-07</summary>

- [x] Phase 22: Clipboard Save/Restore Removal (1/1 plan) — completed 2026-03-07

</details>

<details>
<summary>✅ v1.4 Per-App Settings (Phases 23-26) — SHIPPED 2026-03-07</summary>

- [x] Phase 23: Foreground Detection Backend (2/2 plans) — completed 2026-03-07
- [x] Phase 24: Pipeline Override Integration (1/1 plan) — completed 2026-03-07
- [x] Phase 25: App Rules UI (1/1 plan) — completed 2026-03-07
- [x] Phase 26: Process Dropdown (1/1 plan) — completed 2026-03-07

</details>

### v1.5 Prefix Text (In Progress)

**Milestone Goal:** Add a toggleable prefix string that gets prepended to all dictated output, for annotation use cases like shop drawing review.

- [ ] **Phase 27: Prefix Text** - Global prefix toggle with custom text, pipeline integration, and persistence

## Phase Details

### Phase 27: Prefix Text
**Goal**: Users can prepend a configurable prefix string to all dictated output
**Depends on**: Phase 26 (existing settings and pipeline infrastructure)
**Requirements**: PFX-01, PFX-02, PFX-03, PFX-04
**Success Criteria** (what must be TRUE):
  1. User can toggle prefix on/off from General Settings Output card and the toggle state is reflected immediately
  2. User can type a custom prefix string (e.g., "TEPC: ") and see it applied to the next dictation
  3. Dictated text is injected with the prefix prepended when enabled (and without prefix when disabled)
  4. Prefix is applied after ALL CAPS formatting (e.g., ALL CAPS + prefix yields "TEPC: THIS IS A NOTE")
  5. Prefix toggle state and text survive app restart
**Plans**: 1 plan

Plans:
- [ ] 27-01-PLAN.md — Backend prefix fields, IPC commands, pipeline integration, and frontend UI

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1-8 | v1.0 | 26/26 | Complete | 2026-03-02 |
| 11-14 | v1.1 | 5/5 | Complete | 2026-03-02 |
| 15-20.1 | v1.2 | 15/15 | Complete | 2026-03-07 |
| 22 | v1.3 | 1/1 | Complete | 2026-03-07 |
| 23-26 | v1.4 | 5/5 | Complete | 2026-03-07 |
| 27 | v1.5 | 0/1 | Not started | - |

Full v1.0 milestone details: `.planning/milestones/v1.0-ROADMAP.md`
Full v1.1 milestone details: `.planning/milestones/v1.1-ROADMAP.md`
Full v1.2 milestone details: `.planning/milestones/v1.2-ROADMAP.md`
Full v1.3 milestone details: `.planning/milestones/v1.3-ROADMAP.md`
Full v1.4 milestone details: `.planning/milestones/v1.4-ROADMAP.md`
