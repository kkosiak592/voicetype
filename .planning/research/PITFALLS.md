# Pitfalls Research

**Domain:** Clipboard simplification — removing save/restore from inject_text in a Windows dictation tool (VoiceType v1.3)
**Researched:** 2026-03-07
**Confidence:** HIGH (pitfalls derived from current codebase analysis, Windows clipboard Win32 documentation, and established patterns from clipboard managers and password managers)

---

## Critical Pitfalls

### Pitfall 1: Removing the Post-Paste Sleep Prematurely Along With the Restore

**What goes wrong:**
When removing the clipboard restore logic, a developer naturally removes the 80ms post-paste sleep (line 132 in current inject.rs) thinking "we're not restoring anymore, so we don't need to wait." The paste then intermittently fails — target applications receive an empty paste or stale clipboard content because Ctrl+V was released before the app consumed the clipboard data.

**Why it happens:**
The 80ms sleep at line 132 serves two distinct purposes that look like one:
1. Give the target app time to consume the paste before restoring (the documented purpose)
2. Give the target app time to consume the paste before *any* subsequent clipboard operation occurs

Even without a restore, other code paths can touch the clipboard shortly after inject_text returns — the history panel's click-to-copy feature, or a rapid second transcription. If the sleep is removed and a clipboard write happens within milliseconds of the Ctrl+V, the paste can fail silently.

**How to avoid:**
Keep the 80ms post-paste sleep. It exists to let the target application process WM_PASTE / CF_TEXT, not just to buffer the restore. Add a code comment explicitly stating this sleep is for paste consumption, independent of save/restore logic.

**Warning signs:**
- Paste works 95% of the time but occasionally pastes nothing or old content
- Failure rate higher in Outlook, Word, and other Office apps (they process clipboard asynchronously via WM_CLIPBOARDUPDATE)
- Rapid sequential transcriptions fail more often than single transcriptions

**Phase to address:** The single implementation phase. This is a "do not touch this line" guard during the removal.

---

### Pitfall 2: Clipboard Verification Loop Becoming Unnecessary Complexity — But It Is Still Necessary

**What goes wrong:**
The developer sees the verify-and-retry loop (lines 53-99) and the 150ms pre-paste delay (line 111) and thinks: "These were needed because of the race between setting clipboard and restoring clipboard. Without restore, we can simplify these too." They reduce or remove the verification loop, and intermittent paste failures return — particularly in Chrome/Edge tabs and Office apps.

**Why it happens:**
The verification loop and the 150ms pre-paste delay exist because of two separate problems, neither of which is related to save/restore:

1. **Chromium WebView clipboard race** (lines 47-52): The Tauri WebView (Chromium) can reclaim clipboard ownership via `navigator.clipboard.writeText()` when the user interacts with history panel click-to-copy. This race exists regardless of whether clipboard is restored afterward.

2. **Office async clipboard cache** (lines 103-110): Office apps process WM_CLIPBOARDUPDATE asynchronously and cache clipboard content internally. The 150ms delay ensures Office has ingested the new clipboard content before Ctrl+V fires. This is a paste-reliability concern, not a restore concern.

**How to avoid:**
Keep the verification loop and the 150ms pre-paste delay intact. The only code to remove is:
- Line 43: `let saved: Option<String> = clipboard.get_text().ok();`
- Lines 135-149: The entire `match saved { ... }` restore block

Nothing else should change. Add comments on the verification loop clarifying it protects against Chromium races, not restore races.

**Warning signs:**
- Paste silently fails in Chrome/Edge tabs (Chromium reclaimed clipboard)
- Outlook pastes stale content from its internal cache instead of the transcription
- The verification loop reports mismatches in logs but was disabled because "we don't need it anymore"

**Phase to address:** The single implementation phase. The diff should be minimal — removal of exactly the save (1 line) and restore (15 lines) blocks.

---

### Pitfall 3: Every Transcription Now Appears in Windows Clipboard History (Win+V)

