# Project Research Summary

**Project:** VoiceType v1.2 — Ctrl+Win Modifier-Only Hotkey (Keyboard Hook Milestone)
**Domain:** Win32 WH_KEYBOARD_LL integration into existing Tauri 2.0 voice-to-text app
**Researched:** 2026-03-02
**Confidence:** HIGH

> **Note:** This summary covers only the v1.2 milestone research (modifier-only hotkey via WH_KEYBOARD_LL). The full project research (v1.0 core pipeline, audio, transcription, UI) is in git history as of 2026-02-27.

---

## Executive Summary

VoiceType v1.0/v1.1 ships with a working global hotkey system using `tauri-plugin-global-shortcut` (Win32 `RegisterHotKey`). The v1.2 goal is adding Ctrl+Win as a supported activation hotkey — a modifier-only combination that `RegisterHotKey` fundamentally cannot represent because it requires a non-zero virtual key code. The solution is well-understood: a custom `WH_KEYBOARD_LL` low-level keyboard hook running on a dedicated thread with a Win32 message loop. All candidate library alternatives (`win-hotkeys`, `rdev`, `windows-hotkeys`) are disqualified by concrete, verified technical reasons. The implementation requires two surgical changes to the existing project: three new feature flags on the `windows` v0.58 crate and one line in the Tauri builder (`device_event_filter(Always)`). No new Cargo dependencies, no version bumps.

The architecture is additive: a new `keyboard_hook.rs` module (~200 LOC) handles all hook logic and bridges to the existing `handle_shortcut()` function in `lib.rs` via an `mpsc` channel and a dispatcher thread. The existing audio pipeline, transcription, VAD, and UI are untouched. `tauri-plugin-global-shortcut` is kept as the active backend for standard hotkeys and as a fallback when WH_KEYBOARD_LL installation fails. The implementation is a hybrid that routes based on hotkey type — modifier-only combos go through the hook, everything else stays on `RegisterHotKey`.

All five critical pitfalls concentrate in Phase 1. The three highest-priority ones are: (1) WH_KEYBOARD_LL callbacks silently not firing when the Tauri window has focus — fixed with `DeviceEventFilter::Always` applied before `build()`, no other workaround works; (2) the hook being silently removed by Windows after exceeding the 1-second callback timeout with no error notification — prevented by a strictly non-blocking callback that only sets `AtomicBool` flags and calls `try_send`; and (3) the Start menu opening on Win key release despite the hook consuming the event — requires a VK 0xE8 mask-key injection technique that must also guard against infinite recursion via the `LLKHF_INJECTED` flag. A fourth risk is distribution-time: the combination of global keyboard hook + key injection matches the Defender ML classifier for credential-stealing malware (expanded in September 2024), requiring a VirusTotal pre-release check and code signing.

---

## Key Findings

### Recommended Stack

The existing stack requires only two additions. No version bumps, no new crates.

**Core technologies:**
- `windows` v0.58 + 3 new feature flags (`Win32_Foundation`, `Win32_UI_WindowsAndMessaging`, `Win32_UI_Input_KeyboardAndMouse`): provides `SetWindowsHookExW`, `GetMessageW`, `PostThreadMessageW`, `UnhookWindowsHookEx`, `KBDLLHOOKSTRUCT`, `VK_LWIN`, `VK_RWIN`, `VK_LCONTROL`, `VK_RCONTROL`, `LRESULT` — additive, no API breaks, confirmed stable across windows crate 0.48–0.62
- `tauri::DeviceEventFilter::Always` (one builder call): resolves confirmed Tauri 2.0 defect #13919 — without it, WH_KEYBOARD_LL callbacks are silently swallowed when the Tauri window has focus; this must be applied before `build()` and before hook installation
- `tauri-plugin-global-shortcut` (kept as-is): standard hotkey fallback; mutually exclusive with hook path at runtime; never both active for the same key combo
- `std::sync::mpsc` + `std::sync::OnceLock` (std only, no new crates): channel-based non-blocking hook-to-dispatcher bridge mandated by the 1-second Win32 callback timeout constraint

