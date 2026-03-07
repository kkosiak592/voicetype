# Phase 24: Pipeline Override Integration - Research

**Researched:** 2026-03-07
**Domain:** Pipeline text processing, per-app override resolution, Rust managed state access
**Confidence:** HIGH

## Summary

This phase has a narrow, well-defined scope: modify the ALL CAPS block in `pipeline.rs` (lines 395-404) to consult per-app overrides before applying case transformation. All required infrastructure already exists from Phase 23: `foreground::detect_foreground_app()` returns the current app's exe name, and `foreground::AppRulesState` holds the per-app rules in a Mutex-wrapped HashMap.

The change is approximately 15-25 lines of code. The current ALL CAPS block reads `ActiveProfile.all_caps` (a simple bool). The new logic must: (1) call `detect_foreground_app()` to get the exe name, (2) look up the exe name in `AppRulesState`, (3) if a matching rule has `all_caps: Some(true)` force uppercase, if `Some(false)` force normal case, if `None` or no rule found fall back to the profile's `all_caps` bool.

There are no new dependencies, no new files, no new Tauri commands. This is a single-file modification to `pipeline.rs` with a unit-testable resolution function.

**Primary recommendation:** Add a `resolve_all_caps()` helper function (either in `foreground.rs` or inline in `pipeline.rs`) that takes the profile's `all_caps` bool and the detected exe name, looks up the override, and returns the effective bool. Call this from the existing ALL CAPS block in `run_pipeline`.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| OVR-02 | Per-app override is applied automatically at injection time when foreground app matches a rule | `detect_foreground_app()` is called at pipeline line ~395 (before ALL CAPS), exe name looked up in `AppRulesState`. `Some(true)` forces uppercase, `Some(false)` forces normal case. |
| OVR-03 | Unlisted apps fall back to the global ALL CAPS toggle on the General page | When no rule exists for the detected exe (or exe_name is None), the existing `ActiveProfile.all_caps` bool is used unchanged. |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tauri` | 2.x | Managed state access (`AppRulesState`, `ActiveProfile`) | Already in use |

### Supporting
No new dependencies. All required types and functions exist from Phase 23.

### Alternatives Considered
None -- the implementation path is fully determined by existing architecture.

## Architecture Patterns

### Integration Point

The single modification site in `pipeline.rs`:

```rust
// CURRENT (lines 395-404):
let formatted = {
    let profile = app.state::<crate::profiles::ActiveProfile>();
    let guard = profile.0.lock().unwrap_or_else(|e| e.into_inner());
    if guard.all_caps {
        corrected.to_uppercase()
    } else {
        corrected
    }
};
```

### Pattern 1: Override Resolution Function

**What:** Pure function that resolves the effective ALL CAPS setting given a profile default and an optional per-app override.
**When to use:** Called from the pipeline's ALL CAPS block.

```rust
// In foreground.rs (keeps override logic co-located with override types)
/// Resolve whether ALL CAPS should be applied, considering per-app overrides.
///
/// Priority: per-app rule > profile default.
/// - AppRule.all_caps = Some(true)  -> force ON regardless of profile
/// - AppRule.all_caps = Some(false) -> force OFF regardless of profile
/// - AppRule.all_caps = None        -> use profile default
/// - No rule for this app           -> use profile default
/// - exe_name is None (detection failed) -> use profile default
pub fn resolve_all_caps(
    profile_all_caps: bool,
    exe_name: &Option<String>,
    rules: &std::collections::HashMap<String, AppRule>,
) -> bool {
    if let Some(name) = exe_name {
        if let Some(rule) = rules.get(name) {
            if let Some(override_val) = rule.all_caps {
                return override_val;
            }
        }
    }
    profile_all_caps
}
```

### Pattern 2: Pipeline Integration

**What:** Replace the current ALL CAPS block with detection + resolution.
**When to use:** Replacing lines 395-404 of pipeline.rs.

```rust
// NEW pipeline ALL CAPS block:
let formatted = {
    // Detect foreground app for per-app override lookup
    #[cfg(windows)]
    let detected_exe = crate::foreground::detect_foreground_app().exe_name;
    #[cfg(not(windows))]
    let detected_exe: Option<String> = None;

    let profile_all_caps = {
        let profile = app.state::<crate::profiles::ActiveProfile>();
        let guard = profile.0.lock().unwrap_or_else(|e| e.into_inner());
        guard.all_caps
    };

    #[cfg(windows)]
    let effective_all_caps = {
        let rules_state = app.state::<crate::foreground::AppRulesState>();
        let rules_guard = rules_state.0.lock().unwrap_or_else(|e| e.into_inner());
        crate::foreground::resolve_all_caps(profile_all_caps, &detected_exe, &rules_guard)
    };
    #[cfg(not(windows))]
    let effective_all_caps = profile_all_caps;

    if effective_all_caps {
        corrected.to_uppercase()
    } else {
        corrected
    }
};
```

### Anti-Patterns to Avoid
- **Calling detect_foreground_app() earlier in the pipeline:** The decision says detection happens "at text injection time" (line ~395), not at recording start. The foreground app may change between recording start and injection.
- **Holding both Mutex locks simultaneously:** Lock `ActiveProfile` first, extract the bool, drop the guard. Then lock `AppRulesState`. Never hold both at once -- avoids any deadlock risk.
- **Moving detect_foreground_app into inject.rs:** The decision from STATE.md is explicit: "Detection at pipeline.rs line 395 (before ALL CAPS application), not in inject.rs."

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Foreground detection | New Win32 calls | `foreground::detect_foreground_app()` | Already implemented in Phase 23 |
| Override lookup | Inline matching in pipeline | `resolve_all_caps()` helper | Testable in isolation, reusable |
| Case normalization | Manual lowercasing of exe name | Already done in `detect_foreground_app()` and `set_app_rule` | Both boundaries already normalize |

## Common Pitfalls

### Pitfall 1: Forgetting cfg(windows) Gating
**What goes wrong:** `foreground` module is `#[cfg(windows)]` only. Referencing it without gating causes compilation failure on non-Windows targets.
**Why it happens:** Pipeline.rs currently has no platform-specific code.
**How to avoid:** Wrap all `foreground::` references in `#[cfg(windows)]` blocks. Provide a `#[cfg(not(windows))]` fallback that uses the profile default.
**Warning signs:** `cargo check --target x86_64-unknown-linux-gnu` fails (if cross-compilation is tested).

