# Feasibility Assessment: WH_KEYBOARD_LL Hook for Ctrl+Win Modifier-Only Hotkey

## Strategic Summary

**Go with conditions.** The codebase is surprisingly well-suited for this integration. The `windows` crate is already a dependency, the `handle_shortcut()` interface cleanly separates detection from action, long-lived background threads with Arc/AtomicBool signaling are an established pattern, and the Webview2 main thread already runs a Win32 message loop. The primary risk is Win key Start menu suppression reliability across Windows 10/11 builds — mitigated by a half-day standalone prototype before full integration.

## What we're assessing

Replacing `tauri-plugin-global-shortcut` (which uses `RegisterHotKey` API) with a custom `WH_KEYBOARD_LL` low-level keyboard hook module, enabling **Ctrl+Win modifier-only** hotkey support — the same approach Wispr Flow uses on Windows.

## Technical Feasibility

**Can we build it?**

### Codebase alignment (what works in our favor)

| Factor | Status | Detail |
|--------|--------|--------|
| `windows` crate already in Cargo.toml | Present (v0.58) | Only needs additional feature flags — no new dependency |
| `handle_shortcut()` interface | Clean abstraction | Takes Pressed/Released events, doesn't care about detection mechanism (lib.rs:367) |
| Background thread pattern | Proven | Audio capture, VAD workers, Parakeet warmup all spawn long-lived threads |
| Arc/AtomicBool state signaling | Established | `LevelStreamActive` (lib.rs:62) demonstrates exact pattern needed for hook→main thread communication |
| Win32 message loop | Already running | Webview2 COM init pumps message loop on main thread (lib.rs:1301) |
| AppHandle cross-thread usage | Proven | Tauri v2 AppHandle is Send — already cloned into async tasks for `emit_to()` calls |
| Plugin isolation | Minimal surface | Global shortcut plugin is ~10 lines setup (1521-1530) + 3 command wrappers (494-543) |
| Unsafe Windows API precedent | Exists | `transcribe.rs:44-100` — DXGI COM calls with proper error handling pattern |

### Required changes

| Component | Change | LOC estimate |
|-----------|--------|-------------|
| **New `keyboard_hook.rs` module** | Install WH_KEYBOARD_LL, track Ctrl+Win key state, signal Pressed/Released via channel | ~150-200 |
| **Cargo.toml** | Add `Win32_UI_WindowsAndMessaging`, `Win32_Foundation` features | ~3 |
| **lib.rs setup()** | Replace plugin registration (lines 1521-1530) with hook initialization | ~20 |
| **lib.rs commands** | Update `rebind_hotkey`/`unregister_hotkey`/`register_hotkey` to work with hook | ~30 |
| **HotkeyCapture.tsx** | Remove modifier-only null guard (lines 50-53), accept `ctrl+meta` as valid combo | ~10 |
| **lib.rs shutdown** | Add hook cleanup in `on_window_event` or Drop impl | ~10 |
| **Total** | | **~230-280 lines** |

### Architecture sketch

```
                    ┌─────────────────────────────┐
                    │  Dedicated Hook Thread       │
                    │                              │
                    │  SetWindowsHookExW(          │
                    │    WH_KEYBOARD_LL,           │
                    │    hook_proc,                │
                    │    hInstance,                 │
                    │    0                          │
                    │  )                            │
                    │                              │
                    │  GetMessage() loop            │
                    │  (required for hook dispatch) │
                    └──────────┬──────────────────┘
                               │
                    hook_proc fires on keydown/keyup
                               │
                    tracks VK_LCONTROL + VK_LWIN state
                               │
                    detects "both held" → Pressed
                    detects "either released" → Released
                               │
                    ┌──────────▼──────────────────┐
                    │  mpsc::UnboundedSender       │
                    │  sends HookEvent::Pressed    │
                    │  or HookEvent::Released      │
                    └──────────┬──────────────────┘
                               │
                    ┌──────────▼──────────────────┐
                    │  Tokio task (spawned once)    │
                    │  recv from channel            │
                    │  calls handle_shortcut()      │
                    │  with AppHandle               │
                    └─────────────────────────────┘
```

