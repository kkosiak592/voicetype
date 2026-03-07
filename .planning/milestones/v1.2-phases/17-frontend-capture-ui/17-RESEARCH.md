# Phase 17: Frontend Capture UI - Research

**Researched:** 2026-03-03
**Domain:** React/TypeScript frontend — keyboard event capture, hotkey display formatting
**Confidence:** HIGH

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| UI-01 | Hotkey capture dialog accepts Ctrl+Win as a valid modifier-only combo without requiring a letter key | Requires `keyup` listener alongside `keydown` in `HotkeyCapture.tsx`; modifier-only state detected when all keys have been released after a modifier-only combination |
| UI-02 | Settings panel displays modifier-only combos as "Ctrl + Win" | `formatHotkey()` already handles `"meta"` → `"Win"`; the backend stores `"ctrl+meta"` which splits and maps correctly — display is already correct for any saved modifier-only combo |
</phase_requirements>

---

## Summary

Phase 17 is a surgical modification to a single component: `HotkeyCapture.tsx`. The current capture flow listens only for `keydown` events. When a modifier key fires `keydown`, `normalizeKey()` returns `null` (line 95: `// Modifier-only — wait for a real key`) and the combo is discarded. For Ctrl+Win, neither key ever produces a non-modifier base key, so capture never completes.

The fix is to add a `keyup` listener that fires after the user releases a key. When a `keyup` event fires and the released key is a modifier, and the accumulated held-modifiers (before release) form a valid modifier-only combo, that combo should be accepted. The challenge is tracking which modifiers were held at the moment of release, because by the time `keyup` fires for the last modifier, `e.ctrlKey`/`e.metaKey` flags may already reflect the released state (the key being released is no longer flagged as held in some browsers).

The display side (UI-02) is already functionally correct: `formatHotkey()` maps `"meta"` to `"Win"`, and `"ctrl+meta"`.split(`"+"`) produces `["ctrl", "meta"]` which renders as `"Ctrl + Win"`. No display changes are needed if the stored hotkey uses `"meta"` as the Win token. The backend `is_modifier_only` predicate also accepts `"meta"` as a valid modifier token, so there is no token mismatch.

**Primary recommendation:** Add a `keyup` listener to `HotkeyCapture.tsx` that tracks a `heldModifiers` set across `keydown`/`keyup` events. When the user releases and all keys are up (set becomes empty), if the pre-release set was all-modifiers and non-empty, emit that combo. This fully satisfies UI-01 with no backend or display changes required for UI-02.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| React | existing (Tauri v2 project) | Component state and effects | Already in project |
| TypeScript | existing | Type safety for event handling | Already in project |
| `@tauri-apps/api/core` `invoke` | existing | IPC to backend `rebind_hotkey`, `register_hotkey`, `unregister_hotkey` | Already used in `HotkeyCapture.tsx` |

### Supporting

No new libraries required. This phase is entirely a logic change within the existing component.

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Manual `heldModifiers` set tracking | `useRef` accumulator with `keydown`/`keyup` | Same thing — ref is appropriate for mutable state that doesn't need re-render |
| Detecting modifier-only on `keyup` of last modifier | Detecting it on `keydown` when no non-modifier key is present | `keydown` approach can't know if a base key is coming; `keyup` approach confirms the user finished their combo |

---

## Architecture Patterns

### Existing Component Structure

```
src/components/HotkeyCapture.tsx
  normalizeKey(e: KeyboardEvent): string | null   — extracts "ctrl+a" etc from keydown
  HotkeyCapture({ value, onChange })
    state: listening, error
    ref: boxRef
    useEffect(unregister on listen start)
    useEffect(keydown handler + click-away while listening)
    formatHotkey(hotkey: string): string           — "ctrl+meta" → "Ctrl + Win"
```

### Pattern 1: Dual-Event Capture (keydown + keyup)

**What:** Listen for both `keydown` and `keyup` while in capture mode. Use a `useRef`-held `Set<string>` to track which modifier tokens are currently held. On `keydown`, add the token and proceed with existing standard-hotkey logic (non-modifier base key pressed → emit combo). On `keyup`, check if the held set was all-modifiers; if so, emit the modifier-only combo.

**When to use:** Any time a hotkey capture UI must support combos that contain no base key.

**Example:**

