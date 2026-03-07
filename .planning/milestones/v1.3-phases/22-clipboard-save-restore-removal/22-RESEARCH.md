# Phase 22: Clipboard Save/Restore Removal - Research

**Researched:** 2026-03-07
**Domain:** Rust clipboard manipulation / Windows text injection
**Confidence:** HIGH

## Summary

This phase is a surgical code removal in a single file (`src-tauri/src/inject.rs`, 152 lines). The change removes three distinct pieces: (1) the clipboard save on line 43, (2) the 80ms post-paste sleep on line 132, and (3) the clipboard restore block on lines 134-149. The doc comment (lines 26-38) is updated to reflect the simplified flow.

The existing clipboard verification retry loop (lines 53-99) and 150ms pre-paste delay (line 111) serve orthogonal purposes (Chromium WebView races and Office app cache sync respectively) and must remain untouched. The `release_win_keys` helper and Ctrl+V simulation are also untouched.

**Primary recommendation:** Delete the three targeted code blocks, update the doc comment, verify compilation. No new dependencies, no new code paths, no architectural changes.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Transcription text always stays on clipboard regardless of prior clipboard state
- No clearing, no restoring -- clipboard simply contains what was just dictated
- Re-paste via Ctrl+V is a feature: dictate once, paste into multiple fields
- History panel covers longer-term recall
- 80ms post-paste sleep removed entirely, no replacement delay
- 150ms pre-paste delay already handles app sync (Outlook/Office cache)
- Target apps process paste from their message queue independently of inject_text return

### Claude's Discretion
- Doc comment wording for simplified flow (CLIP-03)
- Whether to add any logging about clipboard state (e.g., "clipboard now contains transcription") or just remove restore-related logs silently

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| CLIP-01 | Transcription replaces clipboard content after injection (no save/restore) | Remove line 43 (`let saved`) and lines 134-149 (restore block). Existing `set_text(text)` on line 58 already places transcription on clipboard. |
| CLIP-02 | Post-paste 80ms sleep removed (only needed for restore timing) | Remove lines 129-132 (`thread::sleep(Duration::from_millis(80))` and its comment). |
| CLIP-03 | inject_text doc comment updated to reflect simplified sequence | Rewrite lines 26-38 doc comment: sequence becomes set -> verify -> paste (3 steps, not 5). |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| arboard | 3.x | Cross-platform clipboard access | Already in use; `set_text()` and `get_text()` are the only APIs needed |
| enigo | 0.6.x | Keyboard simulation (Ctrl+V) | Already in use; no changes needed |

No new libraries required. This phase only removes code.

## Architecture Patterns

### Current inject_text Flow (Before)
```
save clipboard -> set text -> verify -> pre-paste delay -> paste -> post-paste delay -> restore
```

### Target inject_text Flow (After)
```
set text -> verify -> pre-paste delay -> paste
```

### What Gets Removed
1. **Line 43** -- `let saved: Option<String> = clipboard.get_text().ok();` -- no longer needed since we don't restore
2. **Lines 129-132** -- 80ms post-paste sleep and its comment -- only existed to give apps time to consume paste before clipboard restore overwrote the content
3. **Lines 134-149** -- The `match saved { ... }` restore block -- the entire save/restore mechanism

### What Stays (Explicitly)
- Lines 1-4: imports (all still needed: `Clipboard`, `Enigo`, `thread`, `Duration`)
- Lines 6-24: `release_win_keys()` helper
- Lines 53-99: Clipboard verification retry loop
- Line 111: 150ms pre-paste delay
- Lines 113-127: Enigo Ctrl+V simulation with Win key release

### Doc Comment Update (CLIP-03)
The doc comment should describe the simplified 3-step sequence:

```rust
/// Inject text at the current cursor position using clipboard paste.
///
/// Sequence:
///   1. Write `text` to clipboard with verify-and-retry loop (up to 5 attempts):
///      - set_text() -> sleep 25ms -> get_text() -> compare
///      - Retries on mismatch (handles Chromium WebView clipboard races)
///   2. Sleep 150ms — let Office apps sync their internal clipboard cache
///   3. Simulate Ctrl+V (with defensive Win key release)
///
/// After injection, the transcription text remains on the clipboard. Users can
/// re-paste via Ctrl+V in any application.
///
/// Intentionally synchronous — callers must wrap in tokio::task::spawn_blocking.
/// A fresh Enigo instance is created per call (do not share across invocations).
```

### Discretion: Logging
Recommendation: remove restore-related logs silently. No new "clipboard now contains transcription" log is needed -- the existing clipboard verification loop already logs on retry/failure, which covers the relevant diagnostic path. Adding a success log for every injection would create noise in normal operation.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| N/A | N/A | N/A | This phase only removes code; no new functionality |

## Common Pitfalls

