# Requirements: VoiceType

**Defined:** 2026-03-02
**Core Value:** Voice dictation must feel instant — sub-1500ms from end-of-speech to text appearing at the cursor, with zero internet dependency.

## v1.2 Requirements

Requirements for Ctrl+Win modifier-only hotkey activation. Each maps to roadmap phases.

### Hook Infrastructure

- [x] **HOOK-01**: App installs WH_KEYBOARD_LL hook on a dedicated thread with Win32 GetMessage loop
- [x] **HOOK-02**: Hook callback completes in under 5ms using only AtomicBool writes and non-blocking channel sends
- [x] **HOOK-03**: Tauri builder applies DeviceEventFilter::Always so hook fires when Tauri window is focused
- [x] **HOOK-04**: App cleanly uninstalls hook on shutdown via PostThreadMessageW(WM_QUIT) with no dangling hook

### Modifier Detection

- [ ] **MOD-01**: Hook detects Ctrl+Win held simultaneously and sends Pressed event to handle_shortcut()
- [ ] **MOD-02**: Hook detects Ctrl or Win released after combo and sends Released event to handle_shortcut()
- [ ] **MOD-03**: 50ms debounce window allows either key to be pressed first without affecting detection
- [ ] **MOD-04**: Start menu is suppressed when Ctrl+Win combo is active via VK_E8 mask injection
- [ ] **MOD-05**: Win key alone still opens Start menu when not part of Ctrl+Win combo

### Integration

- [ ] **INT-01**: Hold-to-talk works end-to-end with Ctrl+Win (hold to record, release to transcribe)
- [ ] **INT-02**: rebind_hotkey routes modifier-only combos through hook and standard combos through tauri-plugin-global-shortcut
- [ ] **INT-03**: If WH_KEYBOARD_LL installation fails, app falls back to RegisterHotKey and surfaces failure in settings

### Frontend

- [ ] **UI-01**: Hotkey capture dialog accepts Ctrl+Win as a valid modifier-only combo without requiring a letter key
- [ ] **UI-02**: Settings panel displays modifier-only combos as "Ctrl + Win"

### Distribution

- [ ] **DIST-01**: Signed v1.2 binary passes VirusTotal scan with no new detections vs v1.1 baseline

## v1.1 Requirements

All 17 requirements shipped. See `.planning/milestones/v1.1-REQUIREMENTS.md` for archive.

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Hook Enhancements

- **HOOK-05**: Hook persistence across Win+L lock/unlock via WTSRegisterSessionNotification
- **HOOK-06**: Periodic health-check timer detects silent hook removal and reinstalls

### Modifier Enhancements

- **MOD-06**: Left vs right modifier distinction in hotkey binding
- **MOD-07**: Double-tap modifier combo for toggle mode entry
- **MOD-08**: Additional modifier-only combos (double-Ctrl, Shift+Win)

### Distribution

- **DIST-02**: Windows code signing certificate to eliminate SmartScreen warnings
- **DIST-03**: Update channels (stable/beta) for staged rollouts
- **DIST-04**: Delta/differential updates to reduce download size
- **DIST-05**: macOS/Linux build targets in CI/CD pipeline

## Out of Scope

| Feature | Reason |
|---------|--------|
| Suppress all Win key combos (Win+L, Win+D, Win+Tab) | Only suppress when Ctrl+Win combo is active; all other Win usage must pass through |
| Registry-based Win key disable | Requires elevated privileges, survives crash, breaks Win key globally |
| Debounce window under 50ms | Below that, press-order sensitivity causes inconsistent activation |
| Remove tauri-plugin-global-shortcut entirely | Keep as fallback for standard hotkeys and hook failure scenarios |
| CrabNebula Cloud hosting | Adds cost with no benefit for <20 users |
| Auto-update without user prompt | Users should see what's being updated |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| HOOK-01 | Phase 15 | Complete |
| HOOK-02 | Phase 15 | Complete |
| HOOK-03 | Phase 15 | Complete |
| HOOK-04 | Phase 15 | Complete |
| MOD-01 | Phase 15 | Pending |
| MOD-02 | Phase 15 | Pending |
| MOD-03 | Phase 15 | Pending |
| MOD-04 | Phase 15 | Pending |
| MOD-05 | Phase 15 | Pending |
| INT-01 | Phase 15 | Pending |
| INT-02 | Phase 16 | Pending |
| INT-03 | Phase 16 | Pending |
| UI-01 | Phase 17 | Pending |
| UI-02 | Phase 17 | Pending |
| DIST-01 | Phase 18 | Pending |
| UPD-01 | Phase 11 | Complete |
| UPD-02 | Phase 12 | Complete |
| UPD-03 | Phase 12 | Complete |
| UPD-04 | Phase 12 | Complete |
| UPD-05 | Phase 12 | Complete |
| UPD-06 | Phase 12 | Complete |
| UPD-07 | Phase 12 | Complete |
| CICD-01 | Phase 13 | Complete |
| CICD-02 | Phase 13 | Complete |
| CICD-03 | Phase 13 | Complete |
| CICD-04 | Phase 13 | Complete |
| CICD-05 | Phase 13 | Complete |
| CICD-06 | Phase 11 | Complete |
| REL-01 | Phase 11 | Complete |
| REL-02 | Phase 12 | Complete |
| REL-03 | Phase 14 | Complete |
| REL-04 | Phase 14 | Complete |

**Coverage:**
- v1.2 requirements: 15 total
- Mapped to phases: 15
- Unmapped: 0

---
*Requirements defined: 2026-03-02*
*Last updated: 2026-03-02 after v1.2 roadmap creation*