**What goes wrong:**
After removing save/restore, the transcription text remains on the clipboard. Windows Clipboard History (if enabled) captures every transcription. The user presses Win+V and sees a long list of every dictated sentence — potentially sensitive content like emails, passwords dictated into login fields, or private messages. In enterprise environments with clipboard sync enabled, this data may be synced to Microsoft's cloud.

**Why it happens:**
The old save/restore pattern had a subtle privacy benefit: the transcription was on the clipboard only during the ~230ms paste window (150ms pre-paste delay + 80ms post-paste consumption), after which the original content was restored. Windows Clipboard History captures items when they are set, so the transcription was still captured — but the restore immediately pushed the original content back, meaning the clipboard history showed the original content as the "current" item rather than the transcription.

With simplification, the transcription stays on the clipboard indefinitely. Every transcription is now a permanent clipboard history entry until the user clears history or it ages out.

**How to avoid:**
This is an accepted behavior change, not a bug. Standard dictation tools (Dragon NaturallySpeaking, Windows Voice Typing) all leave transcription on the clipboard. However, document this in release notes for user awareness.

If privacy becomes a concern later, Windows provides clipboard history exclusion via the `ExcludeClipboardContentFromMonitorProcessing` clipboard format. Setting this format alongside the text data tells Windows Clipboard History and cloud sync to ignore the entry. arboard does not expose this directly — it would require raw Win32 clipboard API calls via `OpenClipboard` / `SetClipboardData` with a custom registered format. This is an optional future enhancement, not a v1.3 requirement.

**Warning signs:**
- Users report sensitive dictation appearing in Win+V clipboard history
- Enterprise users with clipboard cloud sync enabled see transcriptions synced to other devices
- Third-party clipboard managers (Ditto, CopyQ) capture every transcription as a new entry

**Phase to address:** Document in release notes during the implementation phase. The exclusion format is a potential follow-up if users report privacy concerns.

---

### Pitfall 4: Non-Text Clipboard Content Permanently Lost

**What goes wrong:**
User copies an image, a file path from Explorer, or rich text (formatted text from Word/Outlook) to their clipboard. They then dictate with VoiceType. The transcription replaces the clipboard content. The image/file/rich-text is gone — no way to recover it.

