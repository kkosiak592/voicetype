---
phase: 16-rebind-and-coexistence
verified: 2026-03-03T15:00:00Z
status: passed
score: 10/10 must-haves verified
re_verification: false
---

# Phase 16: Rebind and Coexistence Verification Report

**Phase Goal:** Changing the hotkey in settings correctly switches between the hook backend and tauri-plugin-global-shortcut at runtime, with no double-firing, and surfaces hook installation failure as a visible status in settings
**Verified:** 2026-03-03
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `rebind_hotkey` routes modifier-only combos to hook and standard combos to GlobalShortcutExt | VERIFIED | lib.rs:540/556: `if is_modifier_only(&old)` tears down hook; `if is_modifier_only(&new_key)` installs hook or calls `app.global_shortcut().on_shortcut()` |
| 2 | `unregister_hotkey` and `register_hotkey` also route correctly based on `is_modifier_only` | VERIFIED | lib.rs:627 (`unregister_hotkey`) and lib.rs:650 (`register_hotkey`) both call `is_modifier_only(&key)` and branch to hook or plugin accordingly |
| 3 | Startup routing in `setup()` reads saved hotkey and activates the correct backend | VERIFIED | lib.rs:1584-1785: reads `read_saved_hotkey()`, calls `is_modifier_only(&hotkey)` at line 1717, routes to `keyboard_hook::install()` (modifier-only) or `tauri_plugin_global_shortcut::Builder` (standard) |
| 4 | If hook installation fails at startup for modifier-only hotkey, fallback `ctrl+shift+space` is registered via plugin and persisted to settings.json | VERIFIED | lib.rs:1728-1738: on `Err(e)` from `keyboard_hook::install()`, persists `"ctrl+shift+space"` to settings.json via `write_settings()`, sets `effective_hotkey` to fallback, which the plugin then registers at line 1773 |
| 5 | `HookAvailable` managed state tracks whether the hook is usable, queryable via `get_hook_status` IPC | VERIFIED | lib.rs:108: `pub struct HookAvailable(pub Arc<AtomicBool>)`. lib.rs:686-688: `get_hook_status` IPC reads it. lib.rs:1785: `app.manage(HookAvailable(hook_available))` registers it after startup routing. lib.rs:1527: registered in `invoke_handler`. |
| 6 | `rebind_hotkey` refuses to switch backends while pipeline is recording | VERIFIED | lib.rs:533-536: `pipeline.current() != pipeline::Phase::Idle` returns Err immediately. `PipelineState::current()` exists in pipeline.rs:47-53. |
| 7 | `rebind_hotkey` refuses modifier-only combos when `HookAvailable` is false and hook install attempt also fails | VERIFIED | lib.rs:558-582: checks `HookAvailable`, attempts install on demand; on `Err(e)` returns error string. On non-Windows returns error unconditionally. |
| 8 | Settings panel shows inline warning "Hook unavailable — using standard shortcut fallback" when hook is not available | VERIFIED | GeneralSection.tsx:83-87: `{!hookAvailable && (<p className="mt-1 text-xs text-amber-600 dark:text-amber-400">Hook unavailable — using standard shortcut fallback</p>)}` |
| 9 | Warning is hidden by default when hook is working or when using standard combos | VERIFIED | App.tsx:33: `useState(true)` — default is true, no warning shown unless IPC returns false |
| 10 | `hookAvailable` state is loaded once on settings mount via `invoke('get_hook_status')` | VERIFIED | App.tsx:91-97: `invoke<boolean>('get_hook_status')` inside `loadSettings()`, with silent catch defaulting to `true` |

