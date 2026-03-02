# Roadmap: VoiceType

## Milestones

- ✅ **v1.0 MVP** — Phases 1-8 + 4.1, 6.1 (shipped 2026-03-02)
- 🚧 **v1.1 Auto-Updates & CI/CD** — Phases 11-14 (in progress)

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

### 🚧 v1.1 Auto-Updates & CI/CD (In Progress)

**Milestone Goal:** Enable seamless auto-updates via tauri-plugin-updater backed by GitHub Releases, with GitHub Actions CI/CD for automated builds, signing, and release publishing.

- [x] **Phase 11: Signing & Repo Setup** - Generate Ed25519 keypair, push source to public GitHub repo, configure secrets (completed 2026-03-02)
- [x] **Phase 12: Plugin Integration** - Add updater plugin to app, implement update check UI with progress and relaunch (completed 2026-03-02)
- [x] **Phase 13: CI/CD Pipeline** - GitHub Actions workflow that builds, signs, and publishes releases on tag push (completed 2026-03-02)
- [x] **Phase 14: Release Workflow** - Document release process and changelog template; verify end-to-end update delivery (completed 2026-03-02)

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
- [ ] 11-01-PLAN.md — Generate Ed25519 keypair, push source to public GitHub repo, store signing secrets, verify round-trip signing

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
**Plans**: TBD

Plans:
- [ ] 12-01: Add tauri-plugin-updater and tauri-plugin-process dependencies and register in Rust backend
- [ ] 12-02: Implement update check, notification UI, download progress, and auto-relaunch in frontend

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
- [ ] 13-01-PLAN.md — Create GitHub Actions release workflow with CUDA/LLVM build environment, tauri-action signing, and latest.json upload

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
- [ ] 14-01-PLAN.md — Create RELEASING.md runbook and CHANGELOG.md template

## Progress

**Execution Order:**
Phases execute in numeric order: 11 → 12 → 13 → 14

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
| 11. Signing & Repo Setup | 1/1 | Complete    | 2026-03-02 | - |
| 12. Plugin Integration | 2/2 | Complete    | 2026-03-02 | - |
| 13. CI/CD Pipeline | 1/1 | Complete    | 2026-03-02 | - |
| 14. Release Workflow | 1/1 | Complete    | 2026-03-02 | - |

Full v1.0 milestone details: `.planning/milestones/v1.0-ROADMAP.md`
