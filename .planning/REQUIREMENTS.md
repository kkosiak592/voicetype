# Requirements: VoiceType

**Defined:** 2026-03-02
**Core Value:** Voice dictation must feel instant — sub-1500ms from end-of-speech to text appearing at the cursor, with zero internet dependency.

## v1.1 Requirements

Requirements for auto-updates and CI/CD release infrastructure. Each maps to roadmap phases.

### Updater Integration

- [ ] **UPD-01**: App generates Ed25519 signing keypair and configures tauri-plugin-updater with public key
- [ ] **UPD-02**: App registers tauri-plugin-updater and tauri-plugin-process plugins in Rust backend
- [ ] **UPD-03**: App checks for updates on launch by fetching latest.json from GitHub Releases endpoint
- [ ] **UPD-04**: User sees a non-blocking notification when an update is available showing version and release notes
- [ ] **UPD-05**: User can download update with visible progress indication
- [ ] **UPD-06**: App installs update and relaunches automatically after download completes
- [ ] **UPD-07**: Updater capabilities permissions (updater:default, process:allow-restart) are configured

### CI/CD Pipeline

- [ ] **CICD-01**: GitHub Actions workflow triggers on version tag push (v*)
- [ ] **CICD-02**: Workflow builds Windows NSIS installer using tauri-action
- [ ] **CICD-03**: Workflow signs release artifacts with Ed25519 private key from GitHub secrets
- [ ] **CICD-04**: Workflow generates latest.json with correct download URLs and signature
- [ ] **CICD-05**: Workflow creates GitHub Release with installer, latest.json, and release notes
- [ ] **CICD-06**: GitHub repo secrets configured for TAURI_SIGNING_PRIVATE_KEY and password

### Release Infrastructure

- [ ] **REL-01**: Source code pushed to public GitHub repository
- [ ] **REL-02**: tauri.conf.json updater endpoint configured to point at GitHub Releases latest.json
- [ ] **REL-03**: Documented release workflow: version bump → commit → tag → push → automatic build
- [ ] **REL-04**: Changelog/release notes template for consistent release communication

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Distribution

- **DIST-01**: Windows code signing certificate to eliminate SmartScreen warnings
- **DIST-02**: Update channels (stable/beta) for staged rollouts
- **DIST-03**: Delta/differential updates to reduce download size
- **DIST-04**: macOS/Linux build targets in CI/CD pipeline

## Out of Scope

| Feature | Reason |
|---------|--------|
| CrabNebula Cloud hosting | Adds cost (~EUR 9/mo) with no benefit for <20 users |
| Custom self-hosted update server | Unnecessary infrastructure for this scale |
| Auto-update without user prompt | Users should see what's being updated before it happens |
| Private repo with auth tokens | Adds complexity; public repo is acceptable |
| Rollback to previous version | Not needed for initial release; users can manually reinstall |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| UPD-01 | Phase 11 | Pending |
| UPD-02 | Phase 12 | Pending |
| UPD-03 | Phase 12 | Pending |
| UPD-04 | Phase 12 | Pending |
| UPD-05 | Phase 12 | Pending |
| UPD-06 | Phase 12 | Pending |
| UPD-07 | Phase 12 | Pending |
| CICD-01 | Phase 13 | Pending |
| CICD-02 | Phase 13 | Pending |
| CICD-03 | Phase 13 | Pending |
| CICD-04 | Phase 13 | Pending |
| CICD-05 | Phase 13 | Pending |
| CICD-06 | Phase 11 | Pending |
| REL-01 | Phase 11 | Pending |
| REL-02 | Phase 12 | Pending |
| REL-03 | Phase 14 | Pending |
| REL-04 | Phase 14 | Pending |

**Coverage:**
- v1.1 requirements: 17 total
- Mapped to phases: 17
- Unmapped: 0 ✓

---
*Requirements defined: 2026-03-02*
*Last updated: 2026-03-02 after roadmap creation*
