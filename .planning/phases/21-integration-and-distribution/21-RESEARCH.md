# Phase 21: Integration and Distribution - Research

**Researched:** 2026-03-03
**Domain:** Manual integration testing (modifier state desync recovery, Start menu suppression on Win10/Win11), binary distribution safety (VirusTotal scan of signed v1.2 NSIS installer)
**Confidence:** HIGH (codebase directly inspected; Win32 hook docs verified; VirusTotal workflow confirmed from official docs and ghaction-virustotal README)

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| DIST-01 | Signed v1.2 binary passes VirusTotal scan with no new detections vs v1.1 baseline | VirusTotal submission process via ghaction-virustotal GitHub Action; scan is manual gate — no automation can "pass" it, only surface results. v1.1 baseline is available from the existing v1.1.0 GitHub Release. |
</phase_requirements>

---

## Summary

Phase 21 is purely a verification and distribution phase, not an implementation phase. No new Rust, TypeScript, or configuration code is written. The phase has two orthogonal concerns: (1) manual integration testing of runtime behaviors that the unit test infrastructure cannot cover — specifically modifier state desync recovery and Start menu suppression on both Windows 10 and Windows 11; and (2) scanning the signed v1.2 binary on VirusTotal and confirming no new detections relative to the v1.1 baseline before distribution.

The modifier state desync scenario (Success Criterion 1) is a real risk the codebase addresses architecturally: `WH_KEYBOARD_LL` is a global hook that receives key events regardless of which window has focus, so Ctrl-up after Alt+Tab WILL be delivered. However, the success criterion tests the specific path: hold Ctrl (no Win), Alt+Tab away from VoiceType, release Ctrl outside VoiceType. The hook receives the Ctrl-up, clears `ctrl_held`, and no phantom session can start. The `reset_state()` function exists but is only called on hook thread exit (shutdown), not on window deactivation — this is correct, because the global hook already handles all key events regardless of focus. The test confirms this is sufficient.

The VirusTotal scan is a blocking gate before distribution. The project already uses `TAURI_SIGNING_PRIVATE_KEY` in the GitHub Actions release workflow for Ed25519 update signing, but does NOT use a code-signing certificate — the binary is unsigned in the Win32/SmartScreen sense. Known Tauri false positives (Trojan.Heur.LShot.1 from BitDefender/SecLookup/MaxSecure) are endemic to the NSIS installer pattern and pre-date the v1.2 hook additions. The v1.2 additions (WH_KEYBOARD_LL + SendInput) are recognized keylogger API patterns by antivirus ML models — the STATE.md research flag confirms that the actual impact "cannot be determined pre-build." Any new detection above the v1.1 baseline is a blocking issue requiring investigation before distribution.

**Primary recommendation:** Phase 21 is two tasks — (1) run the three manual integration tests and document results; (2) build the v1.2 release via the existing `bundle-release` skill, upload the NSIS installer to VirusTotal, compare against the v1.1 baseline, and gate distribution on no new detections.

---

## Standard Stack

### Core

| Tool | Version | Purpose | Why Standard |
|------|---------|---------|--------------|
| `bundle-release` skill | project skill | Full release workflow: version bump, version file updates, CHANGELOG update, tag, push to trigger CI | Already exists in `.claude/skills/bundle-release/SKILL.md`; gates every step on user approval |
| VirusTotal web UI | n/a | Upload NSIS installer, view detection counts by engine | Free, no API key needed for manual scan; sufficient for a one-time distribution gate |
| `ghaction-virustotal` GitHub Action | latest | Automated VirusTotal scan of release assets in CI | Optional addition to release.yml; useful if automation is wanted; `crazy-max/ghaction-virustotal` supports scanning release assets and appending analysis links to release notes |
| Windows 10 (test machine) | Win10 | Verify Ctrl+Win behavior and Start menu suppression on v1.1 target OS | Already the primary dev OS per tauri.conf.json NSIS target |
| Windows 11 (test machine) | Win11 | Verify Ctrl+Win behavior on Win11 (Start menu trigger mechanism differs) | STATE.md research flag: VK_E8 KEYDOWN-only vs KEYDOWN+KEYUP requires empirical validation on Win11 |

