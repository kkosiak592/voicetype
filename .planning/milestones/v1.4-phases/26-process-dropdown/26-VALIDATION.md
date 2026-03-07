---
phase: 26
slug: process-dropdown
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-03-07
---

# Phase 26 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust unit tests (cargo test) |
| **Config file** | Cargo.toml (existing) |
| **Quick run command** | `cargo test --lib -p voice-to-text` |
| **Full suite command** | `cargo test -p voice-to-text` |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --lib -p voice-to-text`
- **After every plan wave:** Run `cargo test -p voice-to-text`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 26-01-01 | 01 | 1 | UI-04 | build | `cargo build -p voice-to-text` | N/A | pending |
| 26-01-02 | 01 | 1 | UI-04 | build | `cargo build -p voice-to-text` | N/A | pending |
| 26-02-01 | 02 | 1 | UI-04 | manual | N/A | N/A | pending |
| 26-02-02 | 02 | 1 | UI-04 | manual | N/A | N/A | pending |

*Status: pending / green / red / flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. No new test framework or stubs needed.

- Cargo test infrastructure already configured
- Build verification sufficient for Win32 API integration code
- Frontend verification is manual (React component + Tauri IPC)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| list_running_processes returns processes with visible windows | UI-04 | Requires live Win32 desktop session with running GUI processes | Open dropdown, verify list shows running apps with window titles, no background services |
| Dropdown search filters by exe name and window title | UI-04 | React component visual behavior | Type partial name, verify list filters; type window title substring, verify match |
| Selecting process adds to rules list with Inherit default | UI-04 | IPC integration requires running Tauri app | Click process in dropdown, verify it appears in rules list with Inherit mode |
| Already-added processes appear dimmed and non-clickable | UI-04 | Visual + interaction behavior | Add a process, reopen dropdown, verify it shows dimmed with "already added" label |
| Dropdown closes on selection and outside click | UI-04 | Interaction behavior | Select item (verify close), click outside (verify close) |

---

## Validation Sign-Off

- [x] All tasks have automated verify or Wave 0 dependencies
- [x] Sampling continuity: build verification after each task
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 10s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
