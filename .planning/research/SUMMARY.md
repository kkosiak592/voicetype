# Project Research Summary

**Project:** VoiceType v1.3 — Clipboard Simplification
**Domain:** Voice-to-text dictation tool (Windows, Tauri 2.0, Rust backend)
**Researched:** 2026-03-07
**Confidence:** HIGH

## Executive Summary

VoiceType v1.3 is a surgical code-removal milestone. The existing `inject_text()` function in `inject.rs` saves the user's clipboard before pasting transcription text, then restores it afterward. Research across competing dictation tools (Dragon NaturallySpeaking, Superwhisper, OpenWhispr, Windows Voice Typing) confirms this save/restore pattern is non-standard -- every clipboard-paste dictation tool leaves the transcription on the clipboard by default. The current behavior adds ~82ms latency, creates edge cases with non-text clipboard content, and produces behavior no user expects from a dictation tool.

The recommended approach is a minimal, deletion-only change to `inject.rs`: remove the clipboard save (1 line), the 80ms post-paste sleep (1 line), and the restore block (~15 lines). No new dependencies, no API changes, no modifications to any other file. The function signature is unchanged, `pipeline.rs` requires zero modifications, and the existing clipboard verification loop and pre-paste delay must remain untouched.

The primary risk is collateral damage during implementation: accidentally removing the clipboard verification retry loop or the 150ms pre-paste delay, which exist for unrelated reasons (Chromium WebView clipboard races and Office app cache sync respectively). These look related to save/restore but are orthogonal. The diff must be minimal and precise -- three deletions, nothing else. A secondary concern is clipboard history exposure (every transcription appears in Win+V), but this matches standard dictation tool behavior and requires only release note documentation.

## Key Findings

### Recommended Stack

No stack changes. This is pure code deletion.

**Core technologies (unchanged):**
- **arboard 3.6.1**: Clipboard read/write -- only `set_text()` and `get_text()` (for verification) remain in use after simplification
- **enigo 0.6**: Ctrl+V keystroke simulation -- completely unaffected by this change

API usage after simplification: `Clipboard::new()`, `clipboard.set_text(text)`, `clipboard.get_text()` (verify only). Clipboard operations per injection drop from 4 to 2.

### Expected Features

**Must have (table stakes):**
- Transcription stays on clipboard after injection -- every comparable tool does this
- Reliable clipboard paste across target apps (VS Code, Chrome, Outlook, Word, Teams) -- already built
- Clipboard verification before paste -- already built, must keep

**Should have (differentiators):**
- Transcription history with click-to-copy -- already built in v1.2; mitigates "lost clipboard" concern

**Defer (v2+):**
- Optional clipboard restore toggle (Superwhisper offers this as advanced setting, default OFF)
- `ExcludeClipboardContentFromMonitorProcessing` clipboard format for privacy-sensitive users

### Architecture Approach

Single-file modification within a stable architecture. Only `inject.rs::inject_text()` changes. Pipeline orchestration, frontend history panel, history persistence, keyboard hook, and Tauri setup are all untouched. No new components, no new data flows, no API surface changes.

**Affected component:**
1. **inject.rs::inject_text()** -- Remove save/restore/post-paste-sleep; retain verify loop, pre-paste delay, Win key release, Ctrl+V simulation

**Unchanged components:**
2. **pipeline.rs** -- Calls inject_text() with same signature via spawn_blocking
3. **HistorySection.tsx** -- Independent clipboard usage via browser API

### Critical Pitfalls

1. **Accidentally removing the verify-and-retry loop** -- Protects against Chromium WebView clipboard races, not restore races. Keep it untouched.
2. **Accidentally removing the 150ms pre-paste delay** -- Exists for Office app WM_CLIPBOARDUPDATE processing, unrelated to save/restore. Outlook pastes stale content without it.
3. **Clipboard history exposure** -- Every transcription now appears in Win+V history and potentially syncs to Microsoft cloud. Accepted behavior change matching all competitors; document in release notes.
4. **Non-text clipboard content permanently replaced** -- Was already the case (old code set empty string for non-text), but now more visible. Accepted and more honest behavior.

**Resolved conflict -- post-paste sleep:** STACK.md and ARCHITECTURE.md recommend removing the 80ms post-paste sleep (its documented purpose is restore timing). PITFALLS.md recommends keeping it (theoretical concern about subsequent clipboard operations). The code comments explicitly state the sleep is for restore timing. The next possible clipboard operation after inject_text returns is a new transcription (200ms+ minimum) or history panel copy (requires user focus switch). No realistic race exists without the restore. **Recommendation: Remove the sleep, saving 80ms per injection. Monitor during testing.**