### Supporting

| Tool | Version | Purpose | When to Use |
|------|---------|---------|-------------|
| VirusTotal API (free tier) | v3 | Programmatic scan if adding ghaction-virustotal to CI | 4 req/min, 500 req/day limits; sufficient for a single file per release |
| GitHub Releases | v1.1.0 tag | v1.1 baseline VirusTotal results | Compare detection count/engines from v1.1 scan to v1.2 scan to identify regressions |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Manual VirusTotal upload | `ghaction-virustotal` GitHub Action in release.yml | Action automates scanning and appends results to release notes — useful long term, but adds setup for this phase; manual is sufficient for one-time gate |
| Manual integration testing | Automated UI tests | The three success criteria require focus switching and physical modifier-key hold scenarios that cannot be mocked without full OS interaction; manual testing is the only option here |

**Installation:** No new dependencies. Phase 21 is verification-only.

---

## Architecture Patterns

### Pattern 1: Modifier State Desync Test Protocol

**What:** Manual test that confirms the hook's global-hook nature handles Alt+Tab correctly without phantom sessions.

**What the code does (confirmed from codebase inspection):**

The `WH_KEYBOARD_LL` hook in `keyboard_hook.rs` is a **global** hook — it fires for every keyboard event on the entire desktop, regardless of which window has focus. When the user holds Ctrl, then Alt+Tab away, then releases Ctrl:

1. Alt-down is received by hook → not a tracked modifier, passes through via `CallNextHookEx`
2. Tab-down is received by hook → not a tracked modifier, passes through
3. Focus switches to another window — the hook continues to receive events (global hook)
4. Ctrl-up is received by hook → `STATE.ctrl_held.store(false, Relaxed)` at line 275 of `keyboard_hook.rs`
5. At this point `combo_active` is `false` (Win was never pressed), so the Ctrl-up branch returns `CallNextHookEx` — no Released event sent, no phantom recording

On returning to VoiceType: `ctrl_held = false`, `win_held = false`, `combo_active = false` — clean state. Pressing Win or Ctrl next will start a fresh debounce timer. No phantom recording can occur.

**Desync risk that DOES exist:** If Windows swallows a key-up event for a modifier during a system-level operation (screen lock, UAC prompt, fast user switch), the `ctrl_held` or `win_held` flag could remain `true` indefinitely. The current code has `reset_state()` only on hook thread exit (shutdown). This is acceptable for the Alt+Tab scenario but is a potential issue for screen-lock scenarios — however, this is explicitly deferred to v2 (HOOK-05 in REQUIREMENTS.md).

**Test steps for Success Criterion 1:**
1. Start VoiceType with Ctrl+Win hotkey active
2. Hold Ctrl (do not press Win)
3. Press Alt+Tab to switch to another window (release Alt+Tab, keep holding Ctrl)
4. Release Ctrl (while focus is on another window)
5. Alt+Tab back to VoiceType
6. Do NOT press any hotkey
7. Verify: no recording starts, pill overlay does not appear, tray icon remains idle

**Expected hook state after step 4:** `ctrl_held = false`, `win_held = false`, `combo_active = false`

### Pattern 2: Start Menu Suppression Test Protocol

**What:** Manual test on both Windows 10 and Windows 11 to verify Success Criterion 3.

**Test steps:**
1. Press and release Win alone → Start menu opens (MOD-05: Win alone opens Start menu)
2. Press and release Ctrl+Win → dictation activates, Start menu does NOT open (MOD-04)
3. Press Ctrl, then press Win → same behavior as step 2

**Win11 known risk from STATE.md:** VK_E8 KEYDOWN-only injection may not suppress the Start menu on Windows 11. If step 2 above shows the Start menu opening on Win11, add KEYUP injection to `inject_mask_key()`:

```rust
// If KEYDOWN-only fails on Windows 11, add KEYUP injection:
unsafe fn inject_mask_key() {
    let inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0xE8),
                    wScan: 0,
                    dwFlags: KEYBD_EVENT_FLAGS(0),  // KEYDOWN
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0xE8),
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,  // KEYUP
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];
    SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
}
```

