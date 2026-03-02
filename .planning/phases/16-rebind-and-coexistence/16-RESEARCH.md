# Phase 16: Rebind and Coexistence - Research

**Researched:** 2026-03-02
**Domain:** Runtime hotkey backend switching — tauri-plugin-global-shortcut vs WH_KEYBOARD_LL hook, with hook-failure UX
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

All areas deferred to Claude's best judgment based on standard practices.

**Switchover behavior:**
- Atomic swap: unregister/stop the old backend before starting the new one — never have both active simultaneously
- If a recording session is in progress when the user changes hotkey, finish the current session before switching backends (don't interrupt mid-recording)
- Brief gap (~10ms) between old teardown and new registration is acceptable — user is in settings UI, not actively dictating
- **Route based on string parsing of the existing hotkey format — no new field needed**
- A modifier-only combo contains only modifier tokens (ctrl, alt, shift, meta/win) with no base key
- "ctrl+win" → modifier-only → route to hook backend
- "ctrl+shift+v" → has base key → route to tauri-plugin-global-shortcut

**Hook failure UX:**
- Inline warning below the hotkey field in settings: "Hook unavailable — using standard shortcut fallback"
- Auto-assign a fallback standard hotkey (Ctrl+Shift+Space, the v1.1 default) so the user isn't left with no working hotkey
- Log the failure reason for debugging but don't expose technical details in the UI
- If the user later selects a modifier-only combo and the hook is unavailable, show an error explaining that modifier-only combos require the hook and suggest a standard combo instead

**Backend indicator:**
- Hidden by default
- Only surface backend info in the hook-failure warning state
- No "hook active" / "standard shortcut active" indicator in normal operation

**Store format:**
- Store in settings.json as the same plain string field ("hotkey"), same as today

### Claude's Discretion

(None specified beyond the locked decisions above — all areas covered explicitly.)

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| INT-02 | `rebind_hotkey` routes modifier-only combos through hook and standard combos through `tauri-plugin-global-shortcut` | Covered by: string-parsing routing logic, atomic swap pattern, `GlobalShortcutExt::unregister` + `on_shortcut` runtime API, hook module start/stop API from Phase 15 |
| INT-03 | If WH_KEYBOARD_LL installation fails, app falls back to `RegisterHotKey` and surfaces failure in settings | Covered by: hook failure detection pattern, `Arc<AtomicBool>` hook-status managed state, Tauri IPC `get_hook_status` command pattern, frontend inline warning using existing `GeneralSection` structure |
</phase_requirements>

---

## Summary

Phase 16 wires two existing subsystems together: the `tauri-plugin-global-shortcut` plugin (already shipping in v1.1) and the `keyboard_hook` module delivered by Phase 15. The core logic is a routing function that inspects the hotkey string at two moments — startup and rebind — and directs each to the correct backend. Neither backend has a "coexistence" mode; exactly one is active at any time.

The `tauri-plugin-global-shortcut` runtime API (`GlobalShortcutExt`) is confirmed stable and already used by `rebind_hotkey()`, `register_hotkey()`, and `unregister_hotkey()` in `lib.rs`. The only new Rust surface this phase adds is: (1) a `is_modifier_only(hotkey: &str) -> bool` helper used at every routing decision point; (2) hook start/stop calls wrapping Phase 15's `keyboard_hook` module API; (3) a `HookStatus` managed state (`Arc<AtomicBool>`) populated at startup and queryable via a new IPC command; and (4) a small frontend addition to `GeneralSection.tsx` that conditionally renders the inline warning.

The "finish current session before switching" rule integrates cleanly with the existing `PipelineState` AtomicU8 CAS machine: `rebind_hotkey` spins on `pipeline::IDLE` before proceeding, or more simply returns early if not idle (user must wait for recording to finish, which is fast). The frontend already handles rebind failures via the existing `setError` path in `HotkeyCapture.tsx`.

**Primary recommendation:** Implement `is_modifier_only()` as the single routing predicate used in both `setup()` and `rebind_hotkey()`. Route to hook on true, to `tauri-plugin-global-shortcut` on false. Never call both. Expose hook availability as a boolean IPC command queried once on settings panel mount.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tauri-plugin-global-shortcut` | 2.x (already in Cargo.toml) | Standard hotkey backend for non-modifier-only combos | Already shipping; GlobalShortcutExt runtime API confirmed |
| `keyboard_hook` module | Phase 15 output | Hook backend for modifier-only combos | The Phase 15 deliverable; Phase 16 calls its public API |
| `std::sync::atomic::AtomicBool` | stdlib | Hook-available status flag (cross-thread, zero-allocation) | Same pattern as `LevelStreamActive` already in codebase |
| `tauri::Manager` / `app.state::<T>()` | Tauri 2.x | Managed state access in IPC commands | All existing IPC commands use this pattern |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `serde_json` | 1.x (already in Cargo.toml) | Settings persistence for fallback hotkey | Already used by `write_settings()` |
| React `useState` / `useEffect` | React 18 (already in project) | Frontend hook-status polling on mount | Existing pattern in `App.tsx` for `get_engine` reconciliation |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `Arc<AtomicBool>` for hook status | `Mutex<HookState>` enum | AtomicBool is sufficient for a binary available/unavailable signal; Mutex adds overhead for no benefit here |
| Polling IPC on mount | Tauri event emit from Rust on startup | Events require listener setup before they fire; polling on mount is simpler and the status is stable after startup |
| Returning error from `rebind_hotkey` when busy | Spinning/waiting for pipeline idle | Return error immediately — frontend shows it, user waits a second and retries; avoids blocking the command thread |

**Installation:** No new dependencies. Phase 15 adds `Win32_Foundation`, `Win32_UI_WindowsAndMessaging`, `Win32_UI_Input_KeyboardAndMouse` feature flags to the existing `windows = "0.58"` dependency. Phase 16 consumes Phase 15's module — no additional Cargo changes.

---

## Architecture Patterns

### Recommended Project Structure

```
src-tauri/src/
├── lib.rs                  # Modified: routing logic, new IPC commands, managed state
├── keyboard_hook.rs        # Phase 15 deliverable — Phase 16 calls start/stop/is_available
└── (all other modules unchanged)

src/components/sections/
└── GeneralSection.tsx      # Modified: hook-status warning display below HotkeyCapture
```

### Pattern 1: `is_modifier_only` routing predicate

**What:** A pure function that returns `true` if all tokens in a `+`-separated hotkey string are modifier names and no base key is present.
**When to use:** Called in `setup()` at startup and in `rebind_hotkey()` on every rebind. The single decision point — no other code should contain routing logic.

```rust
// Source: derived from CONTEXT.md decision on hotkey format routing
fn is_modifier_only(hotkey: &str) -> bool {
    const MODIFIERS: &[&str] = &["ctrl", "alt", "shift", "meta", "win", "super"];
    !hotkey.is_empty()
        && hotkey
            .split('+')
            .all(|token| MODIFIERS.contains(&token.to_lowercase().as_str()))
}

#[cfg(test)]
mod tests {
    use super::is_modifier_only;

    #[test]
    fn modifier_only_combos() {
        assert!(is_modifier_only("ctrl+win"));
        assert!(is_modifier_only("ctrl+meta"));
        assert!(is_modifier_only("alt+shift"));
    }

    #[test]
    fn standard_combos() {
        assert!(!is_modifier_only("ctrl+shift+v"));
        assert!(!is_modifier_only("ctrl+shift+space"));
        assert!(!is_modifier_only("alt+f4"));
    }

    #[test]
    fn edge_cases() {
        assert!(!is_modifier_only(""));
        assert!(!is_modifier_only("ctrl+"));   // trailing + yields empty token
    }
}
```

### Pattern 2: Atomic swap in `rebind_hotkey`

**What:** Tear down the old backend completely before starting the new one. The two backends are mutually exclusive.
**When to use:** Every path through `rebind_hotkey` — including same-type-to-same-type rebind (e.g., one standard hotkey to another).

```rust
// Source: Context7 confirmed GlobalShortcutExt API + Phase 15 keyboard_hook API shape
#[tauri::command]
fn rebind_hotkey(app: tauri::AppHandle, old: String, new_key: String) -> Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    // Guard: refuse to switch backends mid-recording
    let pipeline = app.state::<pipeline::PipelineState>();
    if pipeline.current() != pipeline::IDLE {
        return Err("Recording in progress — wait for it to finish before changing hotkey".to_string());
    }

    // Tear down old backend
    if !old.is_empty() {
        if is_modifier_only(&old) {
            keyboard_hook::stop(&app);
        } else {
            app.global_shortcut().unregister(old.as_str()).map_err(|e| e.to_string())?;
        }
    }

    // Start new backend
    if is_modifier_only(&new_key) {
        let hook_status = app.state::<HookAvailable>();
        if !hook_status.0.load(std::sync::atomic::Ordering::Relaxed) {
            // Hook unavailable — refuse modifier-only selection
            return Err("Modifier-only combos require the keyboard hook, which is unavailable on this system. Choose a standard combo like Ctrl+Shift+V.".to_string());
        }
        keyboard_hook::start(&app).map_err(|e| e.to_string())?;
    } else {
        app.global_shortcut()
            .on_shortcut(new_key.as_str(), |app, _shortcut, event| {
                handle_shortcut(app, &event);
            })
            .map_err(|e| e.to_string())?;
    }

    // Persist
    let mut json = read_settings(&app)?;
    json["hotkey"] = serde_json::Value::String(new_key);
    write_settings(&app, &json)?;

    Ok(())
}
```

### Pattern 3: Startup routing in `setup()`

**What:** At startup, after reading the saved hotkey, route to the correct backend. If hook installation fails, record the failure, assign the fallback hotkey, and persist it.

```rust
// Source: existing setup() structure in lib.rs:1429
let hotkey = read_saved_hotkey(app).unwrap_or_else(|| "ctrl+win".to_owned());
let hook_available = Arc::new(AtomicBool::new(false));

