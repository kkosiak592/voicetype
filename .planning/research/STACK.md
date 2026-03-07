# Stack Research: Clipboard Simplification

**Domain:** Voice-to-text clipboard flow simplification (v1.3)
**Researched:** 2026-03-07
**Confidence:** HIGH

## Verdict: No New Dependencies Required

The clipboard simplification is a pure code removal task. No new crates, no version bumps, no new APIs. The existing `arboard 3.6.1` and `enigo 0.6` remain unchanged.

## Current Stack (Unchanged)

### Clipboard and Injection

| Technology | Version | Purpose | Status for v1.3 |
|------------|---------|---------|------------------|
| arboard | 3.6.1 | Clipboard read/write | KEEP -- no API changes needed |
| enigo | 0.6 | Ctrl+V keystroke simulation | KEEP -- no changes needed |

### arboard API Usage After Simplification

Only these arboard methods remain in use after the change:

| Method | Purpose | Notes |
|--------|---------|-------|
| `Clipboard::new()` | Create clipboard handle | Unchanged |
| `clipboard.set_text(text)` | Write transcription to clipboard | Unchanged |
| `clipboard.get_text()` | Verify clipboard content after write | Unchanged -- still needed for verify-and-retry loop |

**Removed API usage:**
- `clipboard.get_text()` for save (line 43 of current inject.rs) -- eliminated
- `clipboard.set_text(&original)` for restore (line 137) -- eliminated
- `clipboard.set_text("")` for clear-on-empty (line 147) -- eliminated

## What Changes (Code Only)

### Lines to Remove from `inject.rs`

1. **Save** (line 43): `let saved: Option<String> = clipboard.get_text().ok();`
2. **Post-paste sleep** (lines 129-132): The 80ms sleep after Ctrl+V. This existed solely to let the target app consume the paste before clipboard restore overwrote it. Without restore, no overwrite race exists.
3. **Restore block** (lines 134-149): The entire `match saved { ... }` block.

### Lines to Keep

1. **Verify-and-retry loop** (lines 53-99): Still required. Chromium WebView clipboard races are a real problem regardless of save/restore. The loop ensures our text is actually on the clipboard before pasting.
2. **150ms pre-paste delay** (lines 101-111): Still required. Office apps (Outlook, Word) process WM_CLIPBOARDUPDATE asynchronously. This delay prevents pasting stale content from their internal cache.
3. **Win key release** (lines 117-120): Still required. Defensive against Win-key-stuck failure mode from keyboard hook.
4. **Ctrl+V simulation** (lines 123-127): Obviously still required.

### Cargo.toml Comment Update

Line 66 currently reads:
```toml
# Clipboard save/restore for text injection (always available -- no whisper dependency)
arboard = "3"
```

Should become:
```toml
# Clipboard write for text injection (always available -- no whisper dependency)
arboard = "3"
```

## Timing Analysis After Simplification

### Current Timing Budget (per inject_text call)

| Step | Duration | After Simplification |
|------|----------|---------------------|
| Save clipboard | ~1ms | REMOVED |
| Verify loop (happy path) | ~25ms (1 attempt) | KEPT |
| Verify loop (retry) | ~50-125ms (2-5 attempts) | KEPT |
| Pre-paste delay (Office apps) | 150ms | KEPT |
| Win key release | <1ms | KEPT |
| Ctrl+V simulation | <1ms | KEPT |
| Post-paste sleep | 80ms | REMOVED |
| Clipboard restore | ~1ms | REMOVED |

**Net latency reduction: ~82ms** on every injection. The 80ms post-paste sleep is the meaningful savings; save/restore are sub-millisecond.

### Post-Paste Sleep: Why Remove It

The 80ms sleep existed to ensure the target app consumed the paste before the clipboard was overwritten by the restore. Without a restore, there is no overwrite race. The clipboard retains the transcription text indefinitely (until the user copies something else), so even slow applications will read the correct content whenever they process the paste.

Edge case: Some apps (notably Excel, Outlook) process paste asynchronously and may read the clipboard after the Ctrl+V event returns. This is fine -- the clipboard still contains the transcription text. The only scenario where async paste was a problem was when the restore overwrote it too early, which no longer applies.

## Windows Clipboard Ownership Considerations

### Clipboard Ownership Model

When `arboard::set_text()` is called, it:
1. Opens the clipboard (`OpenClipboard`)
2. Empties it (`EmptyClipboard`) -- this transfers ownership to our process
3. Sets the data (`SetClipboardData`)
4. Closes the clipboard (`CloseClipboard`)

After simplification, VoiceType becomes the clipboard owner after every transcription. Implications:

- **No ownership conflict**: We set the clipboard once and leave it. No second open/close for restore.
- **Fewer clipboard operations**: 1 open/close cycle instead of 3 (save + set + restore). Removing 2 cycles.
- **Reduced race window**: Fewer clipboard operations means fewer chances for another app to interleave clipboard access.

### Clipboard Viewer Notifications

Windows sends `WM_CLIPBOARDUPDATE` to clipboard viewers when content changes. Current flow sends this notification twice (once for set, once for restore). Simplified flow sends it once. Strictly better -- clipboard managers and history tools see the transcription as a single clipboard event instead of two.

## What NOT to Add

| Avoid | Why | Notes |
|-------|-----|-------|
| clipboard-win crate | Direct Win32 clipboard API | arboard already uses clipboard-win internally on Windows; no need to duplicate |
| Custom clipboard format handling | Preserving images/rich text from clipboard | Out of scope -- the old save/restore only handled text anyway (`get_text()` returns None for non-text) |
| Async clipboard crate | Non-blocking clipboard access | The function is already called via `spawn_blocking`; synchronous access is correct |
| Post-paste verification | Reading clipboard after paste to confirm delivery | Unnecessary -- the pre-paste verify loop already confirms content; target apps do not modify clipboard on paste |

## Alternatives Considered

| Approach | Why Not |
|----------|---------|
| Keep save/restore but make it optional via settings | Unnecessary complexity. Standard dictation tools (Dragon, Windows Voice Typing, macOS Dictation) all leave transcription on clipboard. Users expect this. |
| Save/restore for non-text clipboard content (images) | The current code never did this -- `get_text()` returns `None` for images, and the restore path set empty string. This was never a real feature. |
| Use `clipboard.clear()` after paste instead of restore | arboard 3.x has no `clear()` method. The old code used `set_text("")` as a workaround. But we want the transcription to remain on clipboard, so clearing is wrong anyway. |

## Sources

- arboard 3.6.1 -- version from Cargo.lock, API from current inject.rs usage (HIGH confidence)
- Windows clipboard ownership model (OpenClipboard/EmptyClipboard/SetClipboardData) -- Win32 documentation, well-established behavior (HIGH confidence)
- Current inject.rs implementation -- direct code review (HIGH confidence)
- Dictation tool clipboard behavior (Dragon, Windows Voice Typing, macOS Dictation) -- common knowledge from domain experience (MEDIUM confidence)

---
*Stack research for: v1.3 Clipboard Simplification*
*Researched: 2026-03-07*
