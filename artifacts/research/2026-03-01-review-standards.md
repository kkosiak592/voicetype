# Standards & Conventions Review

**Date:** 2026-03-01
**Scope:** All source files in `src/` (TypeScript/React) and `src-tauri/src/` (Rust)

---

## Identified Conventions

### TypeScript/React

1. **Component exports**: Named exports (`export function ComponentName`), one component per file. Default export only for `App.tsx`.
2. **File naming**: PascalCase for components (`FrequencyBars.tsx`, `ModelSelector.tsx`), camelCase for non-component modules (`store.ts`). Section-level components live in `components/sections/`.
3. **Import ordering**: React hooks first, then `@tauri-apps` APIs, then local imports. No blank line separators between groups.
4. **Props interfaces**: Defined inline in each file above the component, named `{ComponentName}Props`.
5. **State persistence pattern**: Components call `getStore()` and `store.set()` inline, then call parent `onChange` callback. Store writes are co-located with `invoke()` calls.
6. **CSS class joining**: `.join(' ')` on string arrays rather than template literals or a classnames utility. Consistent across all files.
7. **Async pattern in handlers**: `async function handleX()` declared inside the component body. No `useCallback` wrapping.
8. **Entry points**: Two React roots (`main.tsx` for settings, `pill-main.tsx` for pill overlay), each with `React.StrictMode`.
9. **String quoting**: `App.tsx` and `store.ts` use single quotes for imports. `main.tsx`, `pill-main.tsx`, `Pill.tsx`, `FrequencyBars.tsx` use double quotes.

### Rust

1. **Module organization**: One concern per file (`audio.rs`, `pipeline.rs`, `vad.rs`, etc.), all declared in `lib.rs`.
2. **Tauri managed state**: Newtype wrappers (`PipelineState(AtomicU8)`, `ActiveProfile(Mutex<Profile>)`, `AudioCaptureMutex(Mutex<AudioCapture>)`). Consistent pattern across all state types.
3. **Error handling in commands**: `.map_err(|e| e.to_string())?` for converting errors to Tauri-compatible `Result<_, String>`.
4. **Logging**: `log::info!`, `log::warn!`, `log::error!` used consistently throughout. No `println!` or `eprintln!`.
5. **Feature gating**: `#[cfg(feature = "whisper")]` gates all whisper-dependent code. Consistent application.
6. **Settings persistence**: Manual JSON read/write via `serde_json::Value` manipulation. Repeated `read_saved_*` pattern in `lib.rs`.
7. **Pipeline state machine**: AtomicU8 with constants `IDLE`, `RECORDING`, `PROCESSING` and CAS transitions.
8. **Doc comments**: `///` doc comments on all public functions and types. Internal functions have inline `//` comments.

---

## Deviations and Issues

### D-01: Inconsistent string quote style (TypeScript)

**Severity:** Low (cosmetic)

Some files use single quotes, others use double quotes for string literals and imports.

- Single quotes: `src/App.tsx`, `src/lib/store.ts`, all files in `src/components/` (except below)
- Double quotes: `src/main.tsx:1`, `src/pill-main.tsx:1`, `src/Pill.tsx:1`, `src/components/FrequencyBars.tsx:1`, `src/components/ProcessingDots.tsx`

The split correlates with the two React roots: pill-related files use double quotes, settings-related files use single quotes. A consistent style should be enforced via an ESLint or Prettier config.

### D-02: Duplicate `DownloadEvent` type definition (TypeScript)

**Severity:** Medium (maintainability)

The `DownloadEvent` discriminated union type is defined identically in two files:

- `src/components/FirstRun.tsx:4-8`
- `src/components/ModelSelector.tsx:12-16`

Both files also duplicate `formatMB()`:

- `src/components/FirstRun.tsx:35-37`
- `src/components/ModelSelector.tsx:26-28`

These should be extracted to a shared module (e.g., `src/lib/download.ts`) to avoid drift.

### D-03: Duplicate `ProfileInfo` interface (TypeScript vs Rust)

**Severity:** Low (cross-boundary)

The `ProfileInfo` type is defined differently between frontend and backend:

- **Rust** `lib.rs:402-407`: fields are `id`, `name`, `is_active`
- **TypeScript** `ProfileSwitcher.tsx:3-7`: fields are `id`, `name`, `active`