This is a code change needed only if Success Criterion 3 fails on Windows 11. It is a conditional finding — test first, implement only if needed.

### Pattern 3: VirusTotal Scan Workflow

**What:** Manual process to scan the signed v1.2 NSIS installer and compare against v1.1 baseline.

**Step 1 — Get v1.1 baseline (if not already documented):**
- Navigate to `https://github.com/kkosiak592/voicetype/releases/tag/v1.1.0`
- Download the NSIS installer (`VoiceType_1.1.0_x64-setup.exe` or equivalent filename)
- Upload to `https://www.virustotal.com` and record:
  - Total engines checked
  - Number of detections
  - Which specific engines flagged it

**Step 2 — Build and scan v1.2:**
- Use the `bundle-release` skill to trigger the GitHub Actions release pipeline
- Wait for the CI build to complete (expected: 20-40 minutes based on SKILL.md)
- Download the v1.2 NSIS installer from the GitHub Release assets
- Upload to VirusTotal and record results in the same format

**Step 3 — Compare and gate:**
- If v1.2 has the SAME engines flagging as v1.1 (e.g., BitDefender, SecLookup, MaxSecure with "Trojan.Heur.LShot.1"): this is a known Tauri false positive pattern, not a v1.2 regression → distribution is safe
- If v1.2 shows NEW engines flagging that did NOT flag v1.1: this is a blocking issue — investigate whether WH_KEYBOARD_LL + SendInput triggered new ML detections before distributing

