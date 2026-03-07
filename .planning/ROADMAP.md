# Roadmap: VoiceType

## Milestones

- ✅ **v1.0 MVP** — Phases 1-8 + 4.1, 6.1 (shipped 2026-03-02)
- ✅ **v1.1 Auto-Updates & CI/CD** — Phases 11-14 (shipped 2026-03-02)
- ✅ **v1.2 Keyboard Hook** — Phases 15-20.1 (shipped 2026-03-07)
- ✅ **v1.3 Clipboard Simplification** — Phase 22 (shipped 2026-03-07)
- 🚧 **v1.4 Per-App Settings** — Phases 23-26 (in progress)

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

### 🚧 v1.4 Per-App Settings (In Progress)

**Milestone Goal:** Enable per-application setting overrides, starting with ALL CAPS, detected automatically based on the foreground window at injection time.

- [x] **Phase 23: Foreground Detection Backend** - Win32 detection module, data model, persistence, and Tauri commands (completed 2026-03-07)
- [x] **Phase 24: Pipeline Override Integration** - Wire per-app override resolution into the transcription pipeline at injection time (completed 2026-03-07)
- [x] **Phase 25: App Rules UI** - Sidebar page with rules list, detect button, three-state toggles, and rule management (completed 2026-03-07)
- [ ] **Phase 26: Process Dropdown** - Searchable dropdown of running processes for adding apps without detection

## Phase Details

### Phase 23: Foreground Detection Backend
**Goal**: The app can identify which application is in the foreground and store per-app rules
**Depends on**: Nothing (first phase of v1.4)
**Requirements**: DET-01, DET-02, DET-03, OVR-04
**Success Criteria** (what must be TRUE):
  1. Calling `detect_foreground_app` from the frontend returns the lowercase exe name of the currently focused application (e.g., "acad.exe")
  2. Detection handles elevated processes gracefully -- returns a fallback result instead of crashing or hanging
  3. App rules added to settings.json persist across application restarts and are loaded on startup
  4. UWP apps (e.g., Calculator, Windows Store apps) resolve to their real process name, not "applicationframehost.exe"
**Plans:** 2/2 plans complete
Plans:
- [ ] 23-01-PLAN.md — Foreground detection module with types, Win32 API chain, UWP resolution, and unit tests
- [ ] 23-02-PLAN.md — Wire into lib.rs: Tauri commands, managed state, startup loading, persistence

### Phase 24: Pipeline Override Integration
**Goal**: Per-app ALL CAPS overrides take effect automatically at text injection time
**Depends on**: Phase 23
**Requirements**: OVR-02, OVR-03
**Success Criteria** (what must be TRUE):
  1. Dictating into an app with a "Force ON" rule produces ALL CAPS text even when the global toggle is OFF
  2. Dictating into an app with a "Force OFF" rule produces normal-case text even when the global toggle is ON
  3. Dictating into an app with no rule uses the global ALL CAPS setting
**Plans:** 1/1 plans complete
Plans:
- [ ] 24-01-PLAN.md — resolve_all_caps() function with unit tests + pipeline wiring

### Phase 25: App Rules UI
**Goal**: Users can manage per-app overrides through a dedicated settings page
**Depends on**: Phase 24
**Requirements**: UI-01, UI-02, UI-03, UI-05, OVR-01
**Success Criteria** (what must be TRUE):
  1. "App Rules" page is accessible from the sidebar navigation alongside existing pages
  2. User can click "Detect Active App", switch to target app within 3 seconds, and have that app auto-added to the rules list
  3. Each app in the rules list shows its name and a three-state ALL CAPS toggle (Inherit / Force ON / Force OFF)
  4. User can remove an app from the rules list
**Plans:** 1/1 plans complete
Plans:
- [ ] 25-01-PLAN.md — Sidebar registration, rules list with three-state dropdown, detect flow with countdown, delete

### Phase 26: Process Dropdown
**Goal**: Users can add apps from a searchable list without using the detect flow
**Depends on**: Phase 25
**Requirements**: UI-04
**Success Criteria** (what must be TRUE):
  1. User can open a searchable dropdown showing currently running processes with window titles
  2. Selecting a process from the dropdown adds it to the rules list
  3. Dropdown filters results as the user types, showing only matching process names
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 23 -> 24 -> 25 -> 26

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1-8 | v1.0 | 26/26 | Complete | 2026-03-02 |
| 11-14 | v1.1 | 5/5 | Complete | 2026-03-02 |
| 15-20.1 | v1.2 | 15/15 | Complete | 2026-03-07 |
| 22 | v1.3 | 1/1 | Complete | 2026-03-07 |
| 23. Foreground Detection Backend | 2/2 | Complete    | 2026-03-07 | - |
| 24. Pipeline Override Integration | 1/1 | Complete    | 2026-03-07 | - |
| 25. App Rules UI | 1/1 | Complete    | 2026-03-07 | - |
| 26. Process Dropdown | v1.4 | 0/? | Not started | - |

Full v1.0 milestone details: `.planning/milestones/v1.0-ROADMAP.md`
Full v1.1 milestone details: `.planning/milestones/v1.1-ROADMAP.md`
Full v1.2 milestone details: `.planning/milestones/v1.2-ROADMAP.md`
Full v1.3 milestone details: `.planning/milestones/v1.3-ROADMAP.md`