The Rust struct serializes `is_active` (snake_case) without `#[serde(rename)]` or `#[serde(rename_all = "camelCase")]`, so the JSON key is `is_active`. But the TypeScript interface expects `active`. This is either a field name mismatch (bug) or there is implicit remapping happening. The Rust `ProfileInfo` struct at `lib.rs:402` lacks `#[serde(rename_all = "camelCase")]` unlike `FirstRunStatus` at `lib.rs:708` which has it.

### D-04: Inconsistent `FirstRunStatus` interface location (TypeScript)

**Severity:** Low

`FirstRunStatus` is defined inline in `src/App.tsx:12-16` rather than imported from a shared types module. The same pattern (Rust struct with `#[serde(rename_all = "camelCase")]`) is used for `DownloadEvent` but its TypeScript counterpart is in the component files, not a shared location.

### D-05: Inconsistent serde rename strategy across Rust structs

**Severity:** Medium (correctness risk)

- `FirstRunStatus` at `lib.rs:708` uses `#[serde(rename_all = "camelCase")]` -- correct, frontend expects `needsSetup`, `gpuDetected`, `recommendedModel`.
- `ProfileInfo` at `lib.rs:402` does NOT use `rename_all` -- serializes as `is_active`, but TypeScript expects `active`.
- `DownloadEvent` at `download.rs:16-38` uses manual per-field `#[serde(rename)]` with explicit note about why `rename_all` is insufficient for enum variants.
- `ModelInfo` at `lib.rs:670` does NOT use `rename_all` -- but all field names are already single-word or camelCase-compatible, so no issue.

The strategy is inconsistent: some structs use `rename_all`, some use per-field renames, some use neither. Should pick one approach and apply uniformly.

### D-06: Hotkey handler logic duplicated between `setup()` and `rebind_hotkey()`

**Severity:** High (maintainability / bug surface)

The hotkey handler in `lib.rs` `setup()` (lines ~1068-1192) and `rebind_hotkey()` (lines ~153-276) contain nearly identical ~120-line blocks of hotkey press/release logic for both hold-to-talk and toggle modes. Any bug fix or behavior change must be applied to both locations. This is the largest DRY violation in the codebase.

### D-07: Inconsistent async pattern for blocking work (Rust)

**Severity:** Low

Two different patterns for running blocking work off the async runtime:

1. **`tauri::async_runtime::spawn_blocking`**: Used in `pipeline.rs:117`, `lib.rs:773` (set_model)
2. **`std::thread::spawn` + `std::sync::mpsc`**: Used in `lib.rs:895-900` (transcribe_test_file), `lib.rs:934-938` (force_cpu_transcribe)

The `spawn_blocking` pattern is idiomatic for Tauri/Tokio. The `std::thread::spawn + mpsc` pattern in the test commands is a different convention. Since these are dev/test commands, this is minor, but inconsistent.

### D-08: Settings persistence pattern is not DRY (Rust)

**Severity:** Medium (maintainability)

The "read settings.json, parse, modify, write back" pattern is repeated ~8 times in `lib.rs`:

- `set_recording_mode` (line 121-129)
- `set_active_profile` (line 442-480)
- `save_corrections` (line 519-529)
- `set_all_caps` (line 545-555)
- `set_microphone` (line 625-633)
- `set_model` (line 788-796)

And the `read_saved_*` functions repeat similar JSON parsing boilerplate:

- `read_saved_hotkey` (line 75-86)
- `read_saved_mode` (line 90-110)
- `read_saved_profile_id` (line 331-350)
- `read_saved_corrections` (line 354-377)
- `read_saved_all_caps` (line 381-399)
- `read_saved_mic` (line 563-572)
- `read_saved_model_id` (line 642-651)

A small helper like `read_settings_json(app) -> serde_json::Value` and `write_settings_json(app, json)` would eliminate this repetition.

### D-09: Toggle switch UI component duplicated

**Severity:** Low (cosmetic)

The toggle switch UI (the `<button role="switch">` with `.join(' ')` class styling) is implemented identically in three places:

- `src/components/ThemeToggle.tsx:29-47`
- `src/components/AutostartToggle.tsx:38-56`
- `src/components/sections/ProfilesSection.tsx:90-105`

These could share a `<ToggleSwitch>` primitive component.

### D-10: `sample_count` variable unused except for a suppression comment

**Severity:** Low (dead code)