With the old save/restore behavior, this was *also* the case (arboard's `get_text()` only saves text content, so images were already being lost and replaced with an empty string on restore). However, users may not have noticed because:
1. The clipboard appeared to be "unchanged" after dictation (restored to empty or to text content that was there before)
2. Users rarely copy an image and then immediately dictate

With simplification, the clipboard *visibly* contains the transcription instead of appearing unchanged. This makes the content loss obvious and noticeable.

**Why it happens:**
arboard's `set_text()` calls `OpenClipboard` / `EmptyClipboard` / `SetClipboardData` with CF_UNICODETEXT. The `EmptyClipboard` call destroys all clipboard formats — CF_BITMAP, CF_HDROP (file list), CF_HTML, CF_RTF, and any custom formats. This is standard Win32 behavior: the clipboard can only have one owner at a time, and `EmptyClipboard` clears all formats from the previous owner.

**How to avoid:**
This is the intended behavior and matches every other dictation tool. The clipboard is a transient holding area, not a persistent store. Accept the behavior change. The only mitigation needed is documentation clarity — update the CLAUDE.md key decisions to note that clipboard content is replaced by transcription.

If users want clipboard persistence, they should use a clipboard manager (Ditto, Windows Clipboard History via Win+V). This is not VoiceType's responsibility.

**Warning signs:**
- No warning signs needed — this is expected behavior
- The only risk is a user complaint: "I had an image on my clipboard and VoiceType erased it"

**Phase to address:** No code mitigation needed. Acknowledge in release notes.

---

## Moderate Pitfalls

### Pitfall 5: The `Clipboard::new()` Call Fails If Another App Has the Clipboard Open

**What goes wrong:**
`Clipboard::new()` (arboard) internally calls `OpenClipboard`. If another application currently has the clipboard open (e.g., a clipboard manager polling the clipboard, or the user is actively pasting in another app), `OpenClipboard` fails with `ERROR_ACCESS_DENIED`. The current code at line 40 propagates this as an error, failing the entire injection.

**Why it happens:**
The Win32 clipboard is a global resource with exclusive locking. Only one process can have the clipboard open at a time. Clipboard managers like Ditto poll the clipboard frequently, and some hold the clipboard open for several milliseconds during each poll cycle. The existing code already handles this implicitly (the retry loop at lines 57-92 retries `set_text`), but `Clipboard::new()` on line 40 has no retry — it fails immediately.

**How to avoid:**
This pitfall exists in the current code and is unrelated to the save/restore removal. However, removing the save step (line 43: `clipboard.get_text().ok()`) eliminates one clipboard access that could also fail. The simplification marginally *improves* reliability by reducing the number of clipboard operations from 4 (get_text for save, set_text, get_text for verify, set_text for restore) to 2 (set_text, get_text for verify).

No additional mitigation needed for v1.3 — this is already handled acceptably.

**Warning signs:**
- Sporadic `inject_text: clipboard access failed` errors in logs
- Failures correlate with Ditto or another clipboard manager being installed

**Phase to address:** Not a v1.3 concern. Existing behavior is acceptable.

---

### Pitfall 6: Removing Save/Restore Changes the Timing Profile — Regression in Paste Reliability

**What goes wrong:**
The old flow was: `get_text` (save) -> `set_text` -> `get_text` (verify) -> sleep 150ms -> Ctrl+V -> sleep 80ms -> `set_text` (restore). Total clipboard operations: 4. Total time: ~280ms + verification attempts.

The new flow is: `set_text` -> `get_text` (verify) -> sleep 150ms -> Ctrl+V -> sleep 80ms. Total clipboard operations: 2. Total time: ~280ms + verification attempts, but the *start* of the function is faster because there's no initial `get_text`.

The overall latency improvement is minimal (~1-5ms for the removed get_text call), but the *timing window* changes. Some clipboard-monitoring apps track clipboard ownership changes and may behave differently when there are fewer ownership transitions.

**Why it happens:**
The save operation (`get_text`) read the clipboard without changing ownership. But it did open and close the clipboard, which introduced a brief window where the clipboard was "locked" before the set_text call. Some clipboard managers interpret rapid open/close/open/close patterns as "an app is actively using the clipboard" and back off from polling. With the save removed, the first clipboard operation is now `set_text` (which changes ownership immediately), and monitoring apps may compete for clipboard access more aggressively.

**How to avoid:**
This is unlikely to cause real problems but is worth noting. The 150ms pre-paste delay already provides ample time for clipboard propagation. Monitor logs for increased clipboard verification retry rates after the change. If retry rates increase, the existing retry loop already handles it.

**Warning signs:**
- Clipboard verification retry count increases after the change (visible in log output)
- More frequent clipboard mismatch warnings in logs

**Phase to address:** Monitor during testing after implementation. No preemptive action needed.

---

### Pitfall 7: The Empty-String Restore for Non-Text Content Is Also Being Removed

**What goes wrong:**
Lines 144-148 in the current code handle the case where the original clipboard was empty or non-text: they call `clipboard.set_text("")` to "clear" the clipboard. This was a cleanup step. After removal, if the original clipboard had an image, and the user dictates, the clipboard will contain the transcription text. If the user then tries to paste expecting the image... they get text instead.

With the old behavior, they would have gotten an empty string paste — also wrong, but in a different way.

**Why it happens:**
The old code was already broken for non-text clipboard content — it could never properly save/restore images or rich text. It just masked the breakage by setting an empty string. The simplification makes the behavior more *honest*: the clipboard contains exactly what VoiceType put there, no pretense of preservation.

**How to avoid:**
No mitigation needed. The old behavior was already lossy for non-text content. The new behavior is more predictable: after dictation, the clipboard always contains the transcription. Users can reason about this more easily than "sometimes my clipboard is mysteriously empty after dictation."

**Warning signs:**
- None — this is an improvement in predictability

**Phase to address:** Acknowledge in commit message that non-text clipboard content was already not preserved.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Remove the 80ms post-paste sleep alongside the restore | Saves 80ms per transcription | Intermittent paste failures in Office apps that haven't consumed the clipboard yet | Never — the sleep protects paste consumption, not restore |
| Remove the 150ms pre-paste delay alongside the restore | Saves 150ms per transcription | Outlook and Word paste stale cached content instead of the transcription | Never — the delay protects Office app clipboard ingestion |
| Remove the verification loop alongside the restore | Simpler code, fewer clipboard operations | Chromium WebView clipboard races cause silent paste failures | Never — the verification loop protects against a different race condition |
| Skip testing with clipboard managers (Ditto, CopyQ) installed | Faster testing cycle | Undiscovered clipboard contention issues in real-world environments | Only for initial smoke test; must test with clipboard managers before release |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| Windows Clipboard History (Win+V) | Assume transcriptions won't appear in history | Accept that every transcription will appear; document for users. Optionally exclude via `ExcludeClipboardContentFromMonitorProcessing` format in future |
| Third-party clipboard managers (Ditto, CopyQ) | Assume they won't interfere | Test that clipboard managers don't reclaim clipboard ownership between set_text and Ctrl+V. The existing 150ms delay and retry loop handle this. |
| Office apps (Outlook, Word, Excel) | Remove the 150ms pre-paste delay thinking it was for the restore | The delay exists because Office processes WM_CLIPBOARDUPDATE asynchronously. Keep it. |
| Tauri WebView (Chromium) | Remove the verify-and-retry loop thinking it was for the restore | The loop exists because Chromium can reclaim clipboard ownership via navigator.clipboard API. Keep it. |
| Password managers (Bitwarden, KeePass) | Not considering that password managers clear clipboard on a timer | If the user copies a password, dictates, then the password manager timer fires — it may clear the transcription from clipboard. This is the password manager's behavior, not VoiceType's problem. No action needed. |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Removing both sleeps (150ms + 80ms) to "speed up" injection | Paste works on developer's machine but fails in Outlook on slower machines | Keep both sleeps; they are for target app compatibility, not clipboard logic | Immediately on Office apps, especially on machines with many Outlook add-ins |
| Removing the verify loop to reduce clipboard operations from 2 to 1 | Paste occasionally injects wrong text | Keep the loop; it catches real clipboard races | When Tauri WebView has recently used navigator.clipboard API |

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Transcription of sensitive dictation (passwords, PII) persists on clipboard indefinitely | Another app or user can read the clipboard content; appears in clipboard history | Accept as standard dictation tool behavior; users should not dictate passwords. Optionally add `ExcludeClipboardContentFromMonitorProcessing` format in future |
| Clipboard sync to cloud enabled in Windows Settings | Transcribed text synced to Microsoft cloud and other devices | Not VoiceType's responsibility; users control clipboard sync settings. Note in docs if privacy is a concern |

---

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Not documenting the behavior change | Users who relied on clipboard being "unchanged" after dictation are surprised when their copied content is replaced | Add a note in release notes or changelog: "After dictation, the transcription text remains on your clipboard. This matches standard dictation tool behavior." |
| Users expect the clipboard to be "clean" after dictation | Some users may not want the transcription lingering on clipboard (sensitive content) | This is standard behavior for every dictation tool; no special handling needed |

---

## "Looks Done But Isn't" Checklist

- [ ] **Post-paste sleep preserved:** Verify the 80ms sleep after Ctrl+V is still present. Remove only the save/restore code, not the sleep.
- [ ] **Pre-paste delay preserved:** Verify the 150ms delay before Ctrl+V is still present. Test paste in Outlook specifically.
- [ ] **Verify loop preserved:** Verify the 5-attempt clipboard verification loop is still present. Test with Tauri settings window open (WebView active).
- [ ] **Doc comments updated:** The inject_text doc comment (lines 27-37) references save/restore in the sequence. Update to reflect the simplified flow.
- [ ] **Release Win keys still called:** The `release_win_keys` call (line 120) must remain — it is unrelated to clipboard save/restore.
- [ ] **Error handling unchanged:** The `Clipboard::new()` error propagation and verification failure logging should remain unchanged.
- [ ] **Test in Office apps:** Paste into Outlook, Word, Excel, Teams. Verify the correct transcription appears, not stale content.
- [ ] **Test rapid sequential dictation:** Dictate twice in quick succession. Verify both transcriptions paste correctly (no race between the second set_text and the first app's paste consumption).

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Accidentally removed the post-paste sleep | LOW | Add `thread::sleep(Duration::from_millis(80))` back after Ctrl+V release |
| Accidentally removed the pre-paste delay | LOW | Add `thread::sleep(Duration::from_millis(150))` back before Ctrl+V |
| Accidentally removed the verify loop | MEDIUM | Restore the 5-attempt retry loop from git history; requires understanding the Chromium race condition |
| Users report privacy concerns with clipboard history | LOW | Register `ExcludeClipboardContentFromMonitorProcessing` format via raw Win32 API alongside set_text; arboard doesn't expose this natively |
| Paste regression in Office apps after timing change | LOW | Increase pre-paste delay from 150ms to 200ms if Office apps show issues |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Post-paste sleep removed (#1) | Implementation | Code review: verify `thread::sleep(Duration::from_millis(80))` is present after Ctrl+V |
| Verify loop removed (#2) | Implementation | Code review: verify the retry loop and 150ms delay are untouched |
| Clipboard history exposure (#3) | Release notes | Changelog mentions the behavior change |
| Non-text content loss (#4) | Release notes | Acknowledged in commit message |
| Timing profile change (#6) | Post-implementation testing | Compare clipboard retry rates in logs before and after the change |
| Doc comments stale (#checklist) | Implementation | Doc comment on inject_text updated to reflect new flow |

---

## Sources

- [Clipboard Formats — Microsoft Win32 Docs](https://learn.microsoft.com/en-us/windows/win32/dataxchg/clipboard-formats) — HIGH confidence
- [WM_CLIPBOARDUPDATE — Microsoft Win32 Docs](https://learn.microsoft.com/en-us/windows/win32/dataxchg/wm-clipboardupdate) — HIGH confidence
- [arboard Clipboard API — docs.rs](https://docs.rs/arboard/latest/arboard/struct.Clipboard.html) — HIGH confidence
- [arboard GitHub — 1Password](https://github.com/1Password/arboard) — HIGH confidence (arboard only saves text and images; no custom format support)
- [ExcludeClipboardContentFromMonitorProcessing — CopyQ issue #2679](https://github.com/hluk/CopyQ/issues/2679) — MEDIUM confidence (documents the clipboard history exclusion format; confirmed by KeePass implementation)
- [PowerShell clipboard history exclusion — GitHub issue #17454](https://github.com/PowerShell/PowerShell/issues/17454) — MEDIUM confidence (confirms CanIncludeInClipboardHistory and ExcludeClipboardContentFromMonitorProcessing formats)
- [Windows Clipboard History privacy — GhostVolt](https://www.ghostvolt.com/blog/Is-the-Windows-Clipboard-Function-History-or-Sync-Secure.html) — MEDIUM confidence
- [Password manager clipboard security — TechSpot](https://www.techspot.com/news/97320-you-change-password-manager-clipboard-settings-now.html) — MEDIUM confidence (context on clipboard as a security surface)
- Current codebase: `src-tauri/src/inject.rs` — direct analysis of existing implementation

---
*Pitfalls research for: Clipboard simplification — removing save/restore from inject_text (VoiceType v1.3)*
*Researched: 2026-03-07*
