---
status: awaiting_human_verify
trigger: "parakeet-transcription-silent"
created: 2026-03-01T00:00:00Z
updated: 2026-03-01T00:02:00Z
---

## Current Focus

hypothesis: CONFIRMED - "parakeet" feature is NOT in the default features list in Cargo.toml. The binary is compiled without the "parakeet" feature, so the `#[cfg(not(feature = "parakeet"))]` fallback branch in pipeline.rs executes: it logs a warning, emits "error" to the pill, and returns without transcribing. The pill shows then disappears - matching the symptom exactly.
test: Confirmed by reading Cargo.toml line 16: `default = ["whisper"]` - "parakeet" is absent from defaults.
expecting: Fix is to add "parakeet" to the default features in Cargo.toml and rebuild.
next_action: Verify the fallback branch in pipeline.rs matches the symptom, then fix Cargo.toml

## Symptoms

expected: Press and hold shortcut, speak, release — transcription text appears at cursor (same as Whisper behavior)
actual: Pill appears on key press, disappears on release, but no text is typed anywhere. Silent failure.
errors: No visible error messages in the pill UI. Need to check Tauri backend logs.
reproduction: Select Parakeet TDT engine in settings, press the transcription shortcut, speak, release.
started: After switching to Parakeet engine. Whisper worked before switching.

## Eliminated

(none yet)

## Evidence

- timestamp: 2026-03-01T00:01:00Z
  checked: Cargo.toml [features] section
  found: `default = ["whisper"]` — "parakeet" is NOT in the default features. The "parakeet" feature is defined but excluded from default.
  implication: Any binary built with `cargo build` or `tauri build` without explicit `--features parakeet` will NOT compile the parakeet code path.

- timestamp: 2026-03-01T00:01:01Z
  checked: pipeline.rs lines 243-251 — the Parakeet engine branch
  found: `#[cfg(not(feature = "parakeet"))]` fallback block: logs "Pipeline: parakeet feature not enabled, engine set to parakeet — falling back", emits "pill-result error", resets to idle, returns.
  implication: When compiled without the parakeet feature, selecting Parakeet engine causes an immediate silent failure path: pill shows (from key press), then "error" event triggers pill hide (matching "disappears on release" symptom), no text injected.

- timestamp: 2026-03-01T00:01:02Z
  checked: lib.rs run() function — ParakeetStateMutex registration
  found: `#[cfg(feature = "parakeet")] { builder = builder.manage(ParakeetStateMutex(...)); }` — managed state is also conditionally compiled. Also, the startup model load is `#[cfg(feature = "parakeet")]` gated.
  implication: Without the feature, no Parakeet state exists at all. The entire parakeet code path is compiled out.

- timestamp: 2026-03-01T00:01:03Z
  checked: Cargo.toml comment on parakeet feature (line 27)
  found: Comment reads "NOTE: parakeet is NOT in default features — wire into pipeline in Plan 02." This is an explicit TODO left in the code indicating the parakeet feature was intentionally excluded from default features as an incomplete implementation step.
  implication: This is not a latent bug but an unfinished feature integration. The feature flag never got added to defaults after the pipeline wiring was completed.

## Resolution

root_cause: "parakeet" feature was absent from Cargo.toml default features (`default = ["whisper"]`). The binary compiled without the parakeet code path. In pipeline.rs, the `#[cfg(not(feature = "parakeet"))]` fallback branch executed on every transcription attempt: it emitted "error" to the pill and returned without transcribing. This caused the pill to flash and disappear with no text output — matching the exact symptom.

fix: Added "parakeet" to default features in src-tauri/Cargo.toml: `default = ["whisper", "parakeet"]`. Also removed the stale Plan 02 TODO comment since the pipeline wiring was already complete.

verification: Requires rebuild and manual test (parakeet model must be downloaded, select Parakeet engine, press shortcut, speak, release — expect transcription text to appear).

files_changed:
  - src-tauri/Cargo.toml (default features, stale comment)