### Pitfall 2: Detection Timing
**What goes wrong:** Calling detection at recording start captures the wrong app (user switches apps during dictation).
**Why it happens:** Natural delay between starting and finishing dictation.
**How to avoid:** Call `detect_foreground_app()` at line ~395 (just before ALL CAPS), as specified in the locked decision. This captures the app the user is actually typing into.
**Warning signs:** Override applies to the wrong app.

### Pitfall 3: Lock Ordering
**What goes wrong:** Holding ActiveProfile lock while acquiring AppRulesState lock (or vice versa) risks deadlock if another code path acquires them in reverse order.
**Why it happens:** Both are needed for the resolution logic.
**How to avoid:** Extract the `all_caps` bool from ActiveProfile, drop the guard, then acquire AppRulesState. Sequential, never nested.
**Warning signs:** App freezes during dictation (deadlock).

### Pitfall 4: Performance Regression
**What goes wrong:** detect_foreground_app() calls 3 Win32 APIs + potentially UWP resolution, adding latency to the pipeline.
**Why it happens:** Win32 calls are fast (microseconds) but if something blocks (e.g., OpenProcess on a hung process), it could stall.
**How to avoid:** The current implementation already returns None on failure rather than blocking. Three Win32 calls take <1ms total. No performance concern.
**Warning signs:** Pipeline latency increases noticeably (measure with logging timestamps if concerned).

## Code Examples

