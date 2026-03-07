# Feasibility Assessment: Auto-detect Caps Lock State

## Strategic Summary

This is trivially feasible. The codebase already uses `windows::Win32::UI::Input::KeyboardAndMouse::*` in `keyboard_hook.rs`, which includes `GetKeyState`. The change is a single API call at the existing ALL CAPS application point in `pipeline.rs`, replacing the persisted profile flag with a live OS query. No new dependencies, no architectural changes.

## What we're assessing

Replace the manual ALL CAPS toggle (stored in `settings.json` as `all_caps`, toggled via UI) with automatic detection of the physical keyboard's Caps Lock state at text injection time. If Caps Lock is on when transcription completes, output is uppercased; otherwise, normal casing.

## Technical Feasibility

**Can we build it?**

- **Known approaches:** Yes — `GetKeyState(VK_CAPITAL)` returns Caps Lock toggle state via the low bit of the return value. This is a synchronous, zero-cost Win32 call.
- **Technology maturity:** Proven — `GetKeyState` has been stable since Windows 95. The `windows` crate already exposes it (imported in `keyboard_hook.rs`).
- **Existing infrastructure:**
  - `keyboard_hook.rs` already imports `windows::Win32::UI::Input::KeyboardAndMouse::*` which includes `GetKeyState` and `VK_CAPITAL`
  - `pipeline.rs:349-358` already has the exact insertion point where ALL CAPS is applied
  - The current code reads `guard.all_caps` from the profile — replace with `GetKeyState(VK_CAPITAL)` check
- **Threading consideration:** `GetKeyState` returns per-thread key state. The pipeline runs on a Tokio blocking thread (`spawn_blocking`), not the hook thread. `GetKeyboardState` or `GetAsyncKeyState` may be more appropriate since they query the physical/async state rather than the calling thread's message-queue state. `GetAsyncKeyState(VK_CAPITAL)` checks the low bit for toggle state — this is the correct choice for a non-UI thread.
- **Technical risks:**
  - Low: `GetAsyncKeyState` toggle bit could theoretically be stale by a few ms — irrelevant for this use case since caps lock state doesn't change during a transcription
  - Low: need to verify `VK_CAPITAL` constant is accessible from `windows` crate in the pipeline module (it is — same crate, just needs a `use` statement)

**Technical verdict:** Feasible — straightforward

## Resource Feasibility

**Do we have what we need?**

- **Skills:** Already demonstrated — team has written extensive Win32 keyboard code in `keyboard_hook.rs`
- **Dependencies:** Zero new dependencies. `windows` crate already in `Cargo.toml` with the required features
- **Effort:** ~30 minutes of implementation + testing
  - Replace profile flag read with `GetAsyncKeyState` call in `pipeline.rs`
  - Remove or deprecate UI toggle + `get_all_caps`/`set_all_caps` Tauri commands
  - Remove `all_caps` from profile struct and settings persistence

**Resource verdict:** Feasible

## External Dependency Feasibility

**Are external factors reliable?**

- **APIs/services:** Win32 `GetAsyncKeyState` — built into Windows, no external dependency
- **Third-party integrations:** None
- **External data:** None

**External verdict:** Feasible — no external dependencies

## Blockers

| Blocker | Severity | Mitigation |
|---------|----------|------------|
| None identified | — | — |

## De-risking Options

- **Test on spawn_blocking thread:** Verify `GetAsyncKeyState(VK_CAPITAL)` returns correct toggle state from Tokio's blocking thread pool (not the main/hook thread). Quick manual test.
- **Keep `all_caps` as override:** If desired, keep the manual toggle as a force-on override independent of physical Caps Lock. Low complexity, optional.

## Overall Verdict

**Go**

No blockers. The codebase already has all required Win32 imports and the exact code location where the change goes. This is a small, self-contained modification.

## Implementation Context

### If Go
- **Approach:** Replace `guard.all_caps` read in `pipeline.rs:349-358` with `unsafe { GetAsyncKeyState(VK_CAPITAL) } & 0x0001 != 0`
- **Start with:** Verify `GetAsyncKeyState` toggle bit from a blocking thread (quick test)
- **Critical path:** Correct API choice (`GetAsyncKeyState` not `GetKeyState`) for cross-thread query

### Risks
- **Technical:** Minimal — toggle bit read is well-documented Win32 behavior
- **External:** None
- **Mitigation:** Manual test with Caps Lock on/off before removing UI toggle

### Alternatives
- **If blocked:** Fall back to keeping manual toggle (current behavior)
- **Simpler version:** Just add auto-detect alongside existing toggle (both paths, toggle becomes override)

### Cleanup
- Remove `get_all_caps` / `set_all_caps` Tauri commands (`lib.rs:956-1019`)
- Remove `all_caps` field from `ActiveProfile` struct (`profiles.rs:14`)
- Remove UI toggle component
- Remove `all_caps` settings migration code (`lib.rs:2019-2046`)

## Sources

- Windows API docs: `GetAsyncKeyState` — returns toggle state in low bit for toggle keys (VK_CAPITAL, VK_NUMLOCK, VK_SCROLL)
- Codebase: `keyboard_hook.rs` — confirms `windows::Win32::UI::Input::KeyboardAndMouse::*` already imported
- Codebase: `pipeline.rs:349-358` — exact location of current ALL CAPS logic
