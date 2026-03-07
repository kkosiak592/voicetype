---
phase: 24
slug: pipeline-override-integration
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-07
---

# Phase 24 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[cfg(test)]` + `cargo test` |
| **Config file** | None (standard Cargo test runner) |
| **Quick run command** | `cd src-tauri && cargo test --lib -- foreground` |
| **Full suite command** | `cd src-tauri && cargo test --lib` |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cd src-tauri && cargo test --lib -- foreground`
- **After every plan wave:** Run `cd src-tauri && cargo test --lib`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 5 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 24-01-01 | 01 | 1 | OVR-02 | unit | `cd src-tauri && cargo test --lib -- foreground::override_tests` | W0 | pending |
| 24-01-02 | 01 | 1 | OVR-03 | unit | `cd src-tauri && cargo test --lib -- foreground::override_tests::test_unlisted_app` | W0 | pending |
| 24-01-03 | 01 | 1 | OVR-02 | manual | N/A — requires running app with foreground detection | N/A | pending |
| 24-01-04 | 01 | 1 | OVR-03 | manual | N/A — requires running app with foreground detection | N/A | pending |

*Status: pending · green · red · flaky*

---

## Wave 0 Requirements

- [ ] `src-tauri/src/foreground.rs` — add `resolve_all_caps()` function and `override_tests` test module

*Existing infrastructure covers framework installation.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Dictate into app with Force ON rule, get ALL CAPS | OVR-02 | Requires running desktop app with foreground detection | 1. Set Force ON rule for notepad.exe 2. Open Notepad 3. Dictate text 4. Verify ALL CAPS output |
| Dictate into app with Force OFF rule, get normal case | OVR-02 | Requires running desktop app with foreground detection | 1. Set Force OFF rule for notepad.exe 2. Enable global ALL CAPS 3. Open Notepad 4. Dictate text 5. Verify normal-case output |
| Dictate into unlisted app, uses global toggle | OVR-03 | Requires running desktop app with foreground detection | 1. Ensure no rule for target app 2. Toggle global ALL CAPS on/off 3. Dictate into unlisted app 4. Verify behavior matches global setting |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 5s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
