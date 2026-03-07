# Feature Research: Clipboard Simplification

**Domain:** Voice-to-text dictation tool clipboard behavior
**Researched:** 2026-03-07
**Confidence:** HIGH

---

## Context: What Is Already Built

This is a subsequent milestone. The existing app (v1.2) already has:

- Clipboard paste injection via `inject_text()` using arboard + enigo Ctrl+V
- Pre-transcription clipboard save and post-paste clipboard restore (the code being simplified)
- Clipboard verification retry loop (5 attempts, 25ms each) to handle Chromium WebView races
- 150ms pre-paste delay for Office/Outlook clipboard cache sync
- 80ms post-paste delay solely for restore timing
- Transcription history panel with click-to-copy

**The new milestone (v1.3) simplifies one thing:** Remove clipboard save/restore from `inject_text`, so transcription simply stays on the clipboard after paste injection. This matches standard dictation tool behavior.

---

## Competitor Clipboard Behavior Analysis

### How Major Dictation Tools Handle Text After Transcription

| Tool | Injection Method | Clipboard After Dictation | Clipboard Restore? |
|------|-----------------|--------------------------|-------------------|
| **Windows Voice Typing** | Text Services Framework (direct input API) | Untouched — no clipboard use | N/A |
| **macOS Dictation** | InputMethodKit (direct input API) | Untouched — no clipboard use | N/A |
| **Dragon NaturallySpeaking** | Direct input for supported apps; Dictation Box + Ctrl+V for unsupported | Contains dictated text when clipboard path used | No |
| **Talon** | `insert()` keyboard simulation (default) | Untouched in default path; restored via `clip.capture()` when clipboard path used | Yes, but only on clipboard code path |
| **Superwhisper** (macOS, commercial) | Clipboard paste (default) | Contains transcription (default) | Optional toggle, OFF by default. Restores 3s after paste |
| **WhisperWriter** | pynput keystroke simulation, char-by-char | Untouched — no clipboard use | N/A |
| **OpenWhispr** | Clipboard paste with fallback chain | Contains transcription | No |
| **HyprVoice** (Linux) | ydotool/wtype; clipboard as fallback | Restored on clipboard fallback path | Yes, on fallback path only |

### Key Patterns

Two categories of tools exist:

1. **OS-integrated dictation** (Windows Voice Typing, macOS Dictation, Dragon for supported apps): Use text input APIs that never touch the clipboard. Gold standard UX but requires deep OS integration impossible for third-party tools.

2. **Third-party clipboard-paste tools** (Superwhisper, OpenWhispr, VoiceType): Use clipboard + Ctrl+V because it works universally. In this category, **the standard behavior is: transcription replaces clipboard, no restore.** Superwhisper -- the most polished commercial tool in this space -- defaults to this and makes clipboard restore an opt-in advanced setting.

**Bottom line:** VoiceType's current clipboard restore behavior is non-standard. Every comparable clipboard-paste dictation tool leaves the transcription on the clipboard by default. The restore logic adds complexity and latency for behavior no user expects.

---

## Feature Landscape

### Table Stakes (Users Expect These)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Transcription stays on clipboard after injection | Every clipboard-paste dictation tool works this way. Superwhisper, OpenWhispr, Dragon (clipboard path) all leave transcription on clipboard. Users can re-paste with Ctrl+V. | LOW | Core v1.3 change: remove save/restore logic |
| Reliable clipboard paste across target apps | Ctrl+V must work in VS Code, Chrome, Outlook, Word, Teams, AutoCAD, Bluebeam | N/A | Already exists and working |
| Clipboard verification before paste | Chromium WebView clipboard races are real. Must verify clipboard contains intended text before Ctrl+V. | N/A | Already exists with 5-attempt retry loop. Keep this. |

### Differentiators (Competitive Advantage)

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Transcription history with click-to-copy | Eliminates the main argument for clipboard restore ("what if I lose what I copied?"). Users can always recover past transcriptions. No competitor at this price point (free, local) offers this combined with clipboard paste. | N/A | Already built in v1.2. Makes clipboard restore unnecessary for most users |
| Optional clipboard restore toggle (future) | Superwhisper offers this as an advanced setting. Power users doing heavy copy-paste alongside dictation might want it. | LOW | Keep save/restore code path behind a settings toggle, default OFF. Not needed for v1.3. Add only if users request it |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Clipboard restore as default behavior | "Don't overwrite my clipboard" -- seems courteous | Adds 80ms+ latency for restore timing. Creates race conditions if user pastes quickly after dictation (paste gets old content). Superwhisper's 3-second restore delay exists precisely because instant restore would paste OLD content on quick re-paste. Creates edge cases with non-text clipboard content (images, files, rich text). No comparable tool does this by default. | Leave transcription on clipboard. Users have Win+V clipboard history and transcription history panel |
| Keystroke simulation instead of clipboard | "Don't touch my clipboard at all" | 5ms/char = 500ms for 100 characters. Breaks with special characters and Unicode. WhisperWriter uses this and it is noticeably laggy. Does not work reliably in all apps. | Clipboard paste is the correct approach. Fast, reliable, universal |
| TSF/IME direct text input | "Inject text like Windows Voice Typing" | Requires implementing a Text Services Framework provider -- enormous complexity. Dragon spent years building this. Only works for apps supporting TSF. Complete injection rewrite. | Clipboard paste works in 95%+ of apps including all VoiceType targets |

---

## Feature Dependencies

