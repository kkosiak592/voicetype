# Phase 3: Core Pipeline - Context

**Gathered:** 2026-02-27
**Status:** Ready for planning

<domain>
## Phase Boundary

End-to-end hold-to-talk dictation loop: hold hotkey, speak, release, transcribed text appears at cursor. Wires together Phase 1 hotkey and Phase 2 audio/whisper into a working pipeline with clipboard-based text injection. No overlay UI (Phase 4), no VAD (Phase 5), no vocabulary corrections (Phase 6).

</domain>

<decisions>
## Implementation Decisions

### Text formatting
- Trust whisper's sentence structure output (capitalization, punctuation) — no additional sentence formatting on top
- Trim leading whitespace from whisper output before injection
- Append trailing space after injected text to bridge consecutive dictations naturally
- Hallucination filtering: Claude's discretion on whether to add lightweight known-pattern stripping (repeated phrases, "Thank you for watching" artifacts) or defer entirely to Phase 5 VAD gating

### Multi-dictation behavior
- Each dictation is an independent insert at current cursor position — no accumulation buffer
- Block new recording while pipeline is processing (hotkey ignored during whisper inference + injection) — prevents race conditions and clipboard conflicts
- Always inject into whatever app has focus when hotkey is released — no target app tracking
- No cooldown between dictations — ready for next as soon as injection + clipboard restore completes

### Error & empty handling
- Empty/whitespace-only whisper results: silent discard, no clipboard touch, return to idle
- Clipboard restore failure: log the failure, move on — text was already injected, clipboard loss is a known edge case
- Paste failure: best-effort, no retry — user will notice and re-dictate
- Whisper inference errors: log and return to idle silently

### Pre-overlay feedback
- System tray icon changes to indicate state — three distinct states:
  - **Idle**: normal app icon
  - **Recording**: active/red icon while hotkey is held
  - **Processing**: different icon/spinner while whisper runs after release
- No audio cues — silent operation, tray icon is the only feedback
- These three states map directly to the Phase 4 overlay states later

### Claude's Discretion
- Whether to add lightweight hallucination filtering now or defer to Phase 5 VAD
- Tray tooltip showing last transcription result (useful for debugging vs unnecessary clutter)
- Text command expansion (e.g., "new line" → newline) — decide if any quick wins are worth adding without overcomplicating

</decisions>

<specifics>
## Specific Ideas

- Consecutive dictations should feel like natural sentence continuation — trailing space bridges them
- The tray icon state machine (idle/recording/processing) is deliberately designed to map 1:1 to Phase 4 overlay states
- User expects immediate readiness after each dictation — no artificial delays

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 03-core-pipeline*
*Context gathered: 2026-02-27*
