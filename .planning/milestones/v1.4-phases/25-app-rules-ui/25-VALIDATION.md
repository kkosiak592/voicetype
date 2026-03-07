---
phase: 25
slug: app-rules-ui
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-07
---

# Phase 25 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust cargo test (backend); no frontend test framework |
| **Config file** | Cargo.toml |
| **Quick run command** | `cd src-tauri && cargo test -- --test-threads=1` |
| **Full suite command** | `cd src-tauri && cargo test -- --test-threads=1` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cd src-tauri && cargo test -- --test-threads=1`
- **After every plan wave:** Run `cd src-tauri && cargo test -- --test-threads=1` + manual UI walkthrough
- **Before `/gsd:verify-work`:** Full suite must be green + all 4 success criteria manually verified
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 25-01-01 | 01 | 1 | UI-01 | manual | N/A -- sidebar registration requires Tauri webview | N/A | pending |
| 25-01-02 | 01 | 1 | UI-02 | manual | N/A -- frontend rendering | N/A | pending |
| 25-01-03 | 01 | 1 | UI-03 | manual | N/A -- requires Win32 + webview | N/A | pending |
| 25-01-04 | 01 | 1 | OVR-01 | manual | N/A -- frontend UX dropdown | N/A | pending |
| 25-01-05 | 01 | 1 | UI-05 | manual | N/A -- frontend + backend integration | N/A | pending |

*Status: pending / green / red / flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. Backend commands are already unit-tested in `src-tauri/src/foreground.rs` (lines 186-331). No frontend test framework exists in the project -- adding one is out of scope for this phase.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Sidebar page accessible | UI-01 | Requires Tauri webview rendering | Open settings, verify "App Rules" appears after Dictionary in sidebar, click to navigate |
| View configured rules | UI-02 | Frontend rendering in webview | Add rules via detect, verify two-line rows with exe name + window title |
| Detect Active App with countdown | UI-03 | Requires Win32 foreground detection + webview | Click "Detect Active App", switch to target app within 3s, verify countdown + app added |
| Three-state toggle | OVR-01 | Frontend UX interaction | Click dropdown on rule row, verify Inherit/Force ON/Force OFF options, verify Inherit shows global state |
| Remove app from list | UI-05 | Frontend + backend integration | Click X on rule row, verify immediate removal, verify rule gone on page reload |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
