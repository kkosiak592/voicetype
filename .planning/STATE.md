---
gsd_state_version: 1.0
milestone: v1.1
milestone_name: Auto-Updates & CI/CD
status: planning
last_updated: "2026-03-02T20:17:12.278Z"
progress:
  total_phases: 4
  completed_phases: 3
  total_plans: 5
  completed_plans: 4
---

# Session State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-02)

**Core value:** Voice dictation must feel instant — sub-1500ms from end-of-speech to text appearing at the cursor, with zero internet dependency.
**Current focus:** Phase 11 — Signing & Repo Setup

## Position

**Milestone:** v1.1 Auto-Updates & CI/CD
**Phase:** 13 of 14 (CI/CD Pipeline)
**Plan:** 01 (complete)
**Status:** Ready to plan
Last activity: 2026-03-02 - Completed Phase 13 Plan 01: CI/CD Pipeline — GitHub Actions release workflow with CUDA + LLVM build environment, Ed25519 signing, NSIS installer, latest.json generation

Progress: [████████░░] 80% (4/5 plans)

## Accumulated Context

### Decisions

- v1.1: tauri-plugin-updater + GitHub Releases chosen (Option A) — zero cost, official Tauri approach, best UX for <20 users
- v1.1: Public GitHub repo required — updater needs unauthenticated access to release assets
- v1.1: Ed25519 signing — private key stored only in GitHub secrets + local backup; loss means existing installs cannot receive future updates
- 11-01: bundle.createUpdaterArtifacts set to "v1Compatible" for Tauri 2 backward-compatible signature format
- 11-01: tauri-plugin-updater NOT added to Cargo.toml in Phase 11 — Phase 12 scope; pubkey/endpoint config only here
- 11-01: Ed25519 private key lives in ~/.voicetype-signing.key — outside repo, no .gitignore entry needed
- 12-01: tauri-plugin-updater registered in setup() not on Builder — requires app handle to read updater config from tauri.conf.json
- 12-01: Rust check_for_update command is check-only; download/install handled by JS plugin API (check().downloadAndInstall())
- 12-02: JS plugin API handles download (not Rust IPC) — enables progress callbacks; Update ref lost on unmount so mid-download close resets banner to 'available'
- 12-02: Updater config belongs under top-level plugins key in tauri.conf.json (Tauri v2 format), not under app section
- 12-02: Tray menus in Tauri 2 must be rebuilt from scratch; set_tray_update_indicator creates new Menu and calls set_menu()
- 13-01: Jimver/cuda-toolkit minimal sub-packages (nvcc, cudart, cublas, cublas_dev, thrust, visual_studio_integration) avoids 4 GB full toolkit download in CI
- 13-01: CMAKE_CUDA_ARCHITECTURES=61;75;86;89 targets Pascal/Turing/Ampere/Ada in one binary; tagName uses github.ref_name directly (tag is release source of truth)

### Pending Todos

1. Investigate microphone icon persisting in system tray (area: ui)
2. Implement sub-500ms transcription latency improvements (area: backend)
3. Simplify profiles to shared dictionary and editable prompts (area: ui)

### Blockers/Concerns

None active. (Ed25519 private key backup completed by user at Task 2 checkpoint — resolved)

### Quick Tasks Completed

| # | Description | Date | Commit | Directory |
|---|-------------|------|--------|-----------|
| 25 | Auto-select recommended model on first startup with DXGI GPU detection | 2026-03-02 | 59551ae | [25-auto-select-recommended-model-on-first-s](./quick/25-auto-select-recommended-model-on-first-s/) |

## Session Continuity

Last session: 2026-03-02
Stopped at: Completed 13-01-PLAN.md (Phase 13 Plan 01: CI/CD Pipeline — release workflow)
Resume file: None