if is_modifier_only(&hotkey) {
    match keyboard_hook::install(app) {
        Ok(()) => {
            hook_available.store(true, Ordering::Relaxed);
            log::info!("Hook backend active for hotkey: {}", hotkey);
        }
        Err(e) => {
            log::warn!("WH_KEYBOARD_LL install failed: {} — falling back to Ctrl+Shift+Space", e);
            // hook_available stays false
            let fallback = "ctrl+shift+space".to_owned();
            // Register fallback via plugin
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_shortcuts([fallback.as_str()])?
                    .with_handler(|app, _shortcut, event| handle_shortcut(app, &event))
                    .build(),
            )?;
            // Persist fallback so next startup also uses it
            let mut json = read_settings_app(app)?;
            json["hotkey"] = serde_json::Value::String(fallback);
            write_settings_app(app, &json)?;
        }
    }
} else {
    // Standard combo — use plugin as today
    app.handle().plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_shortcuts([hotkey.as_str()])?
            .with_handler(|app, _shortcut, event| handle_shortcut(app, &event))
            .build(),
    )?;
    // Hook not needed for standard combos; availability still queryable
    // (try-install to populate the flag, then immediately stop)
    match keyboard_hook::install(app) {
        Ok(()) => {
            hook_available.store(true, Ordering::Relaxed);
            keyboard_hook::stop(app);
        }
        Err(_) => { /* hook_available stays false; doesn't matter for standard combos */ }
    }
}