```typescript
// Track held modifiers across events — useRef so mutations don't trigger re-render
const heldRef = useRef<Set<string>>(new Set());

// Inside useEffect that sets up listeners while listening===true:

const handleKeyDown = async (e: KeyboardEvent) => {
  e.preventDefault();
  e.stopPropagation();

  if (e.code === 'Escape') {
    heldRef.current.clear();
    setListening(false);
    if (value) invoke('register_hotkey', { key: value }).catch(() => {});
    return;
  }

  const modifierForCode = modifierToken(e.code); // e.g. 'ControlLeft' -> 'ctrl'
  if (modifierForCode) {
    heldRef.current.add(modifierForCode);
    return; // wait — could be start of modifier-only OR modifier+key combo
  }

  // Non-modifier key pressed — standard combo path
  heldRef.current.clear();
  const combo = normalizeKey(e);
  if (!combo) return;
  // ... existing save/rebind logic
};

const handleKeyUp = async (e: KeyboardEvent) => {
  e.preventDefault();
  e.stopPropagation();

  const modifierForCode = modifierToken(e.code);
  if (!modifierForCode) {
    heldRef.current.clear();
    return;
  }

  // A modifier was released. The set holds what was down BEFORE this release.
  const combo = [...heldRef.current].join('+');
  heldRef.current.delete(modifierForCode);

  // If set is now empty (all modifiers released) and we had a valid combo, emit it.
  if (heldRef.current.size === 0 && combo.length > 0) {
    setListening(false);
    setError(null);
    if (combo === value) {
      invoke('register_hotkey', { key: value }).catch(() => {});
      return;
    }
    try {
      await invoke('rebind_hotkey', { old: '', newKey: combo });
      const store = await getStore();
      await store.set('hotkey', combo);
      onChange(combo);
    } catch (err) {
      if (value) invoke('register_hotkey', { key: value }).catch(() => {});
      setError(String(err));
    }
  }
};

window.addEventListener('keydown', handleKeyDown, true);
window.addEventListener('keyup', handleKeyUp, true);
return () => {
  window.removeEventListener('keydown', handleKeyDown, true);
  window.removeEventListener('keyup', handleKeyUp, true);
};
```

**Critical detail on `keyup` modifier detection:** When `keyup` fires for a modifier key, `e.ctrlKey` / `e.metaKey` reflect the state AFTER release (i.e., they are already `false` for the key being released). Use `e.code` to identify the key, not `e.ctrlKey`. Map `e.code` to modifier token:

```typescript
function modifierToken(code: string): string | null {
  if (code === 'ControlLeft' || code === 'ControlRight') return 'ctrl';
  if (code === 'MetaLeft' || code === 'MetaRight') return 'meta';
  if (code === 'AltLeft' || code === 'AltRight') return 'alt';
  if (code === 'ShiftLeft' || code === 'ShiftRight') return 'shift';
  return null;
}
```

**Token compatibility:** The frontend emits `"meta"` for the Win key. The backend `is_modifier_only` predicate accepts `["ctrl", "alt", "shift", "meta", "win", "super"]` — all lowercase. `"ctrl+meta"` passes the predicate. `formatHotkey` maps `"meta"` → `"Win"` for display. No token changes needed.

### Pattern 2: Prompt Text Update During Capture

**What:** The current prompt text is `'Press a key combo...'`. With modifier-only support, a progressive prompt like `'Press modifiers, then release to confirm...'` or an accumulating display like `'Ctrl + Win...'` as keys are held gives better UX.

**When to use:** When users may not know they can release without pressing a letter.

**Example:**

```typescript
// Show held keys as they accumulate during capture
const [heldDisplay, setHeldDisplay] = useState<string>('');

// In handleKeyDown, after adding to heldRef:
if (modifierForCode) {
  heldRef.current.add(modifierForCode);
  setHeldDisplay(formatHotkey([...heldRef.current].join('+')));
  return;
}

// In handleKeyUp (when releasing without completing):
setHeldDisplay('');

// In render:
{listening
  ? heldDisplay
    ? `${heldDisplay}...`
    : 'Press a key combo...'
  : formatHotkey(value)}
```

### Anti-Patterns to Avoid

- **Using `e.ctrlKey`/`e.metaKey` flags on `keyup` to detect which modifier was released:** These flags reflect the state AFTER the key fires. On `keyup` for a Ctrl key, `e.ctrlKey` is already `false`. Use `e.code` instead.
- **Completing combo on keydown for modifier-only case:** Cannot know on `keydown` whether a base key is coming. Must wait for `keyup`.
- **Clearing `heldRef` on every `keyup` before checking combo:** Delete only the released key after reading the pre-release set.
- **Forgetting to clear `heldRef` on Escape or click-away:** Stale modifier state persists across capture sessions if not cleaned on cancel paths.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Hotkey parsing/formatting | New parser | Extend existing `normalizeKey` + `formatHotkey` in `HotkeyCapture.tsx` | They already handle the full modifier+key case; only need to add modifier-only path |
| Backend modifier detection | JS predicate | Rely on backend `is_modifier_only` via error response from `rebind_hotkey` | Backend is the authoritative gate; if an invalid combo is sent, it will error |

