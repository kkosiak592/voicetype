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

- [ ] **Phase 15: Hook Module** - Install WH_KEYBOARD_LL on a dedicated thread, implement modifier state machine with debounce and Start menu suppression, wire hold-to-talk end-to-end
- [ ] **Phase 16: Rebind and Coexistence** - Route modifier-only combos through the hook and standard combos through tauri-plugin-global-shortcut; surface hook failure to user
- [ ] **Phase 17: Frontend Capture UI** - Accept and display modifier-only combos in the hotkey capture dialog and settings panel
- [ ] **Phase 18: Integration and Distribution** - Verify behavior across all critical runtime conditions and confirm VirusTotal clean on signed v1.2 binary

## Phase Details

### Phase 11: Signing & Repo Setup
**Goal**: The cryptographic foundation and distribution backend are in place — signing keys exist, the repo is public, and secrets are stored where CI/CD can reach them
**Depends on**: Phase 8 (v1.0 complete)
**Requirements**: UPD-01, CICD-06, REL-01
**Success Criteria** (what must be TRUE):
  1. Ed25519 keypair exists: private key stored in GitHub Actions secrets, public key embedded in tauri.conf.json
  2. Source code is accessible at a public GitHub repository URL
  3. GitHub repo secrets contain TAURI_SIGNING_PRIVATE_KEY and TAURI_SIGNING_PRIVATE_KEY_PASSWORD
  4. Running `tauri signer verify` against a test artifact with the public key succeeds
**Plans**: 1 plan

Plans:
- [x] 11-01-PLAN.md — Generate Ed25519 keypair, push source to public GitHub repo, store signing secrets, verify round-trip signing

### Phase 12: Plugin Integration
**Goal**: The app checks for updates on launch and guides the user through download, install, and relaunch without blocking normal use
**Depends on**: Phase 11
**Requirements**: UPD-02, UPD-03, UPD-04, UPD-05, UPD-06, UPD-07, REL-02
**Success Criteria** (what must be TRUE):
  1. App silently checks for updates on launch; if none available, starts normally with no visible delay
  2. When an update is available, a non-blocking notification appears showing the new version number and release notes
  3. User can initiate download from the notification and see a progress indicator during download
  4. After download completes, app installs the update and relaunches automatically
  5. tauri.conf.json updater endpoint points at the GitHub Releases latest.json URL
**Plans**: 2 plans

Plans:
- [x] 12-01: Add tauri-plugin-updater and tauri-plugin-process dependencies and register in Rust backend
- [x] 12-02: Implement update check, notification UI, download progress, and auto-relaunch in frontend

### Phase 13: CI/CD Pipeline
**Goal**: Pushing a version tag triggers a fully automated pipeline that produces a signed Windows installer, a valid latest.json, and a published GitHub Release — with no manual steps
**Depends on**: Phase 12
**Requirements**: CICD-01, CICD-02, CICD-03, CICD-04, CICD-05
**Success Criteria** (what must be TRUE):
  1. Pushing a tag matching `v*` to GitHub triggers the Actions workflow automatically
  2. Workflow produces a signed NSIS installer (.exe) for Windows
  3. Workflow generates a valid latest.json with correct download URLs and Ed25519 signature
  4. A GitHub Release is created with the installer, latest.json, and release notes attached as assets
  5. An existing v1.0 install can receive and verify the update from the published release
**Plans**: 1 plan

Plans:
- [x] 13-01-PLAN.md — Create GitHub Actions release workflow with CUDA/LLVM build environment, tauri-action signing, and latest.json upload

### Phase 14: Release Workflow
**Goal**: Any future release requires only a version bump, commit, tag, and push — the process is documented and the changelog format is consistent
**Depends on**: Phase 13
**Requirements**: REL-03, REL-04
**Success Criteria** (what must be TRUE):
  1. A documented release runbook exists listing the exact commands to cut a release
  2. A changelog/release notes template exists that produces consistent GitHub Release descriptions
  3. Following the runbook from a clean state produces a working release with no gaps or ambiguity
**Plans**: 1 plan

Plans:
- [x] 14-01-PLAN.md — Create RELEASING.md runbook and CHANGELOG.md template

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
**Plans**: TBD

### Phase 16: Rebind and Coexistence
**Goal**: Changing the hotkey in settings correctly switches between the hook backend and tauri-plugin-global-shortcut at runtime, with no double-firing, and surfaces hook installation failure as a visible status in settings
**Depends on**: Phase 15
**Requirements**: INT-02, INT-03
**Success Criteria** (what must be TRUE):
  1. Changing the hotkey from Ctrl+Win to a standard combo (e.g. Ctrl+Shift+V) in settings stops the hook for that combo and registers it via tauri-plugin-global-shortcut — both activate dictation correctly with no double-firing
  2. Changing back from a standard hotkey to Ctrl+Win unregisters the standard hotkey and re-enables the hook path — no overlap period where both fire
  3. If WH_KEYBOARD_LL installation fails at startup, the settings panel shows a "hook unavailable, using fallback" status and the standard hotkey continues to work
**Plans**: TBD

### Phase 17: Frontend Capture UI
**Goal**: The hotkey capture dialog and settings panel fully support modifier-only combos — users can configure Ctrl+Win without needing to press a letter key, and the display reads naturally
**Depends on**: Phase 15
**Requirements**: UI-01, UI-02
**Success Criteria** (what must be TRUE):
  1. Opening the hotkey capture dialog and pressing only Ctrl+Win (no additional key) registers as a valid hotkey and saves without error
  2. The settings panel displays the saved modifier-only combo as "Ctrl + Win" — not an error state, empty field, or raw key code
  3. The existing capture flow for standard hotkeys (letter key + modifiers) continues to work unchanged alongside the new modifier-only path
**Plans**: TBD

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
| 1. Foundation | v1.0 | 3/3 | Complete | 2026-02-27 |
| 2. Audio + Whisper | v1.0 | 3/3 | Complete | 2026-02-28 |
| 3. Core Pipeline | v1.0 | 2/2 | Complete | 2026-02-28 |
| 4. Pill Overlay | v1.0 | 2/2 | Complete | 2026-02-28 |
| 4.1 Premium Pill UI | v1.0 | 2/2 | Complete | 2026-02-28 |
| 5. VAD + Toggle Mode | v1.0 | 2/2 | Complete | 2026-03-01 |
| 6. Vocabulary + Settings | v1.0 | 4/4 | Complete | 2026-03-01 |
| 6.1 Fix Tray Icons | v1.0 | 2/2 | Complete | 2026-03-01 |
| 7. Distribution | v1.0 | 3/3 | Complete | 2026-03-01 |
| 8. Parakeet TDT + Latency | v1.0 | 3/3 | Complete | 2026-03-02 |
| 11. Signing & Repo Setup | v1.1 | 1/1 | Complete | 2026-03-02 |
| 12. Plugin Integration | v1.1 | 2/2 | Complete | 2026-03-02 |
| 13. CI/CD Pipeline | v1.1 | 1/1 | Complete | 2026-03-02 |
| 14. Release Workflow | v1.1 | 1/1 | Complete | 2026-03-02 |
| 15. Hook Module | v1.2 | 0/? | Not started | - |
| 16. Rebind and Coexistence | v1.2 | 0/? | Not started | - |
| 17. Frontend Capture UI | v1.2 | 0/? | Not started | - |
| 18. Integration and Distribution | v1.2 | 0/? | Not started | - |

Full v1.0 milestone details: `.planning/milestones/v1.0-ROADMAP.md`