```
[Remove clipboard save]
    └──enables──> [Remove clipboard restore]
                      └──simplifies──> [inject_text function]
                      └──removes──> [80ms post-paste sleep]
                                        └──reduces──> [Total injection latency]

[Transcription history] ──mitigates──> [No clipboard restore concern]
    (already exists)

[Optional clipboard restore setting] ──requires──> [Settings infrastructure (exists)]
    (future, not v1.3)                └──requires──> [Keep save/restore code behind feature flag]
```

### Dependency Notes

- **Save and restore are a pair.** Removing save without removing restore would try to restore `None`, which current code handles by clearing clipboard to empty string -- worse than current behavior and worse than the proposed change.
- **Transcription history mitigates the clipboard loss concern.** The main user worry ("I lose what I copied") is addressed by Win+V clipboard history (Windows 10+) and the existing transcription history panel.
- **The 80ms post-paste sleep exists solely for restore timing.** Once restore is removed, this sleep serves no purpose and should be removed, saving 80ms of injection latency.
- **The 150ms pre-paste sleep is unrelated to save/restore.** It exists for Office/Outlook clipboard cache sync and must be kept.
- **The clipboard verification loop is unrelated to save/restore.** It ensures the correct text is on the clipboard before Ctrl+V. Must be kept.

---

## Scope Definition for v1.3

### Must Do

- Remove `saved: Option<String> = clipboard.get_text().ok()` (inject.rs line 43)
- Remove the restore block at lines 134-149 (`match saved` / `set_text` restore)
- Remove the 80ms post-paste sleep at line 132 (exists solely for restore timing)

### Must Keep

- Clipboard verification retry loop (lines 53-99) -- reliability, not save/restore
- 150ms pre-paste delay (lines 101-111) -- Office/Outlook clipboard cache sync
- Win key release before paste (lines 118-120) -- keyboard hook race prevention

### Do Not Do

- Add a clipboard restore toggle setting -- unnecessary for v1.3
- Change to keystroke simulation -- wrong trade-off
- Implement TSF/IME -- massive scope, marginal benefit

### Impact Assessment

| Metric | Before (v1.2) | After (v1.3) | Change |
|--------|--------------|--------------|--------|
| `inject_text` code lines | ~150 | ~100 | -33% |
| Post-paste sleep | 80ms | 0ms | -80ms latency |
| Clipboard after dictation | Restored to previous | Contains transcription | Standard behavior |
| Non-text clipboard edge cases | Must handle image/file types in save path | Eliminated | Entire class of edge cases removed |

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Remove clipboard save/restore | HIGH | LOW (delete code) | P1 |
| Remove 80ms post-paste sleep | MEDIUM | LOW (delete one line) | P1 |
| Keep clipboard verification | HIGH | ZERO (already exists) | P1 |
| Optional restore toggle | LOW | LOW | P3 |

---

## Competitor Feature Analysis

| Behavior | Superwhisper | OpenWhispr | Dragon (clipboard path) | VoiceType v1.2 (current) | VoiceType v1.3 (proposed) |
|----------|-------------|-----------|------------------------|--------------------------|---------------------------|
| Clipboard used for injection | Yes | Yes | Yes (fallback) | Yes | Yes |
| Transcription on clipboard after | Yes (default) | Yes | Yes | No (restored) | Yes |
| Clipboard restore available | Yes (opt-in toggle) | No | No | Yes (always on) | No (removed) |
| Restore timing | 3 seconds after paste | N/A | N/A | 80ms after paste | N/A |
| Re-paste transcription possible | Yes | Yes | Yes | No (clipboard restored) | Yes |

---

## Sources

- [Microsoft Support: Windows Voice Typing](https://support.microsoft.com/en-us/windows/use-voice-typing-to-talk-instead-of-type-on-your-pc-fec94565-c4bd-329d-e59a-af033fa5689f) -- Uses direct text input API, no clipboard (HIGH confidence)
- [Superwhisper Advanced Settings](https://superwhisper.com/docs/get-started/settings-advanced) -- Clipboard restore is opt-in, OFF by default, 3s delay (HIGH confidence)
- [Superwhisper Clipboard Issue #11103](https://github.com/openai/codex/issues/11103) -- Confirms clipboard paste as default behavior (MEDIUM confidence)
- [LinkedIn: Superwhisper clipboard tip](https://www.linkedin.com/posts/akshayluther_superwhisper-pro-tip-preserve-your-clipboard-activity-7320680840307306497-fUv1) -- Clipboard restore is power-user feature (MEDIUM confidence)
- [Talon Community Wiki](https://talon.wiki/Basic%20Usage/basic_usage/) -- Default insert() avoids clipboard; clip.capture() saves/restores when clipboard used (MEDIUM confidence)
- [Nuance: Dictation Box](https://www.nuance.com/products/help/dragon/dragon-for-pc/enx/dps/main/Content/Dictation/using_the_dictation_box.htm) -- Dragon uses Ctrl+V for unsupported apps, no restore (HIGH confidence)
- [WhisperWriter](https://github.com/savbell/whisper-writer) -- Keystroke simulation, no clipboard use (HIGH confidence)
- [OpenWhispr](https://github.com/HeroTools/open-whispr) -- Clipboard paste, no restore (MEDIUM confidence)
- [KnowBrainer Forums](https://forums.knowbrainer.com/forum/dragon-speech-recognition/833-v-pasting-interferes-with-clipboard-history-from-dragoncapture-typemode-on) -- Users complain about Dragon polluting clipboard history, not about overwrite (MEDIUM confidence)

---
*Feature research for: VoiceType v1.3 Clipboard Simplification*
*Researched: 2026-03-07*