**Key insight:** The backend `rebind_hotkey` will error if a modifier-only combo is sent when the hook is unavailable. The frontend already surfaces this error via `setError(String(err))`. No additional validation layer needed in the frontend — trust the backend error path.

---

## Common Pitfalls

### Pitfall 1: `keyup` modifier flag semantics

**What goes wrong:** Developer reads `e.ctrlKey` in the `keyup` handler for `ControlLeft` event and sees `false`, concludes Ctrl was not held, drops the combo.

**Why it happens:** The `ctrlKey` flag reflects the keyboard state at the time the event fires. On `keyup` for Ctrl, Ctrl is no longer held, so `e.ctrlKey === false`.

**How to avoid:** Track modifiers via `e.code` in a `Set`. Add on `keydown`, read-then-delete on `keyup`. The pre-delete state of the set is the "what was held" answer.

**Warning signs:** Modifier-only capture never fires; adding `console.log(e.ctrlKey)` in keyup shows `false`.

### Pitfall 2: Combo emitted when modifier held and base key pressed simultaneously

**What goes wrong:** User presses Ctrl, then A. `keydown` for Ctrl adds to set. `keyup` for Ctrl fires before A's `keydown` (unusual but possible with fast key release). Set has only `["ctrl"]` and emits `"ctrl"` as a combo.

**Why it happens:** Race between key release timings.

**How to avoid:** On any non-modifier `keydown`, clear `heldRef` immediately before proceeding to the standard path. This prevents a spurious `keyup` combo from firing after the standard combo has already been accepted. Also: once a standard combo has been accepted and `listening` is set to `false`, the `keyup` listener is already removed.

**Warning signs:** Ctrl+A combo sometimes registers as just `"ctrl"`.

### Pitfall 3: Win key `keydown` not firing in browser context

**What goes wrong:** On Windows, the Win key `keydown` event may be swallowed by the OS (Start menu intercept) before the WebView receives it.

**Why it happens:** Windows intercepts the Win key at the OS level for Start menu. However, in the Tauri WebView during capture mode, Ctrl is being held simultaneously, which suppresses Start menu via the backend's VK_E8 injection (Phase 15 MOD-04). But the hook is unregistered during capture (unregister_hotkey is called when `listening` becomes true), so MOD-04 suppression is NOT active during capture.

**Critical implication:** When the capture dialog is open and `unregister_hotkey` has been called, the keyboard hook is uninstalled. This means Ctrl+Win pressing during capture will NOT be suppressed by the hook — the Win key keydown may be swallowed by the OS or fire inconsistently.

**How to handle:** Test empirically. The Tauri WebView may still receive the events because the hook uninstall only affects the app's own recording behavior, not the WebView's DOM event reception. But if `metaKey` does not appear in `keydown` events during capture, an alternative approach is needed:

- Option A: Keep hook installed during capture (do not call `unregister_hotkey` for modifier-only combos), but mute the recording action. This requires a backend "capture mode" flag that the hook checks before dispatching.
- Option B: Use a `keyup`-only approach where Win key `keyup` is more reliably delivered even if `keydown` is intercepted.

**Warning signs:** Ctrl+Win combo never captured; console shows no `keydown` with `e.metaKey === true` or `e.code === 'MetaLeft'`.

### Pitfall 4: `heldRef` not reset when capture is cancelled

**What goes wrong:** User opens capture, presses Ctrl, clicks away (cancel). `heldRef` still contains `"ctrl"`. User opens capture again, releases Ctrl (was still physically held) — `keyup` fires, set becomes empty, emits `"ctrl"` as a combo spuriously.

**How to avoid:** Clear `heldRef.current` on all cancel paths: Escape handler, click-outside handler, and the cleanup function of the `useEffect`.

---

## Code Examples

### Modifier token mapper (new helper, extracted)

```typescript
// Source: MDN KeyboardEvent.code values (standard)
function modifierToken(code: string): string | null {
  switch (code) {
    case 'ControlLeft':
    case 'ControlRight':
      return 'ctrl';
    case 'MetaLeft':
    case 'MetaRight':
      return 'meta';
    case 'AltLeft':
    case 'AltRight':
      return 'alt';
    case 'ShiftLeft':
    case 'ShiftRight':
      return 'shift';
    default:
      return null;
  }
}
```

### Updated normalizeKey (unchanged in signature, unchanged behavior for standard combos)

The existing `normalizeKey` can remain exactly as-is. It handles standard combos (modifier + base key). The modifier-only path is handled separately in the `keyup` listener. No changes needed.

### formatHotkey already handles "meta" → "Win"

