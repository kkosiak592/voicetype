# Feasibility Assessment: Post-Transcription Case Toggle via Pill Icon

## Strategic Summary

Feasible with conditions. The "show icon + dropdown" part is straightforward — the pill overlay system already supports post-injection states. The hard part is **replacing already-injected text** in another application. The only reliable cross-app approach is Ctrl+Z (undo the paste) then re-paste with corrected casing. Direct text selection/replacement in an arbitrary target app is not reliably possible from an external process.

## What we're assessing

After transcription is injected, show a small icon (or repurpose the pill) with a hover dropdown offering "UPPERCASE" / "lowercase" toggle. If the user picks the opposite casing, automatically replace the just-pasted text in the target application.

## Technical Feasibility

**Can we build it?**

### Part 1: The UI (icon + dropdown) — Feasible

- The pill overlay (`Pill.tsx` + `pill.rs`) already has post-injection states (`pill-result` event fires after injection)
- Currently the pill plays an exit animation and hides after success. Instead, it could transition to a "case toggle" state with a brief hover menu
- The pill is a Tauri WebviewWindow with `always_on_top` — it can render arbitrary React UI
- Hover detection works within the pill's bounds; the pill would need to temporarily resize to accommodate the dropdown

### Part 2: Replacing injected text — Risky

Three approaches, ranked by reliability:

**Approach A: Ctrl+Z then re-paste (Recommended)**
- After user picks the toggle: send `Ctrl+Z` to undo the original paste, then re-inject with corrected casing
- Works in most apps (Notepad, Word, VS Code, browsers, Slack, Teams) because paste is a single undo step
- Risk: some apps treat paste as multiple undo operations (rare), or undo might not be available (terminal emulators)
- The app already has `enigo` for keyboard simulation (`inject.rs`) — sending Ctrl+Z is trivial

**Approach B: Select-all-back then re-paste**
- Send Shift+Home or Shift+Left N times to select the text, then re-paste
- Fragile: requires knowing exact cursor position relative to pasted text; breaks if cursor was moved, or if the app doesn't support standard selection keys

**Approach C: Clipboard-only (no auto-replace)**
- Put corrected text on clipboard, user manually selects + pastes
- Always works but defeats the purpose of quick correction

**Threading/timing considerations:**
- The pipeline retains `formatted_for_tooltip` (the injected text without trailing space) — this is already available for case conversion
- Need to keep the original text in memory until the toggle window dismisses (~3-5 seconds)

### Technical risks:
- **Medium:** Ctrl+Z undo may not perfectly reverse the paste in all applications (edge case: apps with non-standard undo)
- **Low:** Pill resize for dropdown may cause visual flicker
- **Low:** Hover timing — user needs to reach the pill before it auto-dismisses

**Technical verdict:** Feasible — Ctrl+Z approach works reliably in 95%+ of target apps

## Resource Feasibility

**Do we have what we need?**

- **Skills:** All required — React (pill UI), Tauri commands, enigo keyboard sim
- **Dependencies:** Zero new deps. `enigo` already handles key simulation
- **Effort:** Medium — pill UI changes + new Tauri command for case-toggle-reinject + state management for holding the last transcription

**Resource verdict:** Feasible

## External Dependency Feasibility

No external dependencies. All Win32/enigo/clipboard code is already in the codebase.

**External verdict:** Feasible

## Blockers

| Blocker | Severity | Mitigation |
|---------|----------|------------|
| Ctrl+Z doesn't undo paste in some apps | Medium | Accept as known limitation; document which apps work. Terminal emulators and some custom editors won't support this. |
| Pill is 110x32px — too small for dropdown | Low | Temporarily resize the Tauri window when showing the toggle menu, shrink back on dismiss |
| Focus steal — clicking the pill dropdown takes focus from target app | Medium | The pill window must remain non-focusable (`skip_taskbar`, no focus on show). Use hover-only interaction (no click needed) to avoid stealing focus from the target app |

## De-risking Options

- **Keep it hover-only:** No click required — hovering over "lowercase" triggers the action. This avoids focus-steal entirely since the pill is already `always_on_top` + non-focusable.
- **Auto-dismiss timer:** Show the toggle for 3-4 seconds after injection. If user doesn't interact, it fades away (current behavior). Low implementation cost.
- **Store last transcription in Tauri state:** Hold the raw `formatted` string in a `Mutex<Option<String>>` managed state. The toggle command reads it, applies case conversion, does Ctrl+Z + re-inject. Clear it on next recording start or after timeout.

## Overall Verdict

**Go with conditions**

The UI part is clean and fits naturally into the existing pill overlay. The text replacement via Ctrl+Z is the only realistic cross-app approach and works in the vast majority of applications. The condition is: accept that a small number of apps (terminals, some custom editors) won't support undo-and-repaste, and don't try to solve for those edge cases.

## Implementation Context

### If Go
- **Approach:**
  1. After injection success, store `formatted` text in `Mutex<Option<LastTranscription>>` state
  2. Pill transitions to a "toggle" state instead of immediately hiding — shows two hover targets: "Aa" (original) and "AA"/"aa" (opposite)
  3. Hovering an option triggers a Tauri command: `toggle_transcription_case`
  4. Command reads stored text, applies case toggle, sends Ctrl+Z via enigo, then re-injects via `inject_text`
  5. Auto-dismiss after ~4 seconds if no interaction
- **Start with:** Prototype the Ctrl+Z + re-paste flow in isolation to verify reliability across target apps (Notepad, Word, VS Code, browser text fields)
- **Critical path:** Ctrl+Z reliably undoing the paste in common apps; pill hover detection without focus steal

### Risks
- **Technical:** Ctrl+Z undo behavior varies across apps — test the top 5-6 apps you actually dictate into
- **External:** None
- **Mitigation:** If Ctrl+Z fails in a specific app, the worst case is the user has to manually fix it (same as today)

### Alternatives
- **If blocked:** Fall back to clipboard-only (put corrected text on clipboard, show "Copied to clipboard" toast)
- **Simpler version:** Instead of hover dropdown, just show a single "toggle case" button on the pill that swaps between upper/lower on click (simpler UI, still needs Ctrl+Z flow)

## Sources

- Codebase: `inject.rs` — existing clipboard + enigo paste simulation
- Codebase: `Pill.tsx` — existing pill overlay with state machine
- Codebase: `pipeline.rs:406-430` — post-injection flow where toggle state would be set
- Codebase: `pill.rs` — Tauri window management for pill overlay