**What not to use — all evaluated and disqualified:**
- `win-hotkeys` crate: `windows = "0.60"` version conflict with project's `windows = "0.58"` pin; API requires a trigger key plus modifiers with no modifier-only variant
- `rdev` crate: confirmed open bug — keyboard events dropped when Tauri window is focused (tauri-apps/tauri #14770, unresolved March 2026)
- `windows-hotkeys` (dnlmlr): uses `RegisterHotKey` internally; same modifier-only limitation as the existing plugin
- `VK_07` for Start menu mask injection: opens Xbox Game Bar on Windows 10 1909+ — use `VK_E8` (documented Microsoft "unassigned") instead

### Expected Features

The v1.2 feature set is non-divisible. All six P1 features depend on the WH_KEYBOARD_LL hook module and must ship together. There is no viable MVP subset because Start menu suppression and press-order independence are prerequisites for usability, not polish.

**Must have (P1 — all required for v1.2):**
- WH_KEYBOARD_LL hook module — foundational dependency for all other features in this milestone
- Ctrl+Win combo detection with 50ms debounce — press-order independence is required; without it users get inconsistent activation on physical keyboards (5–30ms natural press stagger)
- Start menu suppression via Win keyup consumption — without this the feature is unusable (Start menu opens on every dictation)
- Hold-to-talk behavior on modifier-only combo — Wispr Flow (the direct Windows competitor) uses Ctrl+Win hold-to-talk as its default; this is the expected UX model
- Frontend hotkey capture dialog accepts modifier-only combos — users must be able to configure Ctrl+Win in settings; the existing capture dialog cannot represent it
- Fallback to RegisterHotKey if WH_KEYBOARD_LL installation fails — prevents complete hotkey breakage; must surface failure state to user in settings

**Should have (P2 — add after v1.2 validation):**
- Hook persistence across Win+L lock/unlock — add if users report hotkey stops working after screen lock (requires `WTSRegisterSessionNotification`)
- Left vs. right modifier distinction in binding — low complexity once hook is in place; `VK_LCONTROL` vs. `VK_RCONTROL` already disambiguated by `KBDLLHOOKSTRUCT.vkCode`

**Defer (v2+ only):**
- Double-tap modifier combo for toggle mode entry — Wispr Flow has it but complexity is high and demand is unclear
- Additional modifier-only combos (double-Ctrl, Shift+Win) — out of scope for this milestone

**Anti-features to avoid:**
- Suppressing all Win key combos (Win+L, Win+D, Win+Tab) — only suppress when Ctrl+Win combo is active; all other Win key usage must pass through unmodified
- Registry-based Win key disable — requires elevated privileges, survives app crash, breaks Win key globally until manually restored
- Debounce window under 50ms — below that, press-order sensitivity causes inconsistent activation for normal typing speed

### Architecture Approach

Three-thread design: the existing Tauri main thread (unchanged), a new `keyboard-hook` thread that owns the WH_KEYBOARD_LL installation and Win32 `GetMessage` / `DispatchMessage` message pump, and a new `hook-dispatcher` thread that reads from the `mpsc` channel and calls the existing `handle_shortcut_pressed/released()` functions in `lib.rs`. The hook thread cannot block, cannot lock mutexes, and cannot call async functions — it only updates `AtomicBool` state flags and calls `try_send` with primitive values. All business logic stays in the existing `lib.rs`.

**Major components:**
1. `keyboard_hook.rs` (NEW, ~200 LOC) — `SetWindowsHookExW` installation, Win32 `GetMessage` loop, modifier state machine with `AtomicBool` flags (`CTRL_DOWN`, `WIN_DOWN`, `COMBO_ACTIVE`), 50ms debounce via `KBDLLHOOKSTRUCT.time`, Win keyup suppression via `LRESULT(1)`, VK_E8 mask key injection, `mpsc::Sender::try_send`, clean shutdown via `PostThreadMessageW(WM_QUIT)`
2. `lib.rs` (MODIFIED) — `mod keyboard_hook`; conditional hook startup in `setup()` based on saved hotkey format; routing in `rebind_hotkey`, `register_hotkey`, `unregister_hotkey` commands; teardown call to `keyboard_hook::stop()`
3. Frontend hotkey capture UI (MODIFIED) — must accept and display modifier-only combos (`"ctrl+win"` with no letter key suffix) as a valid distinct hotkey string format
4. `OnceLock<AppHandle<Wry>>` (NEW static in `keyboard_hook.rs`) — sole bridge between the fixed-signature `extern "system"` callback and the Tauri runtime; `AppHandle` is `Send` in Tauri 2.x (historical issue #2343 was Tauri 1.x, resolved)

**Key invariants:**
- Hook callback execution time must stay under ~5ms (Windows enforces 1-second max; 11 cumulative timeouts = silent permanent hook removal with no notification)
- Only one hotkey backend active at a time for a given key combo; the `rebind_hotkey` command manages the switch
- `UnhookWindowsHookEx` must be called from the hook thread itself (before it exits), triggered by `PostThreadMessageW(WM_QUIT)` breaking the `GetMessage` loop

### Critical Pitfalls

1. **WH_KEYBOARD_LL callbacks silently never fire when Tauri window is focused** — apply `.device_event_filter(tauri::DeviceEventFilter::Always)` to the Tauri builder before `.build()`; install the hook on a dedicated `std::thread` (not a Tokio task, not the Tauri main thread); verify by opening settings window, giving it focus, pressing Ctrl+Win — must fire (Tauri issue #13919, confirmed closed July 2025)

2. **Start menu opens despite hook suppression, especially on Windows 11** — suppress both Win KEYDOWN and Win KEYUP (return `LRESULT(1)` for both); inject VK 0xE8 via `keybd_event` before returning from KEYDOWN to mask the Win key in the OS's internal state machine; check `LLKHF_INJECTED` flag at the top of the callback to skip re-processing synthetic events and prevent infinite recursion

3. **Hook silently removed by Windows after exceeding 1-second callback timeout** — the callback must only update `AtomicBool` flags and call `try_send` (non-blocking); all debounce evaluation, state machine logic, and `AppHandle` calls go in the dispatcher thread; never lock a `Mutex`, never allocate, never call async code inside the callback

4. **Rust panic crossing `extern "system"` FFI boundary causes undefined behavior** — wrap the entire callback body in `std::panic::catch_unwind`; alternatively, keep the callback so minimal (one atomic store, one channel send, one return) that no panic is possible; zero `.unwrap()` or index operations in the callback

5. **RegisterHotKey + WH_KEYBOARD_LL double-firing or deadlocking during coexistence** — scope the WH_KEYBOARD_LL callback to only process `VK_LWIN`/`VK_RWIN` events; unregister the overlapping standard hotkey from `tauri-plugin-global-shortcut` when the hook is active; never acquire a `Mutex` inside the hook callback (only `AtomicBool` via `Ordering::Relaxed`)

---

## Implications for Roadmap

The dependency tree for v1.2 is unusually flat: WH_KEYBOARD_LL is the single prerequisite for everything. The phase structure follows directly from this constraint and from the requirement that all five critical pitfalls must be embedded in Phase 1 architecture (they cannot be retrofitted).

### Phase 1: WH_KEYBOARD_LL Hook Module

**Rationale:** Every v1.2 feature depends on a working hook. The critical architectural decisions — non-blocking callback, dedicated thread, Win32 message loop, `AtomicBool` state machine, `LLKHF_INJECTED` guard — cannot be retrofitted after the fact. They must be correct from the first commit. A strict incremental build order within this phase is required to verify each sub-component before the next depends on it.

**Delivers:** A working, tested `keyboard_hook.rs` that detects Ctrl+Win with 50ms debounce and drives hold-to-talk recording with proper Start menu suppression on both Windows 10 and Windows 11, and a clean shutdown path that does not leave a dangling hook.

**Addresses (from FEATURES.md):** WH_KEYBOARD_LL hook module (P1), Ctrl+Win combo detection with 50ms debounce (P1), Start menu suppression (P1), hold-to-talk on modifier-only combo (P1)

**Avoids (from PITFALLS.md):** All five critical pitfalls — hook dead when Tauri focused (#1), Start menu not suppressed (#2), hook silently removed on timeout (#3), panic across FFI (#4), double-firing with RegisterHotKey (#5); also moderate pitfall recursive re-entry from synthetic key injection (#6)

**Build order within phase (must be sequential):**
1. `keyboard_hook.rs` skeleton — `HookEvent` enum, `OnceLock` statics, stub `start()`/`stop()` — verify compilation
2. `DeviceEventFilter::Always` added to Tauri builder — verify before hook is installed
3. Modifier state machine unit tests — `CTRL_DOWN`/`WIN_DOWN`/`COMBO_ACTIVE` logic with mock timestamps, no Win32 calls
4. WH_KEYBOARD_LL installation + Win32 `GetMessage` loop + logging only — verify callback fires for every keystroke
5. `mpsc` channel wiring — `try_send` in hook proc → dispatcher prints events — verify Pressed/Released arrive correctly for Ctrl+Win
6. Connect to `handle_shortcut_pressed/released` — replace print with actual calls — verify hold-to-talk end-to-end
7. Start menu suppression — `COMBO_ACTIVE` tracking + `LRESULT(1)` on Win keyup + VK_E8 mask injection — test on Windows 10 AND Windows 11; verify Win key alone still works
8. Shutdown — `PostThreadMessageW(WM_QUIT)` + `UnhookWindowsHookEx` in hook thread — verify clean exit with no dangling hook

**Research flag:** The Windows 11 Start menu suppression timing (whether VK_E8 injection on KEYDOWN alone is sufficient, or also required on KEYUP) needs empirical validation during implementation — the AutoHotkey community documents a Windows 11 behavioral change but does not specify exact requirements. Standard pattern otherwise.

### Phase 2: Rebind and Coexistence Logic

**Rationale:** Once the hook works in isolation, wire it into the hotkey rebind flow and define the state ownership boundary with `tauri-plugin-global-shortcut`. This is where the double-firing and deadlock risks concentrate outside of the callback itself.

**Delivers:** The `rebind_hotkey` Tauri IPC command correctly routes between the hook path and the plugin path based on hotkey type; runtime hotkey switching works cleanly; fallback to `RegisterHotKey` on WH_KEYBOARD_LL installation failure is detected and surfaced to the user in settings.

**Addresses:** Fallback to RegisterHotKey on hook failure (P1), rebind flow as specified in ARCHITECTURE.md integration points

**Avoids:** Pitfall 5 (double-firing from both backends observing the same key), pitfall from leaving both backends registered simultaneously

**Implements (from ARCHITECTURE.md):** Routing logic in `lib.rs` `rebind_hotkey`, `register_hotkey`, `unregister_hotkey` commands; `keyboard_hook::rebind(combo: HookCombo)` for atomic modifier set updates

**Research flag:** Standard pattern — complete specification is in ARCHITECTURE.md. No additional research needed.

### Phase 3: Frontend Modifier-Only Capture UI

**Rationale:** The frontend settings change has no dependency on Phase 2 internals and can be developed in parallel with Phase 2. Isolated as a separate phase for scope clarity; can be started after Phase 1 is stable.

**Delivers:** Hotkey capture dialog accepts `"ctrl+win"` (no letter key required) as a valid input; stores modifier-only combos in a distinct format from standard hotkey strings; displays them as "Ctrl + Win" in the settings panel.

**Addresses:** Frontend hotkey capture dialog for modifier-only combos (P1 feature)

**Avoids:** UX pitfall documented in PITFALLS.md — the existing capture mode expects a trigger key and cannot represent modifier-only combos; must be a new capture mode, not a workaround in the existing flow

**Research flag:** Standard pattern — React state management; hotkey format is already defined (`"ctrl+win"` string). No additional research needed.

### Phase 4: Integration Testing and Distribution Verification

**Rationale:** Several critical pitfalls only manifest under specific runtime conditions (Tauri window focused, Windows 11 vs. Windows 10, antivirus active, CPU load) that unit tests cannot cover. Dedicated integration testing pass is required before any distribution.

**Delivers:** Verified behavior across the full "looks done but isn't" checklist from PITFALLS.md; VirusTotal scan of the signed v1.2 binary before any distribution.

**Addresses:** All five critical pitfalls have explicit test cases here; Pitfall 10 (Defender false positive escalation for WH_KEYBOARD_LL + SendInput combination)

**Key verification tests (from PITFALLS.md checklist):**
- Settings window focused → press Ctrl+Win → must fire (validates DeviceEventFilter fix)
- Windows 11 machine: Ctrl+Win must not open Start menu; Win key alone must still work after fix (validates mask-key technique)
- Alt+Tab away while holding Ctrl → release → verify no phantom trigger on return (validates modifier state desync recovery)
- Press Win before Ctrl (reversed order) → verify hotkey fires (validates debounce)
- Close VoiceType while hook is active → verify no lingering hook (validates shutdown path)
- Trigger Ctrl+Win 20 times rapidly → verify exactly 20 recording sessions, not 40 (validates coexistence with RegisterHotKey)
- VirusTotal scan of v1.2 signed binary vs. v1.1 baseline — any new detections are a blocking issue

**Research flag:** Distribution Defender behavior without OV/EV code signing cannot be researched in advance — requires testing the actual binary. Treat any VirusTotal detection escalation vs. v1.1 as a blocking issue.

### Phase Ordering Rationale

- Phase 1 must be first and fully verified before anything else starts: all features in this milestone depend on a working hook; the five critical architectural decisions cannot be retrofitted
- Phase 2 before Phase 3: the `rebind_hotkey` routing logic determines the hotkey string format the frontend must produce; if backend format changes after Phase 3 is complete, the frontend must change again
- Phase 3 is parallelizable with Phase 2: frontend capture UI has no dependency on the Rust backend routing logic; can be developed in parallel after Phase 1 is stable
- Phase 4 last: integration testing requires all components to be complete; VirusTotal check requires the final binary

### Research Flags

Phases needing deeper investigation during planning or implementation:
- **Phase 1 (Windows 11 Start menu suppression):** AutoHotkey community confirms Windows 11 changed Win key handling; exact VK_E8 injection timing requirements for Windows 11 need empirical verification against an actual Windows 11 machine during implementation — STACK.md documents the technique but notes community uncertainty on Windows 11 specifics
- **Phase 4 (Defender false positive rate):** Defender ML sensitivity for WH_KEYBOARD_LL + SendInput was expanded September 2024 (documented in PITFALLS.md Pitfall 10); the exact impact on unsigned vs. signed v1.2 binaries cannot be determined without testing the actual built binary

Phases with standard patterns where no additional research is needed:
- **Phase 2 (rebind routing):** ARCHITECTURE.md provides complete specification including code flow diagrams and component responsibility table
- **Phase 3 (frontend capture UI):** Standard React state management; hotkey format fully defined in ARCHITECTURE.md (`"ctrl+win"` string, no virtual key suffix)

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | windows crate feature flags verified against win-hotkeys 0.5.1 Cargo.toml source; `DeviceEventFilter::Always` fix confirmed in closed Tauri issue #13919 (July 2025); all Win32 APIs documented on MSDN; library disqualifications verified against official APIs and confirmed open issues |
| Features | HIGH | Wispr Flow official docs confirm Ctrl+Win hold-to-talk as the reference Windows UX; Windows Speech Recognition official docs confirm the same; feature dependencies mapped precisely from API constraints; P1/P2/v2+ prioritization grounded in implementation complexity and user impact |
| Architecture | HIGH | Win32 API threading requirements are authoritative MSDN-documented; `OnceLock<AppHandle>` pattern verified against Tauri 2.x community discussions (Tauri Discussion #6309); Tauri 2.x `AppHandle` Send+Sync confirmed resolved (issue #2343 was Tauri 1.x); three-thread design directly implements Win32 specs |
| Pitfalls | HIGH | Critical pitfalls sourced from official MSDN (`LowLevelKeyboardProc` timeout, `GetAsyncKeyState` restriction), confirmed Tauri GitHub issues (#13919, #14770), Rust Nomicon and RFC 2945 (FFI panic UB); moderate pitfalls sourced from AutoHotkey community documentation (multi-source, consistent) |

**Overall confidence:** HIGH

### Gaps to Address

- **Windows 11 Start menu suppression exact technique:** AutoHotkey community documents that Windows 11 changed Win key handling and the VK_E8 masking technique may require injection on both KEYDOWN and KEYUP transitions (vs. KEYDOWN only on Windows 10). This is empirical — must be validated against an actual Windows 11 machine during Phase 1 implementation. There is no documentation that resolves this definitively.

- **Hook health-check mechanism:** PITFALLS.md recommends a periodic health-check timer that detects silent hook removal (after 11 cumulative timeouts) and reinstalls the hook. The specific implementation is not specified in ARCHITECTURE.md. For v1.2 MVP, acceptable compensating control is a visible "hook inactive" status indicator in the system tray tooltip, with health-check deferred to a v1.2.x patch if silent removal is reported in practice.

- **Defender false positive on unsigned v1.2 binary:** The WH_KEYBOARD_LL + SendInput combination is a confirmed expanded Defender ML target as of September 2024. The exact detection threshold on the v1.2 binary — and whether it differs between signed and unsigned — cannot be determined without testing the actual built binary via VirusTotal. This is a Phase 4 gate, not a planning gap.

---

## Sources

### Primary (HIGH confidence)

- [LowLevelKeyboardProc — Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelkeyboardproc) — callback timeout (300ms default, 1s max on Win10 1709+), message loop requirement, return value semantics, `GetAsyncKeyState` restriction inside callback
- [SetWindowsHookExW — Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexw) — API signature, `dwThreadId=0` for global scope, thread message loop requirement
- [KBDLLHOOKSTRUCT — Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-kbdllhookstruct) — `vkCode`, `flags` (`LLKHF_INJECTED` = bit 4), `time` (millisecond timestamp)
- [RegisterHotKey — Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerhotkey) — confirms `vk` is mandatory and non-zero; modifier-only combos impossible
- [Disabling Shortcut Keys in Games — Microsoft Win32 Docs](https://learn.microsoft.com/en-us/windows/win32/dxtecharts/disabling-shortcut-keys-in-games) — canonical `SetWindowsHookExW` pattern for `VK_LWIN`/`VK_RWIN` suppression
- [Virtual-Key Codes — Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes) — `VK_LCONTROL` (0xA2), `VK_RCONTROL` (0xA3), `VK_LWIN` (0x5B), `VK_RWIN` (0x5C), `VK_E8` (unassigned)
- [Tauri issue #13919](https://github.com/tauri-apps/tauri/issues/13919) — WH_KEYBOARD_LL fails to capture system keys when Tauri window focused; `DeviceEventFilter::Always` fix; confirmed closed July 2025
- [Tauri issue #14770](https://github.com/tauri-apps/tauri/issues/14770) — rdev keyboard events drop when Tauri window focused; same root cause as #13919
- [win-hotkeys source — iholston/win-hotkeys](https://github.com/iholston/win-hotkeys) — confirms `windows = "0.60"` dependency and exact feature flag names (`Win32_Foundation`, `Win32_UI_WindowsAndMessaging`, `Win32_UI_Input_KeyboardAndMouse`); API requires trigger key, no modifier-only support
- [Rust Nomicon: FFI and panics](https://doc.rust-lang.org/nomicon/ffi.html) — Rust panic across `extern "C"` / `extern "system"` boundary is undefined behavior
- [RFC 2945: C-unwind ABI](https://rust-lang.github.io/rfcs/2945-c-unwind-abi.html) — specification for Rust panic/FFI boundary semantics
- [Microsoft Keylogging Malware Protection — Windows IT Pro Blog, Sep 2024](https://techcommunity.microsoft.com/blog/windows-itpro-blog/keylogging-malware-protection-built-into-windows/4256289) — Defender ML sensitivity expansion for WH_KEYBOARD_LL + SendInput combinations
- [Wispr Flow — Starting Your First Dictation](https://docs.wisprflow.ai/articles/6409258247-starting-your-first-dictation) — Ctrl+Win as default PC hold-to-talk hotkey; double-tap hands-free mode
- [Microsoft Support — Windows Speech Recognition Commands](https://support.microsoft.com/en-us/windows/windows-speech-recognition-commands-9d25ef36-994d-f367-a81a-a326160128c7) — Ctrl+Win as WSR toggle activation

### Secondary (MEDIUM confidence)

- [AutoHotkey MenuMaskKey docs](https://autohotkey.com/docs/commands/_MenuMaskKey.htm) — VK_E8 as unassigned mask key for Start menu suppression; VK_07 now reserved for Xbox Game Bar
- [AutoHotkey community — Win key suppression](https://www.autohotkey.com/boards/viewtopic.php?t=101812) — Ctrl+Win naturally does not trigger Start menu on most Windows versions; mask-key technique details
- [AutoHotkey — Disable left Win key on Windows 11](https://www.autohotkey.com/boards/viewtopic.php?t=96593) — Windows 11 changed Win key behavior; mask-key requirements may differ
- [Tauri Discussion #6309](https://github.com/orgs/tauri-apps/discussions/6309) — `OnceLock` pattern for `AppHandle` in `extern "system"` callbacks
- [Tauri Discussion #8538](https://github.com/tauri-apps/tauri/discussions/8538) — `AppHandle` state access across threads in Tauri 2.x
- [How do I access Windows Low Level Hooks using the Windows rust crate — Microsoft Q&A](https://learn.microsoft.com/en-us/answers/questions/1530452/how-do-i-access-the-windows-low-level-hooks-using) — community-confirmed feature flag set for keyboard hooks in windows-rs
- [Wispr Flow — Supported/Unsupported Hotkeys](https://docs.wisprflow.ai/articles/2612050838-supported-unsupported-keyboard-hotkey-shortcuts) — confirms Ctrl+Win is viable on Windows; Wispr Flow recommends it as primary

---
*Research completed: 2026-03-02*
*Ready for roadmap: yes*