app.manage(HookAvailable(hook_available));
```

> **Note on startup hook-availability probing:** The simplest and most correct approach is to only install the hook at startup when the saved hotkey is modifier-only. Do not probe availability for standard combos at startup — this avoids unnecessary hook churn. Instead, populate `HookAvailable` based solely on the startup install result. If the user tries to switch to a modifier-only combo and the hook was never tested, attempt install at that point. This is the recommended approach (see Pitfall 3 below).

### Pattern 4: Hook status IPC — query-on-mount

**What:** A simple `get_hook_status` IPC command returns the boolean availability. Frontend calls it once when the settings panel mounts.

```rust
/// Returns whether WH_KEYBOARD_LL hook installation succeeded at startup.
/// Used by the frontend to display the inline fallback warning.
#[tauri::command]
fn get_hook_status(app: tauri::AppHandle) -> bool {
    app.state::<HookAvailable>().0.load(std::sync::atomic::Ordering::Relaxed)
}
```

```typescript
// Source: existing invoke pattern in App.tsx (get_engine reconciliation)
// In GeneralSection.tsx or App.tsx loadSettings():
const hookOk = await invoke<boolean>('get_hook_status');
// Pass as prop to GeneralSection → display inline warning if !hookOk
```

### Pattern 5: Frontend hook-failure warning

**What:** Inline warning rendered below `<HotkeyCapture />` in `GeneralSection.tsx`, conditionally shown when hook is unavailable. Matches existing error display style in `HotkeyCapture.tsx`.

```tsx
// Source: existing pattern in HotkeyCapture.tsx error display
{!hookAvailable && (
  <p className="mt-1 text-xs text-amber-600 dark:text-amber-400">
    Hook unavailable — using standard shortcut fallback
  </p>
)}
```

The `hookAvailable` prop flows: `App.tsx` (loads once via `invoke('get_hook_status')`) → `GeneralSection` props → rendered below `<HotkeyCapture />`.

### Pattern 6: Managed state for hook availability

```rust
// Source: existing Arc<AtomicBool> pattern in lib.rs (LevelStreamActive)
pub struct HookAvailable(pub Arc<AtomicBool>);
```

Registered via `app.manage(HookAvailable(hook_available))` in `setup()`. Accessed in `get_hook_status`, `rebind_hotkey`, and any other IPC command that needs to gate on hook availability.

### Pattern 7: `PipelineState::current()` — may need adding

`rebind_hotkey` needs to check if recording is in progress. `PipelineState` currently exposes `transition()` and `reset_to_idle()` but not a read accessor. A `current()` method needs to be added to `pipeline.rs`:

```rust
pub fn current(&self) -> Phase {
    match self.0.load(Ordering::Relaxed) {
        1 => Phase::Recording,
        2 => Phase::Processing,
        _ => Phase::Idle,
    }
}
```

### Anti-Patterns to Avoid

- **Dual-active backends:** Never leave both `tauri-plugin-global-shortcut` registered AND the hook running for the same hotkey. The gap between teardown and setup is deliberate (~10ms is fine per CONTEXT.md decisions).
- **Routing logic scattered across multiple functions:** `is_modifier_only` must be the single predicate used everywhere — don't duplicate the modifier list or the routing logic.
- **Blocking the rebind on pipeline busy:** Don't spin-wait. Return `Err` immediately; the frontend's existing error display handles it gracefully.
- **Calling `app.handle().plugin()` again after startup:** `tauri_plugin_global_shortcut::Builder` is registered once at app startup via `app.handle().plugin()`. Runtime changes use `app.global_shortcut()` (the `GlobalShortcutExt` trait on the already-registered plugin). Do not call `app.handle().plugin()` in `rebind_hotkey()` — this is what the existing code already does correctly via `gs.on_shortcut()`.
- **Persisting "hook unavailable" state to settings.json:** Hook availability is a runtime condition (hardware/OS), not a user preference. Only persist the actual hotkey string. Re-detect hook availability on every startup.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Hotkey format parsing | Custom tokenizer | Simple `split('+').all(|t| MODIFIERS.contains(&t))` | The format is already well-defined and simple; over-engineering creates maintenance cost |
| Cross-thread hook status | Custom channel/event | `Arc<AtomicBool>` managed state | Already the project pattern (LevelStreamActive); zero allocations |
| Frontend state sync | WebSocket/polling loop | Single `invoke('get_hook_status')` on mount | Status is stable after startup; one query is sufficient |
| Backend-switching lock | Custom mutex around backends | `PipelineState::current()` check + immediate error return | Pipeline CAS machine already serializes state; no additional locking needed |

**Key insight:** This phase is plumbing, not a new feature. All the complex pieces (hook module, global shortcut plugin, pipeline state, settings persistence) already exist or come from Phase 15. Phase 16 only adds the routing decision and the status surface.

---

## Common Pitfalls

### Pitfall 1: Double-firing when switching from hook to plugin
**What goes wrong:** If `rebind_hotkey` registers the new plugin shortcut before unregistering the hook, both fire for a brief window if the user happens to press the old modifier key during the switch.
**Why it happens:** Out-of-order teardown/setup.
**How to avoid:** Always teardown old → then setup new. The CONTEXT.md decision ("atomic swap") makes this explicit.
**Warning signs:** During testing, pressing a key combo immediately after rebind causes two `handle_shortcut` invocations back-to-back.

### Pitfall 2: `unregister_hotkey` / `register_hotkey` IPC commands not updated
**What goes wrong:** `HotkeyCapture.tsx` calls `invoke('unregister_hotkey')` when entering capture mode and `invoke('register_hotkey')` when cancelling. If these commands don't route to the hook backend for modifier-only hotkeys, the hook keeps firing while the user is trying to capture a new hotkey.
**Why it happens:** Only `rebind_hotkey` gets routing logic; the other two commands are forgotten.
**How to avoid:** All three IPC commands — `rebind_hotkey`, `unregister_hotkey`, `register_hotkey` — must call `is_modifier_only()` and route accordingly.
**Warning signs:** Capture mode opens, then Ctrl+Win still triggers dictation.

### Pitfall 3: Hook availability probe at startup for standard-combo users
**What goes wrong:** If you always install+uninstall the hook at startup to probe availability (even when the saved hotkey is a standard combo), you introduce unnecessary hook churn and log noise on every launch for the majority of v1.1 users who haven't switched to Ctrl+Win yet.
**Why it happens:** Eagerness to pre-populate `HookAvailable` for all users.
**How to avoid:** Only install the hook at startup when the saved hotkey is modifier-only. When the saved hotkey is standard, leave `HookAvailable` as `false` (unknown/not-tested). If the user later tries to select a modifier-only combo, attempt hook install at that point and update the flag. This lazy-probe approach is simpler and avoids churn.
**Warning signs:** Log shows "WH_KEYBOARD_LL installed" and "WH_KEYBOARD_LL uninstalled" on every startup even for standard-hotkey users.

### Pitfall 4: Fallback hotkey not persisted
**What goes wrong:** At startup, hook install fails, app falls back to `ctrl+shift+space` in memory and registers it. But settings.json still says `ctrl+win`. On next startup, app tries the hook again, fails again, and goes through the same fallback path — correct, but noisy. More critically, the frontend reads settings.json for the displayed hotkey and shows `Ctrl + Win` even though the active hotkey is `Ctrl + Shift + Space`.
**Why it happens:** Fallback is applied in memory but not written to disk.
**How to avoid:** When hook fails at startup and the fallback is applied, write the fallback hotkey to settings.json immediately. This keeps the displayed hotkey in sync and makes the next startup clean.
**Warning signs:** Settings UI shows `Ctrl + Win` but the actual active shortcut is `Ctrl + Shift + Space`.

### Pitfall 5: `unregister_hotkey` called with a modifier-only key on the plugin
**What goes wrong:** `unregister_hotkey` currently calls `app.global_shortcut().unregister(key)`. If the active hotkey is `ctrl+win` (a modifier-only/hook combo), unregistering it via the plugin is a no-op at best and returns an error at worst (the plugin never registered it). The hook keeps running.
**Why it happens:** `unregister_hotkey` is not updated alongside `rebind_hotkey`.
**How to avoid:** Same routing logic in all three IPC commands (see Pitfall 2).

### Pitfall 6: `write_settings` signature mismatch in `setup()`
**What goes wrong:** `write_settings` takes `&tauri::AppHandle`, but in the `setup()` closure the available handle is `&tauri::App`. The existing startup code uses `read_saved_hotkey(app: &tauri::App)` to get an App reference, then later uses `app.handle()` for operations needing AppHandle. Writing settings during the fallback path in setup() needs care to use the right type.
**Why it happens:** Tauri 2 distinguishes `App` (setup-time) from `AppHandle` (runtime). `write_settings` signature in lib.rs uses AppHandle.
**How to avoid:** Use `app.handle()` to get the AppHandle when calling `write_settings` inside the setup() closure. This is already the pattern used elsewhere in setup().
**Warning signs:** Compile error: "expected `&AppHandle`, found `&App`" in setup().

---

## Code Examples

Verified patterns from the existing codebase and Context7:

### GlobalShortcutExt runtime unregister + re-register

```rust
// Source: existing rebind_hotkey() in lib.rs:495 + Context7 /tauri-apps/tauri-plugin-global-shortcut
use tauri_plugin_global_shortcut::GlobalShortcutExt;