**Known Tauri false positives (v1.1 baseline context):**
From community research (Tauri issues #4749, #2486, #10649): BitDefender, SecLookup, MaxSecure commonly flag Tauri NSIS installers with `Trojan.Heur.LShot.1`. These are false positives endemic to the NSIS installer pattern, not related to app behavior. If these same engines flag v1.2, it is not a new detection.

**v1.2 new API risk — WH_KEYBOARD_LL + SendInput:**
The STATE.md research flag states: "Defender ML sensitivity for WH_KEYBOARD_LL + SendInput on unsigned vs signed binary cannot be determined pre-build." These are the exact Win32 APIs used by keyloggers. The Ed25519 signing (`TAURI_SIGNING_PRIVATE_KEY`) used in `release.yml` is for the Tauri updater (content integrity), NOT for Win32 code signing (SmartScreen/AV reputation). The binary remains unsigned in the Windows Authenticode sense, meaning AV ML heuristics rely entirely on binary static analysis — the new APIs are visible in the binary's import table and call patterns.

**What to do if new detections appear:**
1. Identify which new engine(s) flagged v1.2 but not v1.1
2. Check if the detection name relates to keylogger/hook patterns (e.g., "Heuristic.KEYLOGGER", "Trojan.Hook", "SpyWare.*")
3. Submit the binary as a false positive to those specific vendors (NOT to VirusTotal — VirusTotal is an aggregator and cannot fix individual vendor detections)
4. Optionally: file report with VirusTotal community to add context for other users
5. Do NOT distribute until major/trusted AV vendors (Symantec, ESET, Kaspersky, Malwarebytes, Windows Defender) are clear

### Anti-Patterns to Avoid

- **Treating Phase 18 as an implementation phase:** No new code is required unless the Win11 Start menu test fails, in which case the KEYUP injection fix is the only permitted change. Do not add features, refactor, or make unrelated changes.
- **Distributing before VirusTotal comparison:** DIST-01 requires comparison against v1.1 baseline, not just "scan passed." A scan that shows 3 engines flagging v1.2 when 0 flagged v1.1 is a blocking issue even if 3 is a "low" absolute count.
- **Conflating Tauri updater signing with code signing:** The `TAURI_SIGNING_PRIVATE_KEY` in `release.yml` is Ed25519 signing for `tauri-plugin-updater` integrity verification, not Windows Authenticode. The app remains unsigned from SmartScreen/AV perspective.
- **Running all three test scenarios in sequence without resetting state:** Between test runs, terminate and restart VoiceType to ensure a clean hook state. Reusing the same running instance between tests could contaminate results.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Release build | Manual cargo build + sign steps | `bundle-release` skill | Handles version bumps in all 3 files, CHANGELOG, tag, push; CI then handles build and upload |
| VirusTotal comparison tracking | Custom spreadsheet or CI gate | Manual documentation in phase SUMMARY | This is a one-time gate for each release; full automation adds infrastructure overhead without proportional benefit for a <20 user app |

---

## Common Pitfalls

### Pitfall 1: Testing on only one OS
**What goes wrong:** Success Criterion 3 requires BOTH Windows 10 and Windows 11 testing. Testing only on Windows 10 misses the Win11 VK_E8 timing issue flagged in STATE.md.
**Why it happens:** Dev machine is Windows 10; Windows 11 requires a second machine or VM.
**How to avoid:** Test on both. If a second machine is unavailable, use a Windows 11 VM. The Win11 Start menu behavior difference is a documented behavioral change in Windows 11's Start menu trigger mechanism.
**Warning signs:** Success Criterion 3 says "on both Windows 10 and Windows 11" — if only one OS is tested, the criterion is not met.

### Pitfall 2: Scanning an unsigned local build instead of the CI-built signed release
**What goes wrong:** Local debug or release builds behave differently on VirusTotal than CI-built NSIS installers. The CI build uses TAURI_SIGNING_PRIVATE_KEY and tauri-action's build pipeline, producing a different artifact.
**Why it happens:** Impatience to get VirusTotal results before waiting for CI.
**How to avoid:** Always scan the GitHub Release artifact produced by the CI pipeline. This is the binary that will be distributed. The v1.1.0 baseline should also be from the GitHub Release, not a local build.
**Warning signs:** Filename pattern: local builds output to `target/release/` and are not NSIS-packaged; CI builds produce `VoiceType_X.Y.Z_x64-setup.exe`.

### Pitfall 3: Confusing combo_active desync with Alt+Tab desync
**What goes wrong:** Conflating two different desync scenarios — (a) Alt+Tab while Ctrl-only held vs (b) Alt+Tab while combo is active (recording in progress). Scenario (a) is what Success Criterion 1 tests; scenario (b) is not covered by any success criterion.
**Why it happens:** The success criterion description is specific but easy to misread.
**How to avoid:** Read the success criterion literally: "hold Ctrl, Alt+Tab, release Ctrl, return — no phantom." This tests that `ctrl_held` does not cause a phantom recording when Win is pressed later. The recording-in-progress Alt+Tab case is different behavior (recording continues since the hook still fires).
**Warning signs:** Testing scenario (b) instead of (a), concluding "recording continues as expected" and marking criterion passed — this is the wrong test.

### Pitfall 4: Interpreting VirusTotal detection count as absolute truth
**What goes wrong:** "3 detections on VirusTotal" is reported as "3 viruses detected" — triggering unnecessary alarm or causing distribution to be blocked when the same 3 engines flagged v1.1.
**Why it happens:** VirusTotal's per-engine results are not weighted by engine quality or reputation.
**How to avoid:** Compare v1.2 against v1.1 on the same specific engines. The key question is: are there NEW engines flagging v1.2 that did not flag v1.1? The absolute count is secondary.
**Warning signs:** "VirusTotal shows 4 detections" reported without specifying which engines or whether they are the same engines that flagged v1.1.

### Pitfall 5: Testing the modifier desync without an active hook (standard hotkey users)
**What goes wrong:** If the user saved a standard hotkey (e.g., ctrl+shift+space), the hook is NOT active. The Alt+Tab desync test is meaningless — no hook, no state machine, no desync risk.
**Why it happens:** Testing on a machine where the saved hotkey is not ctrl+win.
**How to avoid:** Before running the desync test, verify the active hotkey is ctrl+win (modifier-only). Check settings.json or the settings UI. If using a standard hotkey, switch to ctrl+win first and restart the app.
**Warning signs:** App uses tauri-plugin-global-shortcut backend; pressing ctrl+win registers as separate key events with no state machine.

---

## Code Examples

### Conditional Win11 Fix — VK_E8 KEYUP injection (apply only if Win11 test fails)

This change is ONLY needed if Success Criterion 3 fails on Windows 11. The file to modify is `src-tauri/src/keyboard_hook.rs`:

```rust
// Source: AHK MenuMaskKey docs — KEYDOWN+KEYUP variant for Windows 11 compatibility
unsafe fn inject_mask_key() {
    use windows::Win32::UI::Input::KeyboardAndMouse::*;

    // KEYDOWN only is standard (works on Win10). If Win11 Start menu still opens,
    // add KEYUP injection (inputs[1]) — the two-event sequence is the safe fallback.
    let inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0xE8),
                    wScan: 0,
                    dwFlags: KEYBD_EVENT_FLAGS(0),  // KEYDOWN
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0xE8),
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,       // KEYUP
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];
    // Only send both if Win11 test required it; otherwise use &inputs[..1]
    SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
}
```

### ghaction-virustotal Optional CI Integration

If adding automated VirusTotal scanning to `release.yml` is desired (not required for this phase):

```yaml
# Add as a separate job in .github/workflows/release.yml
# Runs after publish-release job completes

  virustotal-scan:
    needs: publish-release
    runs-on: ubuntu-latest
    steps:
      - name: Scan release assets with VirusTotal
        uses: crazy-max/ghaction-virustotal@v4
        with:
          vt_api_key: ${{ secrets.VT_API_KEY }}
          files: |
            *.exe
          update_release_body: true
          request_rate: 4
```

Requires `VT_API_KEY` GitHub repository secret (free VirusTotal account API key).

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| No VirusTotal gate (v1.0, v1.1) | Explicit VirusTotal scan required as DIST-01 gate for v1.2 | v1.2 REQUIREMENTS.md | New because v1.2 adds WH_KEYBOARD_LL + SendInput — keylogger API pattern that did not exist in v1.1 |
| Manual release steps | `bundle-release` skill in `.claude/skills/` | Phase ≤13 (established before v1.2) | Full release workflow gated on user approval at each step; CI handles build + upload |
| WH_KEYBOARD_LL desync untested | Phase 18 explicit integration test | Phase 18 success criteria | Tests specific runtime behavior unverifiable by unit tests |

---

## Open Questions

1. **Does the Alt+Tab test need a code change?**
   - What we know: WH_KEYBOARD_LL is global; Ctrl-up is received even when focus is elsewhere; `ctrl_held` is cleared on Ctrl-up; no phantom recording possible in the specific scenario tested.
   - What's unclear: Are there OS-level operations (UAC prompt, fast user switch) that could swallow a modifier key-up and leave state stale? These are real risks but are deferred to v2 (HOOK-05, HOOK-06).
   - Recommendation: Run the test as specified. If it passes, no code change needed. If it fails (phantom recording starts), investigate whether `reset_state()` should be called on a window-activation event — but first confirm the failure is reproducible.

2. **Is the v1.1 VirusTotal baseline documented anywhere?**
   - What we know: v1.1.0 GitHub Release exists (`git tag v1.1.0`). The installer is in the release assets. No VirusTotal scan result was documented in the planning artifacts.
   - What's unclear: Whether the v1.1 scan was ever performed, or what the detection count was.
   - Recommendation: Perform the v1.1 scan first (or in parallel with v1.2) to establish the baseline before comparing. Document results in the phase SUMMARY.

3. **Win11 VK_E8 KEYDOWN-only vs KEYDOWN+KEYUP**
   - What we know: KEYDOWN-only is the standard AHK MenuMaskKey behavior. STATE.md flags this as requiring empirical validation. Win11 redesigned the Start menu trigger.
   - What's unclear: Whether Windows 11 24H2 (the version that broke Alt+Tab in PCWorld's reporting from Nov 2024) also changed the Win key trigger mechanism.
   - Recommendation: Test on Windows 11. If the Start menu opens when it should not, apply the KEYUP injection fix (code shown above). Document which Windows 11 build was tested (winver).