### Pitfall 1: Accidentally Removing the Verification Loop
**What goes wrong:** The clipboard verification retry loop (lines 53-99) looks similar to save/restore logic and could be accidentally removed or modified.
**Why it happens:** The verification loop reads the clipboard (get_text), which looks like the save operation.
**How to avoid:** Only remove the exact lines specified: line 43 (save), lines 129-132 (post-paste sleep), lines 134-149 (restore block).
**Warning signs:** If `MAX_CLIPBOARD_RETRIES` or `clipboard_verified` variables are removed, something went wrong.

### Pitfall 2: Removing the 150ms Pre-Paste Delay Instead of the 80ms Post-Paste Delay
**What goes wrong:** Two different sleeps exist. The 150ms pre-paste delay (line 111) is essential for Office apps. The 80ms post-paste delay (line 132) is the one to remove.
**Why it happens:** Both are `thread::sleep(Duration::from_millis(...))` calls in the same function.
**How to avoid:** Remove only the sleep at line 132 (80ms, after Ctrl+V simulation), not the one at line 111 (150ms, before Ctrl+V simulation).
**Warning signs:** If the 150ms delay disappears, Outlook/Office paste reliability will regress.

### Pitfall 3: Leaving Unused Imports
**What goes wrong:** After removing the save/restore code, all existing imports are still used (Clipboard, thread, Duration are used by the verification loop and pre-paste delay). This is NOT actually a pitfall here -- all imports remain needed.
**How to avoid:** Verify with `cargo build` after changes.

## Code Examples

### Exact Lines to Remove

**Line 43 (clipboard save):**
```rust
    let saved: Option<String> = clipboard.get_text().ok();
```

**Lines 129-132 (post-paste sleep + comment):**
```rust
    // Allow target app to consume the paste before clipboard restore
    // 80ms paste consumption. Previously reduced to 50ms alongside the propagation delay
    // reduction; reverting to 80ms to match the documented fallback guidance.
    thread::sleep(Duration::from_millis(80));
```

**Lines 134-149 (restore block):**
```rust
    // Restore original clipboard content — per user decision: log failure, move on
    match saved {
        Some(original) => {
            if let Err(e) = clipboard.set_text(&original) {
                log::warn!(
                    "inject_text: clipboard restore failed: {} — clipboard contents lost",
                    e
                );
            }
        }
        None => {
            // Original was empty or non-text — clear by setting empty string
            // arboard has no explicit clear() method; empty string is the fallback
            let _ = clipboard.set_text("");
        }
    }
```

### Expected Final Function (After Changes)
```rust
pub fn inject_text(text: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;

    // [verification retry loop -- unchanged, lines 45-99]
    // ...

    // [150ms pre-paste delay -- unchanged, line 111]
    thread::sleep(Duration::from_millis(150));

    // [Enigo Ctrl+V simulation -- unchanged, lines 113-127]
    // ...

    Ok(())
}
```

The function body ends with `Ok(())` immediately after the Ctrl+V simulation block (line 127).

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Save/restore clipboard around paste | Leave transcription on clipboard | This phase | Matches Dragon, Superwhisper, OpenWhispr behavior |
| 80ms post-paste delay | No post-paste delay | This phase | Faster injection, ~80ms saved per dictation |

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust (cargo test) |
| Config file | src-tauri/Cargo.toml |
| Quick run command | `cargo build --manifest-path src-tauri/Cargo.toml` |
| Full suite command | `cargo build --manifest-path src-tauri/Cargo.toml` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CLIP-01 | No save/restore in inject_text | manual-only | Manual: dictate text, verify clipboard contains transcription via Ctrl+V in another app | N/A |
| CLIP-02 | No 80ms post-paste delay | manual-only | Manual: verify injection speed feels faster, no paste reliability regression | N/A |
| CLIP-03 | Doc comment updated | manual-only | Code review: verify doc comment describes set -> verify -> paste sequence | N/A |

**Justification for manual-only:** inject_text relies on OS clipboard and keyboard simulation (Enigo). These require a running Windows desktop session with an active window to paste into. Unit testing clipboard set/get is possible but doesn't validate the paste behavior. The project has no existing test infrastructure. The changes are pure deletion -- `cargo build` succeeding is the primary automated verification.

### Sampling Rate
- **Per task commit:** `cargo build --manifest-path src-tauri/Cargo.toml`
- **Per wave merge:** `cargo build --manifest-path src-tauri/Cargo.toml`
- **Phase gate:** Successful build + manual verification of clipboard behavior

### Wave 0 Gaps
None -- no test infrastructure needed for pure code removal. `cargo build` is sufficient to verify no compilation errors.

## Open Questions

None. This phase is fully specified with exact line numbers and clear removal targets.

## Sources

### Primary (HIGH confidence)
- Direct source code inspection of `src-tauri/src/inject.rs` (152 lines, fully read)
- CONTEXT.md with user-locked decisions
- REQUIREMENTS.md with CLIP-01, CLIP-02, CLIP-03 definitions

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - no new libraries, only removing existing code
- Architecture: HIGH - single file, exact lines identified, flow is straightforward
- Pitfalls: HIGH - only realistic risk is removing wrong lines, mitigated by exact specification

**Research date:** 2026-03-07
**Valid until:** No expiration -- this is a one-time code removal with no external dependencies
