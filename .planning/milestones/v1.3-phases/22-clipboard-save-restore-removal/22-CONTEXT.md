# Phase 22: Clipboard Save/Restore Removal - Context

**Gathered:** 2026-03-07
**Status:** Ready for planning

<domain>
## Phase Boundary

Remove clipboard save/restore logic from inject_text so transcription text stays on clipboard after injection. Remove the 80ms post-paste sleep (only needed for restore timing). Update doc comment. Matches standard dictation tool behavior (Dragon, Superwhisper, OpenWhispr).

Do NOT touch: clipboard verification retry loop (lines 53-99), 150ms pre-paste delay (line 111), Win key release logic, Ctrl+V simulation.

</domain>

<decisions>
## Implementation Decisions

### Clipboard behavior after paste
- Transcription text always stays on clipboard regardless of prior clipboard state
- No clearing, no restoring -- clipboard simply contains what was just dictated
- Re-paste via Ctrl+V is a feature: dictate once, paste into multiple fields
- History panel covers longer-term recall

### Post-paste timing
- 80ms post-paste sleep removed entirely, no replacement delay
- 150ms pre-paste delay already handles app sync (Outlook/Office cache)
- Target apps process paste from their message queue independently of inject_text return

### Claude's Discretion
- Doc comment wording for simplified flow (CLIP-03)
- Whether to add any logging about clipboard state (e.g., "clipboard now contains transcription") or just remove restore-related logs silently

</decisions>

<code_context>
## Existing Code Insights

### Target File
- `src-tauri/src/inject.rs` (152 lines) -- entire change is in this single file

### Lines to Remove
- Line 43: `let saved: Option<String> = clipboard.get_text().ok();` (clipboard save)
- Lines 129-149: Restore block (`match saved { ... }`) and its preceding comment
- Lines 130-132: `thread::sleep(Duration::from_millis(80));` (post-paste sleep)

### Lines to Keep (explicitly)
- Lines 53-99: Clipboard verification retry loop (handles Chromium WebView races)
- Line 111: `thread::sleep(Duration::from_millis(150));` (pre-paste delay for Office apps)
- Lines 17-24: `release_win_keys()` helper
- Lines 114-127: Enigo Ctrl+V simulation with Win key release

### Lines to Update
- Lines 26-38: Doc comment -- simplify sequence description to: set -> verify -> paste

</code_context>

<specifics>
## Specific Ideas

No specific requirements -- open to standard approaches.

</specifics>

<deferred>
## Deferred Ideas

None -- discussion stayed within phase scope.

</deferred>

---

*Phase: 22-clipboard-save-restore-removal*
*Context gathered: 2026-03-07*
