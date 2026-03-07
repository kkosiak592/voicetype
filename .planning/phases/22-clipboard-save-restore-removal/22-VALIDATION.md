---
phase: 22
slug: clipboard-save-restore-removal
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-07
---

# Phase 22 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust (cargo build) |
| **Config file** | src-tauri/Cargo.toml |
| **Quick run command** | `cargo build --manifest-path src-tauri/Cargo.toml` |
| **Full suite command** | `cargo build --manifest-path src-tauri/Cargo.toml` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo build --manifest-path src-tauri/Cargo.toml`
- **After every plan wave:** Run `cargo build --manifest-path src-tauri/Cargo.toml`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 22-01-01 | 01 | 1 | CLIP-01 | manual-only | `cargo build --manifest-path src-tauri/Cargo.toml` | N/A | ⬜ pending |
| 22-01-02 | 01 | 1 | CLIP-02 | manual-only | `cargo build --manifest-path src-tauri/Cargo.toml` | N/A | ⬜ pending |
| 22-01-03 | 01 | 1 | CLIP-03 | manual-only | `cargo build --manifest-path src-tauri/Cargo.toml` | N/A | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. No test framework setup needed — this phase is pure code removal verified by successful compilation.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Clipboard contains transcription after paste | CLIP-01 | Requires running Windows desktop with active window | 1. Dictate text 2. Ctrl+V in another app 3. Verify pasted text matches transcription |
| No 80ms post-paste delay | CLIP-02 | Requires timing observation on real hardware | 1. Dictate text 2. Observe injection completes faster 3. Verify no paste reliability regression |
| Doc comment describes simplified flow | CLIP-03 | Code review verification | 1. Read inject_text doc comment 2. Verify it describes set -> verify -> paste sequence |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
