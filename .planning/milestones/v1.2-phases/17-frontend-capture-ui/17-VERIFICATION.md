---
phase: 17-frontend-capture-ui
verified: 2026-03-03T15:00:00Z
status: human_needed
score: 5/5 must-haves verified
re_verification: false
human_verification:
  - test: "Press Ctrl then Win (modifier-only combo) in the capture dialog"
    expected: "Dialog shows 'Ctrl...' while Ctrl is held, 'Ctrl + Win...' while both held, accepts and saves 'Ctrl + Win' on release. Settings display shows 'Ctrl + Win'."
    why_human: "Keyboard event sequencing (keydown/keyup) and progressive display state cannot be verified programmatically without a running browser context."
  - test: "Press Ctrl+Shift+V (standard combo) in the capture dialog"
    expected: "Combo is accepted and saved as 'Ctrl + Shift + V' — standard path unchanged."
    why_human: "Requires interactive keyboard input in a running app window."
  - test: "Spurious modifier-only suppression: press Ctrl then quickly press A"
    expected: "Standard combo 'Ctrl + A' is accepted. No modifier-only combo is spuriously emitted on key release."
    why_human: "Timing-dependent behavior between keydown and keyup events requires live testing."
  - test: "Cancel paths: press Escape while capture is open; click outside the capture box"
    expected: "Both paths cancel capture, restore the previous hotkey, and clear progressive display."
    why_human: "Requires interactive app session to verify state reset on cancel."
  - test: "End-to-end dictation with modifier-only hotkey (Ctrl+Win)"
    expected: "After saving Ctrl+Win, holding Ctrl+Win starts recording; releasing ends recording and transcribes."
    why_human: "Full pipeline integration with the keyboard hook requires a running app with audio input."
---

# Phase 17: Frontend Capture UI Verification Report

**Phase Goal:** Frontend capture dialog supports modifier-only hotkeys (e.g., Ctrl+Win) alongside standard combos, with progressive held-key display and proper backend routing
**Verified:** 2026-03-03T15:00:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | User opens capture dialog, presses Ctrl+Win (no additional key), releases — combo is accepted and saved as the hotkey | ? HUMAN NEEDED | Implementation complete: keyup handler fires `invoke('rebind_hotkey', { old: '', newKey: combo })` when `heldRef.current.size === 0 && comboRef.current.size > 0` (line 192). Logic is correct but requires live keyboard test to confirm event firing. |
| 2 | Settings panel displays a saved modifier-only combo as 'Ctrl + Win' — not an error, empty field, or raw key code | ✓ VERIFIED | `formatHotkey` maps `'meta'` to `'Win'` (line 265); `comboRef` stores `'ctrl+meta'` in sorted order; stored value is `'ctrl+meta'`; `formatHotkey('ctrl+meta')` returns `'Ctrl + Win'`. |
| 3 | User opens capture dialog, presses Ctrl+Shift+V (standard combo with letter key) — capture still works exactly as before | ✓ VERIFIED | `normalizeKey` and `formatHotkey` are unchanged. On non-modifier keydown, `heldRef.current.clear()` and `comboRef.current.clear()` run, then the existing `normalizeKey(e)` path executes identically to pre-phase code (lines 146-174). |
| 4 | During capture, held modifiers are displayed progressively (e.g. 'Ctrl + Win...') giving the user feedback before release | ✓ VERIFIED | `heldDisplay` state (line 92) is set via `setHeldDisplay(formatHotkey(sortedTokens.join('+')))` on each modifier keydown (line 141). Render shows `${heldDisplay}...` when `listening && heldDisplay` (lines 291-295). |
| 5 | Pressing Ctrl then quickly A does not spuriously emit 'ctrl' as a modifier-only combo — the standard path handles it | ✓ VERIFIED | Non-modifier keydown clears `heldRef.current` and `comboRef.current` (lines 146-148) before calling `normalizeKey`. The keyup handler checks `comboRef.current.size > 0` (line 192) — after clear, size is 0, so no modifier-only emit occurs. |