## Implications for Roadmap

This is a single-phase milestone. The change is too small and atomic to benefit from phase splitting.

### Phase 1: Clipboard Save/Restore Removal

**Rationale:** The entire v1.3 milestone is one atomic code change -- three block deletions from a single function. No build order complexity, no sub-dependencies worth sequencing.

**Delivers:**
- 82ms injection latency reduction (unconditional, every transcription)
- ~50 lines of code removed (33% of inject_text)
- Standard dictation tool clipboard behavior
- Elimination of non-text clipboard edge case class

**Addresses features:**
- Transcription stays on clipboard (table stakes alignment with Superwhisper, Dragon, OpenWhispr)
- 80ms post-paste sleep removal (latency improvement)

**Avoids pitfalls by:**
- Keeping verify-and-retry loop untouched
- Keeping 150ms pre-paste delay untouched
- Keeping Win key release untouched
- Documenting clipboard history behavior change in release notes

**Implementation is three deletions plus doc updates:**
1. Remove line 43: `let saved: Option<String> = clipboard.get_text().ok();`
2. Remove line 132: `thread::sleep(Duration::from_millis(80));`
3. Remove lines 135-148: entire `match saved { ... }` restore block
4. Update inject_text() doc comment to reflect simplified flow
5. Update Cargo.toml comment (cosmetic)

**Testing matrix:**
- Paste into VS Code, Outlook, Word, Chrome, Teams
- Verify clipboard contains transcription after paste
- Rapid sequential dictation (two transcriptions back-to-back)
- History panel click-to-copy still works
- Win+V shows transcription in clipboard history

### Phase Ordering Rationale

- Single phase -- no meaningful sub-dependencies to sequence
- The diff is ~20 lines of deletion
- All testing can happen in one pass after the code change

### Research Flags

Phases likely needing deeper research during planning:
- **None** -- all code lines to remove and keep are identified with line numbers

Phases with standard patterns (skip research-phase):
- **Phase 1:** Deletion-only change with complete specification. No further research needed.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | No changes needed; analysis based on direct Cargo.lock and inject.rs code review |
| Features | HIGH | Competitor analysis backed by official Superwhisper docs, Dragon docs, and direct product testing |
| Architecture | HIGH | Based on direct source code analysis with line numbers; race condition analysis covers all clipboard interaction points |
| Pitfalls | HIGH | Derived from Win32 clipboard documentation and codebase analysis; one conflicting recommendation resolved via code comment analysis |

**Overall confidence:** HIGH

### Gaps to Address

- **Post-paste sleep removal validation**: Remove as recommended, but run the full testing matrix (especially rapid sequential dictation + Office apps) to confirm no regression. If paste failures appear, re-add with updated comment.
- **Clipboard history privacy**: No code mitigation in v1.3. If users report concerns, `ExcludeClipboardContentFromMonitorProcessing` is a known Win32 solution requiring raw API calls outside arboard. Track as potential v1.4 enhancement.

## Sources

### Primary (HIGH confidence)
- `src-tauri/src/inject.rs` -- direct code analysis with line-by-line review
- `src-tauri/src/pipeline.rs` -- call site analysis
- arboard 3.6.1 API -- clipboard method signatures and Win32 internals
- [Clipboard Formats -- Microsoft Win32 Docs](https://learn.microsoft.com/en-us/windows/win32/dataxchg/clipboard-formats)
- [WM_CLIPBOARDUPDATE -- Microsoft Win32 Docs](https://learn.microsoft.com/en-us/windows/win32/dataxchg/wm-clipboardupdate)
- [Superwhisper Advanced Settings](https://superwhisper.com/docs/get-started/settings-advanced) -- clipboard restore opt-in, OFF by default
- [Nuance Dictation Box](https://www.nuance.com/products/help/dragon/dragon-for-pc/enx/dps/main/Content/Dictation/using_the_dictation_box.htm) -- Dragon clipboard-paste behavior

### Secondary (MEDIUM confidence)
- [Talon Community Wiki](https://talon.wiki/Basic%20Usage/basic_usage/) -- clipboard save/restore patterns
- [OpenWhispr](https://github.com/HeroTools/open-whispr) -- clipboard paste, no restore
- [WhisperWriter](https://github.com/savbell/whisper-writer) -- keystroke simulation approach
- [CopyQ issue #2679](https://github.com/hluk/CopyQ/issues/2679) -- ExcludeClipboardContentFromMonitorProcessing format
- [PowerShell issue #17454](https://github.com/PowerShell/PowerShell/issues/17454) -- clipboard history exclusion formats

---
*Research completed: 2026-03-07*
*Ready for roadmap: yes*