### resolve_all_caps with Unit Tests
```rust
// Source: derived from existing AppRule type and project conventions

#[cfg(test)]
mod override_tests {
    use super::*;

    #[test]
    fn test_no_rules_uses_profile_default_on() {
        let rules = HashMap::new();
        assert!(resolve_all_caps(true, &Some("notepad.exe".into()), &rules));
    }

    #[test]
    fn test_no_rules_uses_profile_default_off() {
        let rules = HashMap::new();
        assert!(!resolve_all_caps(false, &Some("notepad.exe".into()), &rules));
    }

    #[test]
    fn test_force_on_overrides_profile_off() {
        let mut rules = HashMap::new();
        rules.insert("notepad.exe".into(), AppRule { all_caps: Some(true) });
        assert!(resolve_all_caps(false, &Some("notepad.exe".into()), &rules));
    }

    #[test]
    fn test_force_off_overrides_profile_on() {
        let mut rules = HashMap::new();
        rules.insert("notepad.exe".into(), AppRule { all_caps: Some(false) });
        assert!(!resolve_all_caps(true, &Some("notepad.exe".into()), &rules));
    }

    #[test]
    fn test_inherit_uses_profile() {
        let mut rules = HashMap::new();
        rules.insert("notepad.exe".into(), AppRule { all_caps: None });
        assert!(resolve_all_caps(true, &Some("notepad.exe".into()), &rules));
        assert!(!resolve_all_caps(false, &Some("notepad.exe".into()), &rules));
    }

    #[test]
    fn test_detection_failed_uses_profile() {
        let mut rules = HashMap::new();
        rules.insert("notepad.exe".into(), AppRule { all_caps: Some(true) });
        // exe_name is None -> falls back to profile regardless of rules
        assert!(!resolve_all_caps(false, &None, &rules));
    }

    #[test]
    fn test_unlisted_app_uses_profile() {
        let mut rules = HashMap::new();
        rules.insert("notepad.exe".into(), AppRule { all_caps: Some(true) });
        // Different app, no rule -> falls back to profile
        assert!(!resolve_all_caps(false, &Some("code.exe".into()), &rules));
    }
}
```

## State of the Art

No changes from Phase 23. All APIs and patterns remain current.

## Open Questions

None. The integration path is fully determined by:
1. Locked decision: "Detection at pipeline.rs line 395"
2. Existing `detect_foreground_app()` function from Phase 23
3. Existing `AppRulesState` managed state from Phase 23
4. Existing `AppRule.all_caps: Option<bool>` three-state model from Phase 23

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[cfg(test)]` + `cargo test` |
| Config file | None (standard Cargo test runner) |
| Quick run command | `cd src-tauri && cargo test --lib -- foreground` |
| Full suite command | `cd src-tauri && cargo test --lib` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| OVR-02 | Per-app override Force ON/OFF takes effect | unit | `cd src-tauri && cargo test --lib -- foreground::override_tests` | Wave 0 |
| OVR-03 | Unlisted app falls back to global setting | unit | `cd src-tauri && cargo test --lib -- foreground::override_tests::test_unlisted_app` | Wave 0 |
| OVR-02 | End-to-end: dictate into app with Force ON rule, get ALL CAPS | manual-only | N/A -- requires running app with foreground detection | N/A |
| OVR-03 | End-to-end: dictate into unlisted app, uses global toggle | manual-only | N/A -- requires running app with foreground detection | N/A |

**Note:** The `resolve_all_caps()` function is fully unit-testable. The end-to-end behavior (detection + resolution + pipeline) requires a running desktop environment and is verified manually.

### Sampling Rate
- **Per task commit:** `cd src-tauri && cargo test --lib -- foreground`
- **Per wave merge:** `cd src-tauri && cargo test --lib`
- **Phase gate:** Full suite green before verification

### Wave 0 Gaps
- [ ] `src-tauri/src/foreground.rs` -- needs `resolve_all_caps()` function and `override_tests` test module
- [ ] Framework install: None needed

## Sources

### Primary (HIGH confidence)
- Existing codebase: `pipeline.rs` lines 395-404 (current ALL CAPS block) -- the exact integration point
- Existing codebase: `foreground.rs` (detect_foreground_app, AppRule, AppRulesState) -- Phase 23 output
- Existing codebase: `lib.rs` lines 1144-1189 (Tauri command registration, AppRulesState managed state)
- Existing codebase: `profiles.rs` (ActiveProfile, Profile.all_caps) -- the global default source
- STATE.md decisions: "Detection at pipeline.rs line 395", "AppOverrides as separate managed state", "Option<bool> three-state toggle"

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - no new dependencies, all infrastructure exists
- Architecture: HIGH - single integration point clearly identified, all types already defined
- Pitfalls: HIGH - straightforward Mutex access pattern already established in codebase

**Research date:** 2026-03-07
**Valid until:** 2026-04-07 (stable domain -- internal architecture change only)