### Win key Start menu suppression

The hook callback returns a non-zero value (instead of calling `CallNextHookEx`) when it detects the Ctrl+Win combo is active. This prevents the Win key keypress from reaching the shell. On key release, the hook similarly consumes the event to prevent Start menu activation.

**Known behavior:**
- Works reliably when Ctrl is pressed BEFORE Win key (Ctrl down → Win down → combo active)
- Works reliably when both are released while combo is active
- Edge case: pressing Win first, THEN Ctrl — may briefly flash Start menu on some Windows builds
- Mitigation: document "press Ctrl first" or add a small debounce window (~50ms) to detect rapid Ctrl+Win regardless of order

### Technology maturity

- **WH_KEYBOARD_LL**: Proven, stable Win32 API since Windows 2000. Used by AutoHotkey, Wispr Flow, Discord, OBS, and countless other apps.
- **`windows` crate**: Microsoft's official Rust bindings for Win32. v0.58 is mature.
- **The pattern itself**: Well-documented, widely deployed. Not experimental.

### Technical risks

| Risk | Severity | Likelihood | Detail |
|------|----------|------------|--------|
| Start menu flashes on certain key orderings | Medium | Medium | Ctrl-first ordering avoids this; debounce mitigates |
| Antivirus flags low-level keyboard hook | Medium | Low | Code-signed apps are rarely flagged; you already use `enigo` for key simulation which is equally suspicious to AV |
| Hook conflicts with other apps (Discord, OBS, AHK) | Low | Low | Hooks are chained; multiple hooks coexist fine unless one breaks the chain |
| Hook thread must have message pump | Low | None | Standard pattern — `GetMessage` loop on dedicated thread |
| Increased CPU from processing all keystrokes | Low | None | WH_KEYBOARD_LL is efficient; only fires on key events, not polling |

- Known approaches: **Yes** — WH_KEYBOARD_LL is the standard approach, used by Wispr Flow
- Technology maturity: **Proven** — Win32 API, stable for 25+ years
- **Technical verdict: Feasible**

## Resource Feasibility

**Do we have what we need?**

- **Skills**: Unsafe Rust + Win32 API — pattern already exists in codebase (`transcribe.rs` DXGI calls). The `windows` crate makes the API ergonomic.
- **Budget**: ~1-2 days of development. Half day for standalone prototype, half day for integration, half day for testing and edge cases.
- **Tools/infrastructure**: All present — `windows` crate, Tauri async runtime, existing thread patterns.
- **Resource verdict: Feasible**

## External Dependency Feasibility

**Are external factors reliable?**

- **Windows API stability**: WH_KEYBOARD_LL has been stable across every Windows version since 2000. Not going away.
- **`windows` crate**: Maintained by Microsoft. Auto-generated from Windows metadata. Highly reliable.
- **Win key OS behavior**: Microsoft occasionally changes Win key handling (e.g., Windows 11 changed some Win key combos). The hook approach is resilient to this because it intercepts BEFORE the shell processes the key.
- **External verdict: Feasible**

## Blockers

| Blocker | Severity | Mitigation |
|---------|----------|------------|
| Win key Start menu activation on release | Medium | Consume `WM_KEYUP` for VK_LWIN in hook; press Ctrl first; add 50ms debounce |
| Frontend rejects modifier-only combos | Low | Remove null guard in `HotkeyCapture.tsx` lines 50-53, add `ctrl+meta` as valid |
| No existing shutdown cleanup handler | Low | Add `UnhookWindowsHookEx` call in `on_window_event(Destroyed)` or Drop impl |
| Dual hotkey system (hook for modifier-only, plugin for standard combos) | Low | Either replace plugin entirely with hook (simplest) or keep both with a routing layer |