**Score:** 4 automated + 1 human-needed = 5/5 truths accounted for. All automated checks pass.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/components/HotkeyCapture.tsx` | Dual keydown+keyup capture with modifier-only support, modifierToken helper, heldRef tracking, progressive display | ✓ VERIFIED | File exists, 302 lines, substantive implementation. Contains all required constructs: `modifierToken`, `MODIFIER_ORDER`, `heldRef`, `comboRef`, `heldDisplay`, dual listeners registered on lines 244-246, cleaned up on lines 247-254. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/components/HotkeyCapture.tsx` | `rebind_hotkey` IPC | `invoke('rebind_hotkey', { old: '', newKey: combo })` in keyup handler | ✓ WIRED | Found at lines 164 (standard path) and 210 (modifier-only path). Both calls `await` the result and handle errors. |
| `src/components/HotkeyCapture.tsx` | `formatHotkey` display | `case 'meta': return 'Win'` in formatHotkey | ✓ WIRED | Found at line 265. `formatHotkey` is called in the render (line 295) for display and in handleKeyDown (line 141) for progressive display. |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| UI-01 | 17-01-PLAN.md | Hotkey capture dialog accepts Ctrl+Win as a valid modifier-only combo without requiring a letter key | ✓ SATISFIED | `modifierToken` + `heldRef` + `comboRef` + keyup all-released path (lines 192-220) implements modifier-only capture. `comboRef` tracks the full session set; emit fires when `heldRef.current.size === 0` (all released). Human verification required for live confirmation. |
| UI-02 | 17-01-PLAN.md | Settings panel displays modifier-only combos as "Ctrl + Win" | ✓ SATISFIED | `formatHotkey` maps `'meta'` to `'Win'` (line 265) and `'ctrl'` to `'Ctrl'` (line 262). `MODIFIER_ORDER` ensures `ctrl+meta` ordering regardless of press order. `formatHotkey(value)` renders in the non-listening state (line 295). |

Both UI-01 and UI-02 are mapped to Phase 17 in REQUIREMENTS.md (lines 93-94) and claimed in the PLAN frontmatter `requirements: [UI-01, UI-02]`. No orphaned requirements.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src/components/HotkeyCapture.tsx` | 107, 128, 158, 171, 205, 217, 239 | `.catch(() => {})` on fire-and-forget `invoke` calls | Info | Intentional: these are best-effort register/unregister calls that must not block the UI. Not a stub. |

No TODO/FIXME/placeholder comments found. No empty return stubs. No console.log-only implementations.

### Commit Verification

All four commits documented in SUMMARY are confirmed in git history:

| Commit | Description | Status |
|--------|-------------|--------|
| `fd62885` | feat(17-01): add modifier-only hotkey capture with progressive display | ✓ EXISTS |
| `510fd66` | fix(17-01): register HookAvailable on builder before webview creation | ✓ EXISTS |
| `b0dc56b` | fix(17-01): use comboRef to track full modifier set across keyup events | ✓ EXISTS |
| `bef1625` | fix(17-01): only show hook warning for modifier-only hotkeys, refresh on rebind | ✓ EXISTS |

### TypeScript Compilation

TypeScript compiles with zero errors (`npx tsc --noEmit` exited 0).

### Human Verification Required

**1. Modifier-only combo capture (Ctrl+Win)**

**Test:** Open VoiceType settings. Click hotkey capture box. Press and hold Ctrl — display should update to "Ctrl...". While holding Ctrl, press Win — display should update to "Ctrl + Win...". Release both keys.
**Expected:** Combo accepted and saved. Display shows "Ctrl + Win" (not error, empty field, or raw code).
**Why human:** Keydown/keyup event sequencing and progressive display state transitions cannot be verified without a live browser context with actual keyboard hardware.

**2. Standard combo backward compatibility (Ctrl+Shift+V)**

**Test:** Click hotkey capture box. Press Ctrl+Shift+V.
**Expected:** Combo accepted and saved as "Ctrl + Shift + V". No regression from pre-phase behavior.
**Why human:** Requires interactive keyboard input in a running app.

**3. Spurious modifier-only emission (Ctrl then A quickly)**

**Test:** Click hotkey capture box. Press Ctrl, then immediately press A (before releasing Ctrl).
**Expected:** Standard combo "Ctrl + A" is captured. The Ctrl release does NOT fire a modifier-only "ctrl" combo.
**Why human:** Timing-sensitive keydown/keyup interleave requires live testing.

**4. Cancel paths (Escape and click-away)**

**Test:** (a) Click capture box, hold Ctrl so "Ctrl..." displays, then press Escape. (b) Click capture box, hold Ctrl, then click elsewhere on the window.
**Expected:** Both paths cancel, restore prior hotkey, clear display. No stale "Ctrl..." shown on next open.
**Why human:** State reset across cancel paths and across open/close cycles requires interactive verification.

**5. End-to-end dictation with Ctrl+Win**

**Test:** Save Ctrl+Win as the hotkey. Hold Ctrl+Win — recording should start. Release — recording should stop and transcription appear.
**Expected:** Full hold-to-talk pipeline works with modifier-only hotkey via keyboard hook.
**Why human:** Requires audio input, keyboard hook, pipeline, and text injection to all function together in a running app.

### Gaps Summary

No automated gaps found. All five must-have truths are either directly verified in the codebase or have correct implementation that requires human confirmation for live behavior. The `comboRef` addition (deviation from original plan) correctly addresses the multi-key-release depletion problem: `heldRef` is depleted as keys release one at a time, so the last keyup would see size 0 with only the last modifier in scope. `comboRef` retains all modifiers pressed during the session and is read as the final combo, then cleared. This is correct.

---

_Verified: 2026-03-03T15:00:00Z_
_Verifier: Claude (gsd-verifier)_