```typescript
// Existing code in HotkeyCapture.tsx — already correct for UI-02
function formatHotkey(hotkey: string): string {
  return hotkey
    .split('+')
    .map((part) => {
      switch (part) {
        case 'ctrl': return 'Ctrl';
        case 'alt': return 'Alt';
        case 'shift': return 'Shift';
        case 'meta': return 'Win';   // ← already maps "meta" to "Win"
        case 'space': return 'Space';
        default: return part.toUpperCase();
      }
    })
    .join(' + ');
}
// "ctrl+meta" → "Ctrl + Win"  ✓
```

### unregister_hotkey during capture — modifier-only consideration

```typescript
// Existing useEffect in HotkeyCapture.tsx:
useEffect(() => {
  if (listening && value) {
    invoke('unregister_hotkey', { key: value }).catch(() => {});
  }
}, [listening, value]);
```

For modifier-only combos, `unregister_hotkey` calls `handle.uninstall()` on the keyboard hook. This means during capture mode, the hook is not active. The implication for Pitfall 3 must be validated during implementation.

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| keydown-only capture, modifier-only discarded (line 95 comment) | keydown + keyup dual listener with held-set tracking | Phase 17 | Allows Ctrl+Win and any modifier-only combo |

**Deprecated/outdated:**
- `// Modifier-only — wait for a real key` comment on line 95 of `HotkeyCapture.tsx`: This was intentional for standard hotkeys, becomes wrong for modifier-only support. The comment and the `return null` guard will need to be retained for the standard path but the keyup handler adds the modifier-only path.

---

## Open Questions

1. **Win key `keydown` delivery in Tauri WebView during capture mode**
   - What we know: Backend hook is uninstalled during capture (`unregister_hotkey` takes handle, calls `uninstall()`). Without hook active, MOD-04 (Start menu suppression via VK_E8) is not running.
   - What's unclear: Does the Tauri WebView (WebView2 on Windows) receive `keydown` for the Win/Meta key when the OS Start menu intercept is in play? Empirical testing has not been documented. The Win key behavior in browser contexts on Windows is non-standard.
   - Recommendation: Implement the `keyup`-based approach first. `keyup` events are typically delivered even when `keydown` is intercepted. If Win `keydown` is needed for the held-display (Pattern 2 UX enhancement), this can be tested separately. The core requirement (capturing the combo on release) should work via `keyup` alone.

2. **Order of tokens in modifier-only combo string**
   - What we know: The `heldRef` is a `Set<string>`. Sets in JavaScript maintain insertion order. If user presses Ctrl first, `"ctrl"` is added first; spread gives `["ctrl", "meta"]` → `"ctrl+meta"`. If Win pressed first, `"meta"` is first → `"meta+ctrl"`.
   - What's unclear: Does the backend `rebind_hotkey` or `is_modifier_only` care about token order? Looking at `is_modifier_only`, it only checks that ALL tokens are modifiers — order is irrelevant. The keyboard hook (currently hardcoded to ctrl+win) also does not parse the hotkey string.
   - Recommendation: Sort the tokens alphabetically or canonicalize to ctrl-first before joining, so the stored value is deterministic regardless of press order. This also ensures `formatHotkey` produces a consistent display.

---

## Sources

### Primary (HIGH confidence)

- Direct code read of `src/components/HotkeyCapture.tsx` — full component implementation
- Direct code read of `src-tauri/src/lib.rs` lines 115-125 (`is_modifier_only`), 529-695 (hotkey IPC commands)
- Direct code read of `src-tauri/src/keyboard_hook.rs` — hook architecture, modifier detection
- Direct code read of `src/components/sections/GeneralSection.tsx` — `hookAvailable` warning display
- Direct code read of `src/App.tsx` — `hotkey` state flow, `get_hook_status` IPC call

### Secondary (MEDIUM confidence)

- MDN standard: `KeyboardEvent.code` values for modifier keys (`ControlLeft`, `ControlRight`, `MetaLeft`, `MetaRight`, `AltLeft`, `AltRight`, `ShiftLeft`, `ShiftRight`) — well-established browser standard, highly stable
- MDN standard: `e.ctrlKey`/`e.metaKey` flags reflect post-release state on `keyup` — documented browser behavior

### Tertiary (LOW confidence — needs empirical validation)

- Win key `keydown` delivery in Tauri WebView2 without hook active: Not documented; needs runtime testing during implementation

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new libraries, existing codebase read directly
- Architecture: HIGH — dual-event pattern is well-understood browser standard; modifier token semantics confirmed from source
- Pitfalls: HIGH for keyup flag semantics (browser standard); MEDIUM for Win key interception (empirical only)
- Display (UI-02): HIGH — `formatHotkey` already correct, backend token accepted

**Research date:** 2026-03-03
**Valid until:** 2026-04-03 (stable browser standards, no external dependencies)