---

## Validation Architecture

`workflow.nyquist_validation` is not present in `.planning/config.json` — validation section omitted.

The three success criteria are all **manual-only** tests by nature:

| Success Criterion | Test Type | Automated? | Reason |
|-------------------|-----------|-----------|--------|
| SC-1: Alt+Tab modifier desync recovery | Manual integration | No | Requires physical key holds across focus changes; cannot be mocked without full OS interaction framework |
| SC-2: VirusTotal no new detections vs v1.1 | Manual artifact scan | No (optional CI) | Requires build artifact + VirusTotal API; is a distribution gate not a test |
| SC-3: Ctrl+Win on Win10 and Win11 | Manual integration on both OS | No | Requires two physical machines or VMs; OS-specific Start menu behavior |

No Wave 0 gaps to address — this phase has no automated test infrastructure.

---

## Sources

### Primary (HIGH confidence)
- `src-tauri/src/keyboard_hook.rs` (direct read, 363 lines) — `reset_state()` call sites, `combo_active` state machine, modifier tracking confirmed; `reset_state()` only called on hook thread exit (line 145)
- `src-tauri/src/lib.rs` (direct read) — `is_modifier_only`, `HookAvailable`, startup routing, `handle_hotkey_event`; confirmed global hook fires regardless of focus
- `.planning/phases/15-hook-module/15-VERIFICATION.md` (direct read) — all 17 phase 15 truths verified, including LLKHF_INJECTED guard, VK_E8 injection, global hook behavior
- `.planning/phases/16-rebind-and-coexistence/16-VERIFICATION.md` (direct read) — all 10 phase 16 truths verified
- [LowLevelKeyboardProc - Win32 Docs](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelkeyboardproc) — "The system calls this function every time a new keyboard input event is about to be posted into a thread input queue" — global scope confirmed; no documented cases of key-up events being suppressed for focus changes
- `.github/workflows/release.yml` (direct read) — TAURI_SIGNING_PRIVATE_KEY is Ed25519 updater signing, NOT Authenticode; binary is unsigned from AV perspective
- `src-tauri/tauri.conf.json` (direct read) — NSIS installer target confirmed; `createUpdaterArtifacts: v1Compatible`
- [VirusTotal false positive docs](https://docs.virustotal.com/docs/false-positive) — VirusTotal is an aggregator; false positives must be submitted to individual vendors, not to VirusTotal

### Secondary (MEDIUM confidence)
- [ghaction-virustotal README](https://github.com/crazy-max/ghaction-virustotal) — GitHub Action supports scanning release assets; free tier 4 req/min; `update_release_body: true` appends analysis links to release notes
- Tauri GitHub issues #4749, #10649, #2486 — known false positive engines (BitDefender, SecLookup, MaxSecure) with "Trojan.Heur.LShot.1" pattern for NSIS installer; basic Tauri apps without customizations did not trigger; custom installer features (firewall rules) were identified as cause in at least one case
- STATE.md research flag — "Defender ML sensitivity for WH_KEYBOARD_LL + SendInput on unsigned vs signed binary cannot be determined pre-build — VirusTotal scan of actual v1.2 binary is the gate"

### Tertiary (LOW confidence — needs validation)
- Windows 11 Start menu VK_E8 KEYDOWN-only vs KEYDOWN+KEYUP — no authoritative source; empirical validation required during Phase 18 testing (this is the same flag from Phase 15 research, still unresolved)

---

## Metadata

**Confidence breakdown:**
- Alt+Tab desync test protocol: HIGH — codebase directly confirms global hook receives all key events; `reset_state()` only on shutdown is correct for this scenario; test steps directly derived from success criterion
- VirusTotal workflow: HIGH — process confirmed from official VirusTotal docs and ghaction-virustotal README; v1.1 baseline available from GitHub Release
- Win11 Start menu test: MEDIUM — test steps are clear; fix is known; whether a fix is needed remains LOW confidence until empirical testing
- Win11 VK_E8 fix correctness: MEDIUM — KEYDOWN+KEYUP is the documented fallback from AHK; no official Win32 docs state which is required on Win11

**Research date:** 2026-03-03
**Valid until:** 2026-06-01 (Win32 hook APIs stable; VirusTotal process stable; expires if Windows 11 receives major Start menu updates)