// Unregister (already used in production):
app.global_shortcut().unregister("ctrl+shift+space").map_err(|e| e.to_string())?;

// Register with handler (already used in production):
app.global_shortcut()
    .on_shortcut("ctrl+shift+v", |app, _shortcut, event| {
        handle_shortcut(app, &event);
    })
    .map_err(|e| e.to_string())?;

// Check if registered (confirmed by Context7):
if app.global_shortcut().is_registered("ctrl+shift+v") { ... }
```

### Managed state access in IPC command

```rust
// Source: existing get_engine() in lib.rs:273
#[tauri::command]
fn get_hook_status(app: tauri::AppHandle) -> bool {
    app.state::<HookAvailable>().0.load(std::sync::atomic::Ordering::Relaxed)
}
```

### Frontend: invoke + prop pattern for startup status

```typescript
// Source: existing App.tsx:71 (get_engine reconciliation pattern)
// In loadSettings() in App.tsx:
const hookAvailable = await invoke<boolean>('get_hook_status').catch(() => true);
// Pass as prop:
<GeneralSection hookAvailable={hookAvailable} ... />
```

### `is_modifier_only` — token-based predicate

```rust
// Source: derived from CONTEXT.md routing decision
const MODIFIER_TOKENS: &[&str] = &["ctrl", "alt", "shift", "meta", "win", "super"];