**Score:** 10/10 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src-tauri/src/lib.rs` | `is_modifier_only` predicate, routed IPC commands, startup routing, `HookAvailable` state, `get_hook_status` IPC | VERIFIED | All present: `is_modifier_only` at line 115, `HookAvailable` at line 108, `get_hook_status` at line 686, all three IPC commands routed at lines 529/624/648, startup routing at lines 1707-1785 |
| `src-tauri/src/pipeline.rs` | `PipelineState::current()` read accessor | VERIFIED | `pub fn current(&self) -> Phase` at lines 47-53 |
| `src/App.tsx` | `hookAvailable` state loaded via `invoke('get_hook_status')` and passed as prop to `GeneralSection` | VERIFIED | Line 33: `useState(true)`, lines 91-97: IPC invoke, line 163: `hookAvailable={hookAvailable}` prop |
| `src/components/sections/GeneralSection.tsx` | Conditional inline warning below HotkeyCapture when hook is unavailable | VERIFIED | Line 14: `hookAvailable: boolean` in props interface, line 24: destructured, lines 83-87: conditional warning with amber styling and exact text |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `lib.rs` | `keyboard_hook.rs` | `keyboard_hook::install()` calls in routed `rebind_hotkey`, `register_hotkey`, and `setup()` | WIRED | lib.rs:563, 594, 660, 1719 — all inside `#[cfg(windows)]` and inside `is_modifier_only` branches only |
| `lib.rs` | `tauri-plugin-global-shortcut` | `GlobalShortcutExt::on_shortcut`/`unregister` in non-modifier-only path | WIRED | lib.rs:551, 607-611, 639, 673-677 — all inside the `else` branches of `is_modifier_only` checks |
| `lib.rs` | `pipeline.rs` | `PipelineState::current()` check before backend switch | WIRED | lib.rs:533-536: `pipeline.current() != pipeline::Phase::Idle` |
| `App.tsx` | `lib.rs` | `invoke('get_hook_status')` -> `get_hook_status` IPC command | WIRED | App.tsx:92: `invoke<boolean>('get_hook_status')`. IPC registered in `invoke_handler` at lib.rs:1527. |
| `App.tsx` | `GeneralSection.tsx` | `hookAvailable` prop | WIRED | App.tsx:163: `hookAvailable={hookAvailable}`. GeneralSection.tsx:14: interface includes it, line 24 destructures it. |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| INT-02 | 16-01-PLAN.md | `rebind_hotkey` routes modifier-only combos through hook and standard combos through tauri-plugin-global-shortcut | SATISFIED | All three hotkey IPC commands use `is_modifier_only` as the single routing predicate. Startup also routes correctly. |
| INT-03 | 16-01-PLAN.md, 16-02-PLAN.md | If WH_KEYBOARD_LL installation fails, app falls back to RegisterHotKey and surfaces failure in settings | SATISFIED | lib.rs startup fallback to `ctrl+shift+space` with settings.json persistence. GeneralSection.tsx amber warning when `!hookAvailable`. Note: INT-03 mentions "RegisterHotKey" but implementation uses `tauri-plugin-global-shortcut` for fallback — this is architecturally equivalent and consistent with the project's out-of-scope decision to keep the plugin as fallback. |

No orphaned requirements: REQUIREMENTS.md traceability table maps INT-02 and INT-03 to Phase 16, both are accounted for in the plan frontmatter and verified implemented.

### Anti-Patterns Found

None. Scans of `lib.rs`, `App.tsx`, and `GeneralSection.tsx` returned no TODOs, FIXMEs, placeholder returns, or stub implementations.

Notable: `keyboard_hook::` references appear only at lines 104 (struct type), 563, 594, 660, 1719 — all inside `is_modifier_only` branches or type definitions. No calls outside routing context.

### Human Verification Required

#### 1. Runtime backend switch — hook to standard

**Test:** With `ctrl+win` as the active hotkey (hook path), open Settings, capture `ctrl+shift+v`, save. Then press `ctrl+shift+v` outside the settings window.
**Expected:** Dictation triggers. Pressing `ctrl+win` does nothing. No double-fire.
**Why human:** Cannot verify at-runtime backend teardown/activation sequence programmatically.

#### 2. Runtime backend switch — standard to hook

**Test:** With `ctrl+shift+space` active, rebind to `ctrl+win`. Close settings. Hold `ctrl+win`.
**Expected:** Recording starts. `ctrl+shift+space` does nothing. No double-fire.
**Why human:** Requires running app with Windows hook installation; can't verify WH_KEYBOARD_LL lifecycle statically.

#### 3. Hook failure warning display

**Test:** On a system where hook installation fails (e.g., low-privilege process, or by forcing install failure), open settings.
**Expected:** Amber inline warning "Hook unavailable — using standard shortcut fallback" appears below the hotkey field. Active hotkey shows `ctrl+shift+space`.
**Why human:** Requires triggering an actual hook installation failure; can't simulate in static analysis.

#### 4. Mid-recording rebind rejection

**Test:** Start a recording (hold `ctrl+win`), then while holding, attempt to call `rebind_hotkey` via settings.
**Expected:** Error returned: "Recording in progress — wait for it to finish before changing hotkey". Recording continues uninterrupted.
**Why human:** Requires concurrent user action; pipeline state race can't be verified statically.

### Gaps Summary

No gaps. All 10 must-have truths are verified. Both requirements (INT-02, INT-03) are satisfied with substantive, wired implementations. Both compilers (Rust `cargo check`, TypeScript `tsc --noEmit`) pass clean. The four human verification items are runtime behavioral checks that require a running app, not deficiencies.

---

_Verified: 2026-03-03T15:00:00Z_
_Verifier: Claude (gsd-verifier)_