No high-severity blockers.

## De-risking Options

- **Standalone prototype first (recommended)**: Build a 50-line Rust binary that installs WH_KEYBOARD_LL, detects Ctrl+Win, prints to console, and suppresses Start menu. Test on your machine. ~2-4 hours. Validates the core mechanism before touching the Tauri app.
- **Keep plugin as fallback**: If the hook approach has issues on a user's machine, fall back to `tauri-plugin-global-shortcut` with `ctrl+meta+space`. Detection: if hook installation fails (SetWindowsHookExW returns null), auto-fallback.
- **Feature-gate the hook**: `[features] keyboard-hook = []` — can ship with or without it, compile-time toggle.

## Overall Verdict

**Go with conditions**

Conditions:
1. Build and validate the standalone prototype first (half day)
2. Accept that Ctrl-first key ordering is more reliable than Win-first (or add debounce)
3. Plan for fallback to standard `ctrl+meta+space` if hook fails on specific machines

The codebase is well-aligned for this. The `handle_shortcut()` abstraction means the hook only needs to produce Pressed/Released events — everything downstream (recording, pipeline, pill UI, tray state) works unchanged. The `windows` crate is already present. Thread + Arc patterns are proven. This is a contained addition, not an architectural overhaul.

## Implementation Context

<claude_context>
<if_go>
- approach: New `keyboard_hook.rs` module with dedicated thread running GetMessage loop + WH_KEYBOARD_LL hook. Hook callback tracks VK_LCONTROL + VK_LWIN state, sends HookEvent via tokio mpsc channel. Receiver task calls existing handle_shortcut(). Replace tauri-plugin-global-shortcut registration in setup() with hook init.
- start_with: Standalone prototype binary — 50 lines, install hook, print Ctrl+Win detection, suppress Start menu. Validate on target machine before integration.
- critical_path: (1) Hook correctly detects Ctrl+Win press/release, (2) Start menu is suppressed, (3) Channel delivers events to Tauri thread reliably, (4) handle_shortcut() receives correct Pressed/Released states
</if_go>
<risks>
- technical: Start menu may flash on Win-first key ordering; mitigate with 50ms debounce window
- external: AV software flagging (low risk with code signing); hook conflicts with Discord/OBS overlays (low risk, hooks chain properly)
- mitigation: Standalone prototype validates suppression; fallback to tauri-plugin-global-shortcut for standard combos; feature-gate for compile-time toggle
</risks>
<alternatives>
- if_blocked: Use ctrl+meta+space (works today, zero code changes) as permanent solution
- simpler_version: Keep tauri-plugin-global-shortcut for standard combos, add hook ONLY for modifier-only combos (dual system with auto-detection based on hotkey string format)
</alternatives>
</claude_context>

**Next Action:** Build standalone prototype to validate WH_KEYBOARD_LL + Ctrl+Win + Start menu suppression on target machine, then proceed to integration plan.

## Sources

- [RegisterHotKey function — Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerhotkey) — 2026-03-02
- [Wispr Flow Supported Hotkeys](https://docs.wisprflow.ai/articles/2612050838-supported-unsupported-keyboard-hotkey-shortcuts) — 2026-03-02
- [global-hotkey crate — docs.rs](https://docs.rs/global-hotkey/latest/global_hotkey/hotkey/struct.HotKey.html) — 2026-03-02
- [win-hotkeys crate (WH_KEYBOARD_LL Rust library)](https://github.com/iholston/win-hotkeys) — 2026-03-02
- [tauri-plugin-global-shortcut — Tauri v2](https://v2.tauri.app/plugin/global-shortcut/) — 2026-03-02
- Codebase analysis: `src-tauri/src/lib.rs`, `src-tauri/src/transcribe.rs`, `src-tauri/src/audio.rs`, `src-tauri/src/pill.rs`, `src-tauri/Cargo.toml`