`pipeline.rs:89`:
```rust
let _ = sample_count; // used for logging above; suppress unused warning
```

The comment says "used for logging above" but `sample_count` is NOT used in any log statement. The variable is extracted at line 49 but never actually logged. The `let _ =` is suppressing an actual unused-variable warning for a value that truly is unused.

### D-11: `isValidating` state never triggered in `FirstRun.tsx`

**Severity:** Low (dead code / unreachable UI)

`FirstRun.tsx:10` defines `'validating'` as a `DownloadState`, and lines 170 and 205-215 check for it, but no code path ever sets `downloadState` to `'validating'`. The `DownloadEvent` from the backend has `started | progress | finished | error` -- no `validating` event. The progress bar "Verifying checksum..." text at line 215 is unreachable.

### D-12: `ModelSelector` has `hasError` logic that is fragile

**Severity:** Low

`ModelSelector.tsx:104`:
```typescript
const hasError = downloadingId === null && downloadError !== null && !model.downloaded;
```

This applies the error state to ALL non-downloaded models when any download fails, not just the specific model that failed. `downloadError` is never cleared between download attempts for different models except by a successful download.

### D-13: Missing error handling in `AutostartToggle`

**Severity:** Low (inconsistency)

`AutostartToggle.tsx:17-29`: The `handleToggle` function calls `enable()`/`disable()` and `store.set()` without try/catch, while similar async handlers in other components (e.g., `HotkeyCapture.tsx:86-93`, `FirstRun.tsx:53-58`) use try/catch.

### D-14: `models_dir()` duplicated across Rust modules

**Severity:** Low

`models_dir()` is defined in both:
- `src-tauri/src/transcribe.rs:16-19`
- `src-tauri/src/download.rs:44-47`

The `download.rs` copy has a comment explaining why: "Duplicated from transcribe::models_dir() to avoid feature-gate coupling". This is a deliberate tradeoff but worth noting as a convention.

### D-15: Frontend `ModelInfo` type defined in two different shapes

**Severity:** Low

- `src/components/ModelSelector.tsx:4-10`: `ModelInfo` has `id, name, description, recommended, downloaded`
- `src/components/FirstRun.tsx:18-33`: `MODELS` constant has `id, name, size, quality, requirement` -- different shape, hardcoded rather than fetched from backend

The FirstRun component uses a hardcoded model list while ModelSection fetches from the backend `list_models` command. These represent the same models but with different metadata schemas. If a model is added or renamed, both must be updated independently.

### D-16: Inconsistent `ProfileInfo.active` field naming

**Severity:** Low

- Rust backend `ProfileSwitcher` expects `active: boolean` (`ProfileSwitcher.tsx:6`)
- Rust backend serializes `is_active: bool` (`lib.rs:406`)

The frontend `ProfileInfo` interface has field `active` but the Rust struct has `is_active`. Without `serde(rename)`, this would serialize as `is_active` in JSON, which the frontend reads as `active`. This looks like a mismatch that would cause `active` to always be `undefined` on the frontend side.

**UPDATE**: On closer look at `ProfilesSection.tsx`, it passes `activeId={activeProfileId}` to `ProfileSwitcher` and the `ProfileSwitcher` component never reads `profile.active` -- it compares `activeId === profile.id` instead. So the `active` field in the TypeScript `ProfileInfo` interface is actually unused/dead.

---

## Summary

| Category | Count | Severity |
|----------|-------|----------|
| DRY violations | 5 (D-02, D-06, D-08, D-09, D-14) | 1 High, 2 Medium, 2 Low |
| Naming/serialization inconsistency | 3 (D-03, D-05, D-16) | 1 Medium, 2 Low |
| Dead/unreachable code | 3 (D-10, D-11, D-16) | Low |
| Style inconsistency | 2 (D-01, D-04) | Low |
| Error handling inconsistency | 2 (D-07, D-13) | Low |
| Data model drift | 1 (D-15) | Low |
| Fragile logic | 1 (D-12) | Low |

**Top priority items:**
1. **D-06**: Extract shared hotkey handler logic from the duplicated setup/rebind blocks in `lib.rs`
2. **D-05 / D-03 / D-16**: Standardize serde rename strategy and verify `ProfileInfo.is_active` serialization actually reaches the frontend correctly (likely a bug)
3. **D-08**: Extract settings JSON read/write helpers to reduce the 15+ copy-pasted JSON manipulation blocks