fn is_modifier_only(hotkey: &str) -> bool {
    if hotkey.is_empty() { return false; }
    hotkey.split('+')
          .filter(|t| !t.is_empty())  // guard against "ctrl+" edge case
          .all(|t| MODIFIER_TOKENS.contains(&t.to_lowercase().as_str()))
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Single backend (tauri-plugin-global-shortcut only) | Dual backend with routing | v1.2 (Phase 15+16) | rebind_hotkey, unregister_hotkey, register_hotkey all need routing |
| Default hotkey: `ctrl+shift+space` | Default hotkey: `ctrl+win` for fresh installs | Phase 15 decision | v1.1 users keep their saved hotkey; Phase 15 changes the code default; Phase 16 inherits this |

**Deprecated/outdated:**
- Current `rebind_hotkey()` (lib.rs:495-518): Assumes all hotkeys go to `tauri-plugin-global-shortcut`. Phase 16 replaces this with the routed version.
- Current `unregister_hotkey()` / `register_hotkey()`: Same assumption. Phase 16 updates both.

---

## Open Questions

1. **Phase 15 hook module public API shape**
   - What we know: Phase 15 delivers `keyboard_hook.rs` with install/uninstall/is_active functions. CONTEXT.md for Phase 15 says "Phase 16 calls its start/stop functions". The 16-CONTEXT.md says Phase 15's module "exposes install/uninstall/is_active API".
   - What's unclear: Exact function signatures (`install(app: &AppHandle)` vs `install(handle: HookHandle)` vs something else). Whether install returns a handle that must be stored or installs globally.
   - Recommendation: Phase 16 planning tasks must be written against the Phase 15 contract as specified in 15-CONTEXT.md. During implementation, adjust call sites to match actual Phase 15 API. This is not a blocker for planning — just a dependency to note.

2. **`unregister_hotkey` / `register_hotkey` during capture mode for modifier-only hotkeys**
   - What we know: `HotkeyCapture.tsx` calls `unregister_hotkey` to suppress the active shortcut while the user selects a new one. For a hook-based combo like `ctrl+win`, "unregistering" means stopping the hook thread.
   - What's unclear: Should the hook be fully stopped (thread torn down) during capture mode, or just paused (hook installed but ignoring events)? Phase 15 may provide a "pause" API, or it may not.
   - Recommendation: Plan for full stop/start (thread teardown+respawn) during capture mode, matching the same pattern used for standard combos. If Phase 15 provides a lighter "pause" mechanism, use it. Brief gap during capture is acceptable per CONTEXT.md.

3. **Startup hook-available detection for existing v1.1 users with standard hotkeys**
   - What we know: Pitfall 3 recommends lazy probe (only install hook if saved hotkey is modifier-only). But `HookAvailable` will be `false` for standard-combo users even if their hardware would support the hook.
   - What's unclear: Should `get_hook_status` report `false` (not needed) or `unknown` for standard-combo users? The CONTEXT.md UX only calls for showing the warning when hook installation actually failed — not when it was never tried.
   - Recommendation: `HookAvailable(false)` for standard-combo users is correct. The warning only shows when the hook was needed and failed. If a standard-combo user later tries to select `ctrl+win` via `HotkeyCapture`, the routing code in `rebind_hotkey` can attempt hook install at that point and return an appropriate error if it fails.

---

## Validation Architecture

> `workflow.nyquist_validation` is not present in `.planning/config.json` — this section is omitted as validation is not configured for this project.

The existing test infrastructure uses Rust's built-in `#[cfg(test)]` module pattern (see `corrections_tests.rs`). The `is_modifier_only` helper is a pure function and is a strong candidate for inline unit tests (see Pattern 1 above). These are cheap, fast, and require no additional test framework.

---

## Sources

### Primary (HIGH confidence)
- `/tauri-apps/tauri-plugin-global-shortcut` (Context7) — `GlobalShortcutExt::on_shortcut`, `unregister`, `is_registered`, `unregister_all` runtime API confirmed
- `src-tauri/src/lib.rs` (direct read) — existing `rebind_hotkey`, `unregister_hotkey`, `register_hotkey`, `handle_shortcut`, `PipelineState` usage patterns
- `src-tauri/src/pipeline.rs` (direct read) — `PipelineState` API; confirmed `current()` method is missing and needs to be added
- `.planning/phases/16-rebind-and-coexistence/16-CONTEXT.md` (direct read) — all routing and UX decisions
- `.planning/phases/15-hook-module/15-CONTEXT.md` (direct read) — hook module API contract, Phase 15 deliverables
- `src/components/HotkeyCapture.tsx` (direct read) — capture mode flow, existing error display pattern
- `src/components/sections/GeneralSection.tsx` (direct read) — settings panel structure for warning placement
- `src/App.tsx` (direct read) — `invoke('get_engine')` pattern for backend status query on mount

### Secondary (MEDIUM confidence)
- None required — all critical patterns verified from primary sources.

### Tertiary (LOW confidence)
- None.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries already in Cargo.toml and in production use
- Architecture: HIGH — all patterns directly derived from existing codebase code
- Pitfalls: HIGH — identified from direct code inspection of affected functions
- Phase 15 API: MEDIUM — API shape inferred from CONTEXT.md contract, not yet implemented

**Research date:** 2026-03-02
**Valid until:** 2026-04-02 (stable — no fast-moving dependencies; expires only if Phase 15 API shape changes significantly)
