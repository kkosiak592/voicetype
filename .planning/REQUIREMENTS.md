# Requirements: VoiceType

**Defined:** 2026-03-08
**Core Value:** Voice dictation must feel instant — sub-1500ms from end-of-speech to text appearing at the cursor, with zero internet dependency.

## v1.5 Requirements

Requirements for v1.5 Prefix Text milestone.

### Prefix

- [x] **PFX-01**: User can enable/disable a prefix toggle in General Settings → Output card
- [x] **PFX-02**: User can set a custom prefix string (e.g., "TEPC: ") via text input
- [x] **PFX-03**: Prefix is prepended to all dictated output when enabled (after ALL CAPS, before trailing space)
- [x] **PFX-04**: Prefix enabled state and text are persisted across app restarts

## Future Requirements

None identified.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Per-app prefix overrides | Option A (global only) chosen for simplicity; can extend to per-app later if needed |
| Suffix text | Not requested; prefix covers the annotation use case |
| Multiple prefix presets | Single custom string is sufficient for current workflow |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| PFX-01 | Phase 27 | Complete |
| PFX-02 | Phase 27 | Complete |
| PFX-03 | Phase 27 | Complete |
| PFX-04 | Phase 27 | Complete |

**Coverage:**
- v1.5 requirements: 4 total
- Mapped to phases: 4
- Unmapped: 0

---
*Requirements defined: 2026-03-08*
*Last updated: 2026-03-08 after roadmap creation*
