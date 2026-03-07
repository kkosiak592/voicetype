# Architecture: Clipboard Simplification (v1.3)

**Domain:** Clipboard flow simplification in voice-to-text injection pipeline
**Researched:** 2026-03-07
**Confidence:** HIGH -- analysis based on actual source code; no external dependencies or unverified claims

---

> **Scope:** This document covers the clipboard save/restore removal for v1.3 only. It focuses on what changes in `inject.rs`, what does NOT change, race condition analysis, timing implications, and build order. The broader application architecture is unchanged.

---

## Current inject_text() Flow (inject.rs)

```
1. Save clipboard         ── clipboard.get_text().ok()                    [line 43]
2. Write transcription     ── clipboard.set_text(text)                    [line 58]
3. Verify-and-retry loop   ── up to 5 attempts, 25ms delay each          [lines 53-99]
4. Pre-paste delay         ── 150ms (Office WM_CLIPBOARDUPDATE latency)  [line 111]
5. Release Win keys        ── defensive against stuck Win from hook       [line 120]
6. Simulate Ctrl+V         ── Press Ctrl, Click V, Release Ctrl           [lines 124-127]
7. Post-paste delay        ── 80ms (let target app consume before restore)[line 132]
8. Restore clipboard       ── set_text(original) or clear if was empty    [lines 135-148]
```

## Pipeline Call Site (pipeline.rs)

```
run_pipeline() async:
  ...steps 1-5 (audio → transcription → formatting)...
  6. spawn_blocking(inject_text(&to_inject))    ← only clipboard interaction
  7. append_history(app, &formatted, engine)     ← no clipboard interaction
  8. reset_to_idle()
```

## History Panel Clipboard Usage (HistorySection.tsx)

```
handleCopy(text, index):
  navigator.clipboard.writeText(text)    ← browser Async Clipboard API
```

---

## Modifications

### inject.rs -- Removals Only

| What | Lines | Action | Rationale |
|------|-------|--------|-----------|
| Save clipboard | 43 (`let saved = clipboard.get_text().ok()`) | Remove | No restore means no need to save |
| Post-paste delay | 132 (`thread::sleep(Duration::from_millis(80))`) | Remove | Existed solely to let target app consume paste before restore overwrites clipboard; without restore, clipboard retains correct content indefinitely |
| Restore block | 135-148 (entire `match saved` block) | Remove | Core change -- transcription stays on clipboard |

### inject.rs -- Unchanged

| Component | Lines | Why Unchanged |
|-----------|-------|---------------|
| Verify-and-retry loop | 53-99 | Correctness mechanism, not related to restore (see Race 2 below) |
| 150ms pre-paste delay | 111 | Office apps need WM_CLIPBOARDUPDATE processing time before Ctrl+V arrives |
| Win key release | 117-123 | Keyboard hook defense, unrelated to clipboard flow |
| Ctrl+V simulation | 124-127 | Core injection mechanism |
| Enigo fresh instance | 114 | Required per-call pattern |

### Other Files -- Zero Changes

| File | Why Unchanged |
|------|---------------|
| `pipeline.rs` | Calls `inject_text()` with same signature, same spawn_blocking wrapper |
| `HistorySection.tsx` | `navigator.clipboard.writeText()` is independent of inject flow |
| `history.rs` | No clipboard interaction whatsoever |
| `keyboard_hook.rs` | Unrelated to clipboard |
| `lib.rs` | No clipboard-related code |

---

## Race Condition Analysis

### Race 1: History Panel vs inject_text()

**Question:** Can `navigator.clipboard.writeText()` from HistorySection race with `inject_text()`?

**Answer: No practical risk.**

Temporal separation makes this impossible in normal use:
- `inject_text()` runs on a `spawn_blocking` thread during pipeline processing, while focus is on the target application (VS Code, Outlook, etc.)
- History panel copy requires user to: switch to the settings window, navigate to History tab, click an entry
- User cannot simultaneously dictate AND click the history panel -- the settings window requires focus, which means the target app lost focus

Even in a theoretical edge case (user copies from history, immediately triggers dictation), pipeline transcription takes 200-1500ms before reaching `inject_text()`. The `navigator.clipboard.writeText()` completes in under 1ms. No temporal overlap.

**After simplification:** No change. The race window was always between `set_text()` and `Ctrl+V` within `inject_text()`, not between inject and restore. Removing restore does not create new overlap.

**Verdict: No mitigation needed.**

### Race 2: Chromium WebView Clipboard Contention

**What:** The verify-and-retry loop exists because Tauri's Chromium WebView can reclaim clipboard ownership after a recent `navigator.clipboard.writeText()` call, silently overwriting arboard's content before the Ctrl+V fires.

**Why the loop is NOT about restore:** The loop runs *before* the paste, ensuring the clipboard contains the correct text when Ctrl+V fires. It has nothing to do with what happens after paste. The loop prevents pasting wrong content; restore prevents leaving transcription on clipboard. These are orthogonal concerns.

**After simplification:** Loop remains necessary. If removed, a Chromium clipboard race would cause the wrong text to be pasted -- a correctness bug unrelated to restore.

**Verdict: Keep verify-and-retry loop unchanged.**

### Race 3: Post-Paste Clipboard State (Behavioral Change)

**Current:** After paste, clipboard is restored to pre-transcription content. User's clipboard appears untouched.

**After:** Clipboard contains the transcription text. Ctrl+V in any app produces the last transcription.

**This is the desired behavior.** Standard dictation tools (Dragon NaturallySpeaking, Windows Voice Typing, macOS Dictation) all leave dictated text on the clipboard. The current save/restore adds complexity for behavior users do not expect from a dictation tool.

**Verdict: Desired change, not a bug.**

---

## Timing Analysis

### Removed Delays

