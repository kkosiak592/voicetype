# Requirements: VoiceType

**Defined:** 2026-03-07
**Core Value:** Voice dictation must feel instant — sub-1500ms from end-of-speech to text appearing at the cursor, with zero internet dependency.

## v1.3 Requirements

Requirements for clipboard simplification release. Each maps to roadmap phases.

### Clipboard Simplification

- [ ] **CLIP-01**: Transcription replaces clipboard content after injection (no save/restore)
- [ ] **CLIP-02**: Post-paste 80ms sleep removed (only needed for restore timing)
- [ ] **CLIP-03**: inject_text doc comment updated to reflect simplified sequence

## Future Requirements

Deferred to future release. Tracked but not in current roadmap.

### Privacy

- **PRIV-01**: ExcludeClipboardContentFromMonitorProcessing to hide transcriptions from clipboard managers (if users report privacy concerns)

### Text Input

- **TINP-01**: TSF/IME direct text insertion (bypass clipboard entirely, like Windows Voice Typing)

## Out of Scope

| Feature | Reason |
|---------|--------|
| Clipboard restore as opt-in toggle | Non-standard behavior; history panel covers recovery use case |
| TSF/IME text insertion | High complexity, clipboard paste works in 95% of apps |
| Clipboard format preservation (images/rich text) | arboard is text-only; not worse than current behavior |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| CLIP-01 | Phase 22 | Pending |
| CLIP-02 | Phase 22 | Pending |
| CLIP-03 | Phase 22 | Pending |

**Coverage:**
- v1.3 requirements: 3 total
- Mapped to phases: 3
- Unmapped: 0 ✓

---
*Requirements defined: 2026-03-07*
*Last updated: 2026-03-07 after initial definition*
