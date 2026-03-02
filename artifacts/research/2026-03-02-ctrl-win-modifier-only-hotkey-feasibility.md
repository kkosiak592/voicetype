# Feasibility Assessment: Ctrl+Win Modifier-Only Hotkey

## Strategic Summary

**Not feasible with the current hotkey stack.** Every layer of the system — from the Windows `RegisterHotKey` API, through the `global-hotkey` Rust crate, the `tauri-plugin-global-shortcut` plugin, and down to the frontend `HotkeyCapture.tsx` component — explicitly requires a non-modifier "base" key. Achieving Ctrl+Win-only would require replacing the entire hotkey subsystem with a custom low-level keyboard hook, and even then the Windows key's deep OS integration creates reliability problems.

## What we're assessing

Using **Ctrl + Windows key** (two modifier keys, no additional base key) as the global hotkey trigger for the voice-to-text app, replacing the current `ctrl+shift+space` default.

## Technical Feasibility

**Can we build it?**

### Layer-by-layer analysis

| Layer | Supports modifier-only? | Details |
|-------|------------------------|---------|
| **Windows `RegisterHotKey` API** | No | Requires `fsModifiers` + `vk` (virtual key code). Modifier keys cannot be the `vk` parameter. |
| **`global-hotkey` crate** (Tauri's underlying lib) | No | API: `HotKey::new(Some(Modifiers), Code)` — `Code` is always required, cannot be a modifier. |
| **`tauri-plugin-global-shortcut`** | No | Passes through to `global-hotkey`. No modifier-only support. |
| **`HotkeyCapture.tsx` (frontend)** | No | Lines 50-53: explicitly returns `null` for modifier-only key presses. Line 95: skips null results with comment "Modifier-only — wait for a real key". |
| **`win-hotkeys` crate** (alternative) | No | Requires a trigger key + modifier array. `VKey::LWin` cannot be the trigger key. |

### The Win key problem

The Windows key is special at the OS level:
- **Start menu activation**: Windows intercepts Win key press/release to toggle the Start menu
- **Reserved combos**: `Win+L` (lock), `Win+D` (desktop), `Win+E` (explorer), `Win+R` (run) are hardcoded in the shell
- **`Ctrl+Win` specifically**: Already partially reserved — `Ctrl+Win+Arrow` switches virtual desktops
- **Even with low-level hooks**: Suppressing the Win key's Start menu behavior is unreliable and can break user expectations

### What it would take

To implement Ctrl+Win-only, you'd need to:

1. **Replace `tauri-plugin-global-shortcut` entirely** with a custom Rust module
2. **Install a WH_KEYBOARD_LL low-level keyboard hook** via the `windows` crate
3. **Track modifier key state manually** (keydown/keyup for VK_LCONTROL, VK_LWIN)
4. **Detect the "both pressed, nothing else" condition** with timing/debounce logic
5. **Suppress the Win key's Start menu activation** (unreliable — requires eating the keyup event)
6. **Rewrite `HotkeyCapture.tsx`** to support modifier-only capture mode
7. **Handle edge cases**: key repeat, left vs right modifier keys, focus changes during key hold, other apps' hooks competing

Estimated effort: 2-4 days of development + ongoing maintenance of a custom Windows hook system.

- Known approaches: **Partial** — low-level hooks CAN detect modifier-only combos, but suppressing OS behavior is unreliable
- Technology maturity: **Experimental** — no established Rust crate supports this pattern cleanly
- Technical risks:
  - **High**: Win key Start menu still fires on key release in many scenarios
  - **Medium**: Low-level hooks can be flagged by antivirus software
  - **Medium**: Hook ordering conflicts with other apps (Discord, OBS, gaming overlays)
  - **Low**: Increased CPU usage from processing every keystroke through the hook
- **Technical verdict: Not feasible** (with current stack) / **Risky** (with custom hook)

## Resource Feasibility

**Do we have what we need?**

- Skills: Need deep Windows API knowledge (WH_KEYBOARD_LL, virtual key codes, message pumps)
- Budget: Development time is the main cost — 2-4 days plus ongoing maintenance
- Tools/infrastructure: `windows` crate is available; no additional infrastructure needed
- **Resource verdict: Feasible** (if willing to invest the time)

## External Dependency Feasibility

**Are external factors reliable?**

- Windows OS behavior: **Unreliable** — Microsoft controls Win key behavior and could change it in updates
- Antivirus interaction: **Risky** — some security software blocks low-level keyboard hooks
- Other applications: **Risky** — apps like AutoHotkey, Discord, gaming overlays may conflict
- **External verdict: Risky**

## Blockers

| Blocker | Severity | Mitigation |
|---------|----------|------------|
| `RegisterHotKey` API doesn't support modifier-only combos | **High** | Must use low-level keyboard hook (WH_KEYBOARD_LL) instead — complete rewrite of hotkey subsystem |
| Win key triggers Start menu on release | **High** | Can suppress with low-level hook by consuming WM_KEYUP for VK_LWIN, but unreliable across Windows versions and update cycles |
| `global-hotkey` / `tauri-plugin-global-shortcut` require a base key | **High** | Must bypass the plugin entirely; lose cross-platform support |
| Frontend `HotkeyCapture.tsx` filters out modifier-only presses | **Low** | Straightforward code change — remove the null guard on lines 50-53 |
| Antivirus false positives on low-level hooks | **Medium** | Code signing helps; may still get flagged by aggressive AV |

## De-risking Options

- **Use Ctrl+Win+[key] instead**: Trivially supported by the existing stack. `ctrl+meta+space` or `ctrl+meta+x` would work today with zero code changes. Best risk/reward ratio.
- **Prototype the low-level hook approach**: Build a standalone Rust binary that installs WH_KEYBOARD_LL, detects Ctrl+Win, and suppresses Start menu. Test on your machine before integrating into Tauri. ~4 hours to validate.
- **Use a different two-key combo**: `ctrl+space`, `alt+space`, or `ctrl+\`` are modifier+key combos that feel lightweight but work with the existing system.

## Overall Verdict

**No-go** for Ctrl+Win modifier-only with the current architecture.

**Go with conditions** if willing to either:
1. **Accept Ctrl+Win+[key]** (e.g., `ctrl+meta+space`) — works today, zero risk, zero effort
2. **Invest in a custom low-level hook** — 2-4 days of work, ongoing maintenance burden, unreliable Win key suppression, loss of cross-platform support

The honest assessment: the modifier-only approach is a disproportionate amount of work for a minor ergonomic preference, with reliability concerns that can't be fully resolved. Option 1 (adding a third key) gets you 95% of the way there.

## Implementation Context

<claude_context>
<if_go>
- approach: For Ctrl+Win+[key] — just set the hotkey string to "ctrl+meta+space" (or similar). The existing `tauri-plugin-global-shortcut` and `HotkeyCapture.tsx` already support `meta` as a modifier. No code changes needed.
- start_with: Test `ctrl+meta+space` via the existing Settings UI hotkey capture
- critical_path: Verify `tauri-plugin-global-shortcut` correctly maps "meta" to the Win key on Windows (it does — uses `Modifiers::SUPER`)
</if_go>
<risks>
- technical: Win key modifier may conflict with some Windows shell shortcuts
- external: None for the Ctrl+Win+[key] approach
- mitigation: Choose a third key that doesn't conflict (Space, backtick, F-keys are safe)
</risks>
<alternatives>
- if_blocked: Use Ctrl+Space (no Win key) — simpler, no OS conflicts
- simpler_version: Ctrl+Win+Space — works today with existing infrastructure
</alternatives>
</claude_context>

**Next Action:** Try setting your hotkey to `Ctrl+Win+Space` in the existing settings UI — it should work immediately with no code changes.

## Sources

- [RegisterHotKey function (winuser.h) — Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerhotkey) — 2026-03-02
- [global-hotkey crate — docs.rs](https://docs.rs/global-hotkey/latest/global_hotkey/hotkey/struct.HotKey.html) — 2026-03-02
- [tauri-plugin-global-shortcut — Tauri v2 docs](https://v2.tauri.app/plugin/global-shortcut/) — 2026-03-02
- [win-hotkeys crate — GitHub](https://github.com/iholston/win-hotkeys) — 2026-03-02
- [global-hotkey crate — GitHub (Tauri)](https://github.com/tauri-apps/global-hotkey) — 2026-03-02
- [globalShortcut API discussion — Tauri GitHub #7121](https://github.com/tauri-apps/tauri/discussions/7121) — 2026-03-02
