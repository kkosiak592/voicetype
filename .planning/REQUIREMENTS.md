# Requirements: VoiceType

**Defined:** 2026-03-07
**Core Value:** Voice dictation must feel instant — sub-1500ms from end-of-speech to text appearing at the cursor, with zero internet dependency.

## v1.4 Requirements

Requirements for per-app settings milestone. Each maps to roadmap phases.

### Detection

- [x] **DET-01**: App auto-detects the foreground application at text injection time using Win32 APIs
- [x] **DET-02**: Detection resolves process executable name (e.g., "acad.exe", "OUTLOOK.EXE")
- [x] **DET-03**: Detection falls back to global defaults when process name cannot be resolved (elevated processes, access denied)

### App Rules UI

- [ ] **UI-01**: New "App Rules" sidebar page accessible from the navigation
- [ ] **UI-02**: User can view a list of configured per-app rules with app icons and names
- [ ] **UI-03**: User can add an app via "Detect Active App" button with 3-second countdown
- [ ] **UI-04**: User can add an app via searchable dropdown of currently running processes
- [ ] **UI-05**: User can remove an app from the rules list

### Override System

- [ ] **OVR-01**: Each app rule has a three-state ALL CAPS toggle (Inherit / Force ON / Force OFF)
- [ ] **OVR-02**: Per-app override is applied automatically at injection time when foreground app matches a rule
- [ ] **OVR-03**: Unlisted apps fall back to the global ALL CAPS toggle on the General page
- [ ] **OVR-04**: Per-app rules persist across app restarts via settings.json

## Future Requirements

Deferred to future milestones. Tracked but not in current roadmap.

### Per-App Extensions

- **EXT-01**: Per-app filler removal override
- **EXT-02**: Per-app corrections dictionary override
- **EXT-03**: Per-app engine/model selection

## Out of Scope

| Feature | Reason |
|---------|--------|
| Window-title-based matching | Titles change constantly, are locale-dependent, and break on updates. Exe name is stable. |
| Auto-populate rules for all running apps | Creates huge list of irrelevant processes (svchost, RuntimeBroker). Users add only apps they dictate into. |
| Per-app profile switching (engine, corrections) | Massive scope increase. Engine switching has startup cost. Deferred to future. |
| Regex/wildcard matching for exe names | Over-engineering. Users have 3-5 apps they dictate into. Exact match is sufficient. |
| Real-time foreground monitoring indicator | Continuous polling wastes CPU. Detection only needed at injection time and on "Detect" button click. |
| Browse-for-exe file picker | Users rarely know where executables live. Detect + searchable dropdown covers all use cases. |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| DET-01 | Phase 23 | Complete |
| DET-02 | Phase 23 | Complete |
| DET-03 | Phase 23 | Complete |
| UI-01 | Phase 25 | Pending |
| UI-02 | Phase 25 | Pending |
| UI-03 | Phase 25 | Pending |
| UI-04 | Phase 26 | Pending |
| UI-05 | Phase 25 | Pending |
| OVR-01 | Phase 25 | Pending |
| OVR-02 | Phase 24 | Pending |
| OVR-03 | Phase 24 | Pending |
| OVR-04 | Phase 23 | Pending |

**Coverage:**
- v1.4 requirements: 12 total
- Mapped to phases: 12
- Unmapped: 0

---
*Requirements defined: 2026-03-07*
*Last updated: 2026-03-07 after roadmap creation*
