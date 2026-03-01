# Technical Research: Pill Rounded Corner Haziness Fix (Windows 10)

## Strategic Summary

The haziness around the pill's rounded corners is caused by Windows 10 applying a **default rectangular window shadow** to undecorated transparent windows. This shadow doesn't respect CSS `border-radius`, so it bleeds outside the pill shape as a visible haze. The primary fix is a one-liner: `setShadow(false)` on the pill window — confirmed by the Tauri community (issue #11321). A secondary approach using Win32 `SetWindowRgn` can provide OS-level pixel-perfect clipping if needed.

## Requirements

- Pill edges must be crisp with zero visible haziness or rectangular artifacts
- Solution must be clean and maintainable (no ugly hacks)
- Must work on Windows 10 (no Win11-only APIs)
- Must not break existing transparency, drag, or animation behavior
- Should not require switching frameworks or major architecture changes

---

## Approach 1: `setShadow(false)` — Disable Windows Default Shadow

**How it works:** Windows 10 adds a default shadow to all top-level windows, including undecorated ones. This shadow is rendered as a rectangular region by DWM (Desktop Window Manager) around the window boundary. When the pill has `transparent: true` and `border-radius: 9999px`, the rectangular shadow extends beyond the rounded CSS clip, creating the visible haze. Calling `setShadow(false)` tells the OS to stop rendering this shadow.

**Libraries/tools:** Built into Tauri 2 — no additional crates needed.

**Implementation (two options):**

### Option A — Rust side (recommended, runs once at startup):
```rust
// In lib.rs setup(), after pill_window.set_focusable(false):
let _ = pill_window.set_shadow(false);
```

### Option B — JavaScript side:
```typescript
// In Pill.tsx useEffect, on mount:
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
const appWindow = getCurrentWebviewWindow();
appWindow.setShadow(false);
```

### Required capability permission:
Add to `src-tauri/capabilities/default.json`:
```json
"core:window:allow-set-shadow"
```

**Pros:**
- One line of code — minimal change, zero new dependencies
- Directly addresses the confirmed root cause (tauri-apps/tauri#11321)
- Clean solution — no hacks, no workarounds
- No visual trade-offs (the CSS `box-shadow` on `.pill-glass` is unaffected — it's rendered inside the webview)

**Cons:**
- May not address edge anti-aliasing artifacts if any remain after shadow removal (unlikely but possible)
- `setShadow(true)` on Windows 10 has poor resize/drag performance (but we're disabling it, so N/A)

**Best when:** The haziness is caused by the OS shadow (which the screenshot strongly suggests)

**Complexity:** S

---

## Approach 2: Win32 `SetWindowRgn` + `CreateRoundRectRgn` — OS-Level Window Clipping

**How it works:** Uses the Win32 GDI API to set a non-rectangular window region. `CreateRoundRectRgn` creates a rounded rectangle region matching the pill shape, and `SetWindowRgn` clips the window to that region at the OS level. Anything outside the region (including shadows, haze, anti-aliasing bleed) is completely invisible — the OS won't render those pixels at all.

**Libraries/tools:**
- `windows` crate (Microsoft's official Rust bindings for Win32): `windows = { version = "0.58", features = ["Win32_Graphics_Gdi", "Win32_UI_WindowsAndMessaging"] }`
- Or `winapi` crate (community bindings): `winapi = { version = "0.3", features = ["wingdi", "winuser"] }`
- Also need `raw-window-handle` to get the HWND from the Tauri window

**Implementation sketch:**
```rust
use windows::Win32::Graphics::Gdi::CreateRoundRectRgn;
use windows::Win32::UI::WindowsAndMessaging::SetWindowRgn;

// Get HWND from Tauri window
let hwnd = pill_window.hwnd().unwrap(); // platform-specific

// Pill is 280x56, fully rounded = 28px corner radius
let rgn = unsafe { CreateRoundRectRgn(0, 0, 280, 56, 56, 56) };
unsafe { SetWindowRgn(hwnd, rgn, true) };
```

**Pros:**
- Absolute pixel-perfect edges — OS-level guarantee, nothing renders outside the region
- Eliminates ALL edge artifacts (shadow, anti-aliasing bleed, everything)
- Well-documented Win32 pattern, works on all Windows versions

**Cons:**
- Binary clip: pixels are either visible or invisible — no anti-aliasing on the curve edges (slight jaggedness on rounded parts)
- For a 56px-tall pill with 28px radius, the jaggedness is minimal but technically present
- Adds a Win32 crate dependency (~2-5MB compile-time, 0 runtime cost)
- Must re-apply region if window is ever resized (not applicable here since pill is fixed-size)
- More code and platform-specific logic

**Best when:** `setShadow(false)` doesn't fully resolve the issue

**Complexity:** M

---

## Approach 3: Background Opacity Boost (Supplementary)

**How it works:** The current pill background is `rgba(10, 10, 10, 0.88)` — the 12% transparency means edge pixels blend with the transparent window background, contributing to a softer/hazier edge appearance. Increasing opacity reduces this blending.

**Implementation:**
```css
/* In pill.css, change: */
.pill-glass {
  background: rgba(10, 10, 10, 0.95);  /* was 0.88 */
  /* OR fully opaque: */
  background: rgb(10, 10, 10);
}
```

**Pros:**
- Zero-effort CSS change
- Sharper edges from reduced alpha blending
- Still maintains the dark aesthetic

**Cons:**
- Doesn't fix the root cause (OS shadow) — must be combined with Approach 1
- Fully opaque loses the subtle see-through effect (at 0.88 it was barely visible anyway)
- Alone, this would only reduce haziness, not eliminate it

**Best when:** Used as a supplement to Approach 1 for maximum edge crispness

**Complexity:** S

---

## Comparison

| Aspect | Approach 1: setShadow(false) | Approach 2: SetWindowRgn | Approach 3: Opacity Boost |
|--------|------------------------------|--------------------------|---------------------------|
| Complexity | S (one line) | M (Win32 code + crate) | S (CSS change) |
| Edge Quality | Clean (shadow removed) | Pixel-perfect (hard clip) | Slightly improved |
| Anti-aliasing | Preserved (CSS handles it) | Lost (binary clip) | Preserved |
| New Dependencies | None | `windows` crate | None |
| Fixes Root Cause | Yes | Yes (different mechanism) | No (supplementary) |
| Risk | Very low | Low | None |
| Maintenance | Zero | Must re-apply on resize | Zero |

---

## Recommendation

**Start with Approach 1 (`setShadow(false)`) — it directly fixes the confirmed root cause with a single line of Rust.**

The haziness in the screenshot shows a rectangular shadow/border artifact extending beyond the pill's rounded shape, which is the exact symptom described in tauri-apps/tauri#11321. The fix is clean, requires no new dependencies, and was confirmed working by the issue reporter on Windows 10.

**Optionally combine with Approach 3** (bump opacity from 0.88 to 0.95) for maximum crispness. The 0.88 transparency was barely visible anyway, and 0.95 would be indistinguishable while reducing any residual edge blending.

**Only pursue Approach 2 (SetWindowRgn) if** Approach 1 doesn't fully resolve the issue — treat it as the escalation path.

---

## Implementation Context

<claude_context>
<chosen_approach>
- name: setShadow(false) + optional opacity boost
- libraries: none (built into Tauri 2 API)
- install: no new dependencies needed
</chosen_approach>
<architecture>
- pattern: Single call during window setup (Rust side)
- components: lib.rs setup function, capabilities/default.json
- data_flow: Rust setup → pill_window.set_shadow(false) → DWM stops rendering shadow
</architecture>
<files>
- modify: src-tauri/src/lib.rs (add set_shadow(false) call after set_focusable(false))
- modify: src-tauri/capabilities/default.json (add "core:window:allow-set-shadow" permission)
- optionally modify: src/pill.css (bump rgba opacity from 0.88 to 0.95)
</files>
<implementation>
- start_with: Add "core:window:allow-set-shadow" to capabilities/default.json
- order:
  1. Add capability permission
  2. Add pill_window.set_shadow(false) in lib.rs setup()
  3. Build and test
  4. If residual haziness, bump opacity in pill.css
  5. If still not resolved, escalate to Approach 2 (SetWindowRgn)
- gotchas:
  - setShadow is on the Window type, not WebviewWindow — in Tauri 2, get_webview_window returns WebviewWindow which derefs to Window, so the method is accessible
  - The CSS box-shadow in .pill-glass is rendered INSIDE the webview and is unaffected by OS shadow removal
  - Don't set setShadow(true) — on Windows 10 it causes performance issues during drag
- testing:
  - Build app, trigger pill overlay with hotkey
  - Visually inspect rounded corners — no rectangular haze should be visible
  - Verify drag still works (mousedown → startDragging)
  - Verify entrance/exit animations still work
  - Check that CSS box-shadow (depth shadow under pill) still renders correctly
</implementation>
</claude_context>

**Next Action:** Implement Approach 1 — add `set_shadow(false)` to the Rust setup and the capability permission, then test.

---

## Sources

- [tauri-apps/tauri #11321 — Fix Transparency and Rounded Corners Issue on Windows 10](https://github.com/tauri-apps/tauri/issues/11321) — confirmed fix via setShadow(false)
- [tauri-apps/tauri #9287 — Problems with window customization's rounded corners and shadows](https://github.com/tauri-apps/tauri/issues/9287)
- [Tauri Discussion #3219 — How to build rounded-corners frameless windows?](https://github.com/tauri-apps/tauri/discussions/3219)
- [Tauri v2 Window API reference](https://v2.tauri.app/reference/javascript/api/namespacewindow/) — setShadow, setEffects documentation
- [Tauri v2 Window Customization guide](https://v2.tauri.app/learn/window-customization/)
- [Win32 CreateRoundRectRgn — Rust windows crate](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Graphics/Gdi/fn.CreateRoundRectRgn.html)
- [Win32 SetWindowRgn documentation](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-createroundrectrgn)
- [Rounded Corners in Win32 Windows](https://www.aloneguid.uk/posts/2022/12/rounded-corners-win32/)
- [WebView2 artifacts on transparent windows — Issue #5492](https://github.com/MicrosoftEdge/WebView2Feedback/issues/5492)