| Delay | Duration | Why Removable |
|-------|----------|---------------|
| Post-paste sleep | 80ms | Comment on line 129 states purpose: "Allow target app to consume the paste before clipboard restore." Without restore, even if the target app reads the clipboard slightly late, it still gets the correct text (transcription remains on clipboard). |

### Retained Delays

| Delay | Duration | Why Still Required |
|-------|----------|--------------------|
| Verify-retry delay | 25ms per attempt (typically 1 attempt = 25ms, worst case 5 = 125ms) | Clipboard propagation is async on Windows. Read-back verification is the only way to confirm the OS clipboard contains the intended text. |
| Pre-paste delay | 150ms | Office apps (Outlook, Word, Excel) maintain internal clipboard caches updated via WM_CLIPBOARDUPDATE. This message is processed asynchronously. Without the delay, Outlook pastes stale cached content. Verified through testing during v1.0 development. |

### Net Impact

**Best case (typical):** 80ms reduction per injection (single verify attempt)
**Worst case:** 80ms reduction per injection (loop retries are independent of restore)

This is a fixed, unconditional improvement. The removed 80ms was not conditional on any state.

### Can the 150ms Pre-Paste Delay Be Reduced?

Not advisable in this milestone. The 150ms was calibrated during v1.0 against Outlook and Word. The comment on lines 107-110 documents the rationale: "WM_CLIPBOARDUPDATE processing in Office apps typically completes within 50-100ms, but we add margin for slower machines." Reducing it is a separate investigation requiring testing across multiple Office versions and hardware. Out of scope for clipboard simplification.

---

## Data Flow After Simplification

```
User speaks
  → audio buffer
  → engine transcription (spawn_blocking)
  → text formatting (trim, filler removal, corrections, ALL CAPS, trailing space)
  → inject_text():
       clipboard.set_text(text)
       verify loop (25ms per attempt, usually 1)
       sleep 150ms (Office app compatibility)
       release_win_keys()
       Ctrl+V (Press Ctrl → Click V → Release Ctrl)
       return                    ← no restore, no 80ms wait
  → append_history()
  → emit pill-result "success"
  → reset_to_idle()

Clipboard state after injection: contains transcription text (by design)
```

## Component Boundaries

| Component | Responsibility | Changed? |
|-----------|---------------|----------|
| `inject.rs::inject_text()` | Write text to clipboard, verify, simulate Ctrl+V | YES -- remove save/restore/post-paste sleep |
| `pipeline.rs::run_pipeline()` | Orchestrate: record → transcribe → format → inject → history | NO |
| `HistorySection.tsx` | Display history entries, click-to-copy via browser API | NO |
| `history.rs` | Persist/retrieve transcription entries | NO |
| `keyboard_hook.rs` | WH_KEYBOARD_LL modifier-only hotkey detection | NO |
| `lib.rs` | Tauri setup, IPC commands, hotkey routing | NO |

## Build Order

This is a single-file, deletion-only change:

1. **Modify `inject.rs`** -- Remove three blocks: clipboard save (line 43), post-paste sleep (line 132), restore block (lines 135-148). Update the doc comment on `inject_text()` to reflect the new sequence.
2. **Update doc comment** -- The function's `///` header describes the current 5-step sequence. Update to reflect the simplified 3-step sequence (write → verify → paste).
3. **Test** -- Verify in target apps: VS Code, Outlook, Chrome, terminal. Confirm clipboard contains transcription after paste. Confirm history panel click-to-copy still works.

No new files. No new components. No API changes. Function signature `fn inject_text(text: &str) -> Result<(), String>` is unchanged. `pipeline.rs` requires zero modifications.

## Anti-Patterns to Avoid

### Anti-Pattern 1: Removing the Verify-and-Retry Loop

**Why tempting:** "Without restore, the clipboard is simpler -- maybe we don't need verification."
**Why wrong:** Verification ensures the correct text is on the clipboard before Ctrl+V fires. Without it, a Chromium WebView clipboard race causes the wrong text to be pasted. This is a correctness mechanism, not a restore-related one. The existing code comments (lines 46-52) document this explicitly.

### Anti-Pattern 2: Removing the 150ms Pre-Paste Delay

**Why tempting:** "We verified the clipboard; just paste now."
**Why wrong:** Verification confirms arboard can read back the content via the Win32 clipboard API. It does not confirm that Office apps have processed WM_CLIPBOARDUPDATE and updated their internal caches. Outlook has been observed pasting stale content without this delay. The comments on lines 101-110 document this.

### Anti-Pattern 3: Adding a Clipboard Clear After Paste

**Why tempting:** "Security -- don't leave dictated text on clipboard."
**Why wrong:** Contradicts the stated goal. The entire point of this milestone is that transcription stays on clipboard, matching standard dictation tool behavior.

### Anti-Pattern 4: Reducing Post-Paste Sleep Instead of Removing It

**Why tempting:** "Maybe 30ms instead of 80ms, for safety."
**Why wrong:** The sleep has exactly one purpose: prevent clipboard restore from overwriting before the target app reads. With no restore, there is nothing to race against. The target app can read the clipboard at any point in the future and will always get the correct text. A reduced sleep adds latency for no benefit.

---

## Sources

- `src-tauri/src/inject.rs` -- current implementation with inline documentation explaining each delay and the verify loop rationale
- `src-tauri/src/pipeline.rs` -- pipeline orchestration showing inject_text() call site and history recording
- `src/components/sections/HistorySection.tsx` -- frontend clipboard usage via navigator.clipboard.writeText()
- `src-tauri/src/history.rs` -- history persistence confirming no clipboard interaction

---
*Architecture research for: Clipboard simplification in Tauri 2.0 voice-to-text (v1.3 milestone)*
*Researched: 2026-03-07*
