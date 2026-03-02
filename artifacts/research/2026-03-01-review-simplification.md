# Codebase Simplification Review

**Date:** 2026-03-01
**Scope:** All frontend (`src/`) and backend (`src-tauri/src/`) source files
**Method:** Read-only static analysis of entire codebase

---

## Executive Summary

The codebase is compact (21 TS/TSX files, 13 Rust files, ~2800 lines total) and generally well-structured. The largest simplification opportunity is eliminating the **massive hotkey handler duplication** in `lib.rs`, which alone accounts for ~120 lines of copy-pasted code. Beyond that, opportunities are mostly moderate: extracting repeated settings.json read/write patterns, deduplicating download state management across two components, and removing dead test commands.

**Estimated total reduction:** ~250-300 lines (20-25% of backend, 5-10% of frontend)

---

## Priority 1 -- High Impact

### 1.1 Hotkey handler fully duplicated between `setup()` and `rebind_hotkey()`

**File:** `src-tauri/src/lib.rs`
**Lines:** 153-277 (rebind_hotkey) and 1068-1190 (setup)
**Severity:** Critical duplication

The hotkey shortcut handler closure is copy-pasted verbatim -- 120 lines of identical logic for press/release across hold-to-talk and toggle modes. Any bug fix or behavior change must be applied in both places.

**Recommendation:** Extract the handler body into a standalone function:

```rust
// lib.rs -- new function
fn handle_shortcut(app: &tauri::AppHandle, event: &ShortcutEvent) {
    let pipeline = app.state::<pipeline::PipelineState>();
    match event.state {
        ShortcutState::Pressed => { /* ... existing logic ... */ }
        ShortcutState::Released => { /* ... existing logic ... */ }
    }
}
```

Then both `setup()` and `rebind_hotkey()` call `handle_shortcut(app, event)` inside their closure. One source of truth.

**Estimated reduction:** ~120 lines removed

---

### 1.2 Repeated settings.json read/write boilerplate

**File:** `src-tauri/src/lib.rs`
**Lines:** 75-110, 121-132, 331-399, 441-445, 519-529, 545-555, 625-633, 788-796

Eight separate functions all follow the same pattern:
1. Get `app_data_dir`
2. Build `settings.json` path
3. Read + parse JSON (or return default)
4. Access/modify a key
5. Write back with `to_string_pretty`

There is also an identical "read settings, parse JSON, early-return on failure" pattern in `read_saved_hotkey`, `read_saved_mode`, `read_saved_profile_id`, `read_saved_corrections`, `read_saved_all_caps`, and `read_saved_mic` (6 functions).

**Recommendation:** Extract two helpers:

```rust
fn read_settings(app: &impl Manager) -> serde_json::Value {
    let data_dir = app.path().app_data_dir().ok();
    // ... shared read + parse logic, returns json!({}) on any failure
}

fn write_settings(app: &impl Manager, json: &serde_json::Value) -> Result<(), String> {
    // ... shared write logic
}
```

All 6 `read_saved_*` functions become 1-2 line wrappers. All write paths shrink from 6 lines to 2.

**Estimated reduction:** ~80-100 lines removed

---

### 1.3 `DownloadEvent` type and `formatMB` duplicated across FirstRun.tsx and ModelSelector.tsx

**Files:**
- `src/components/FirstRun.tsx:4-8` -- `DownloadEvent` type
- `src/components/ModelSelector.tsx:12-16` -- identical `DownloadEvent` type
- `src/components/FirstRun.tsx:35-37` -- `formatMB` function
- `src/components/ModelSelector.tsx:26-28` -- identical `formatMB` function

Both components also duplicate the download invocation pattern (`new Channel`, `onmessage` handler with identical switch cases, `invoke('download_model', ...)`).

**Recommendation:** Extract to a shared module:

```typescript
// src/lib/download.ts
export type DownloadEvent = ...;
export function formatMB(bytes: number): string { ... }
export function createDownloadChannel(callbacks: {...}): Channel<DownloadEvent> { ... }
```

**Estimated reduction:** ~30 lines removed, single source of truth for download types

---

### 1.4 `models_dir()` duplicated between `transcribe.rs` and `download.rs`

**Files:**
- `src-tauri/src/transcribe.rs:16-19`
- `src-tauri/src/download.rs:44-47`

Identical function. The comment in `download.rs:42-43` explicitly notes the duplication: "Duplicated from transcribe::models_dir() to avoid feature-gate coupling."

**Recommendation:** Move `models_dir()` to a small shared utility (e.g., `paths.rs`) that has no feature-gate dependency. Both `transcribe.rs` and `download.rs` import from there.

**Estimated reduction:** ~5 lines, but eliminates a maintenance hazard (change one, forget the other)

---

### 1.5 Model metadata duplicated between `download.rs::model_info()` and `lib.rs::model_id_to_path()` / `list_models()` / `check_first_run()`

**Files:**
- `src-tauri/src/download.rs:51-65` -- `model_info()` maps model_id to (filename, sha256, size)
- `src-tauri/src/lib.rs:657-665` -- `model_id_to_path()` maps model_id to filename (duplicated mapping)
- `src-tauri/src/lib.rs:681-702` -- `list_models()` hardcodes filenames again
- `src-tauri/src/lib.rs:721-736` -- `check_first_run()` hardcodes filenames again
- `src-tauri/src/transcribe.rs:57-66` -- `resolve_model_path()` maps ModelMode to filename (yet another place)

The model_id-to-filename mapping exists in **5 separate places**. Adding a new model requires changes in 5 files/functions.

**Recommendation:** Create a single `ModelCatalog` struct or const array:

```rust
// models.rs (new, no feature gate needed)
pub struct ModelEntry {
    pub id: &'static str,
    pub filename: &'static str,
    pub sha256: &'static str,
    pub size_bytes: u64,
    pub name: &'static str,
    pub description: &'static str,
    pub requires_gpu: bool,
}

pub const MODELS: &[ModelEntry] = &[ /* ... */ ];

pub fn models_dir() -> PathBuf { /* ... */ }
pub fn find_model(id: &str) -> Option<&'static ModelEntry> { /* ... */ }
```

**Estimated reduction:** ~40 lines, and adding a model becomes a single-line addition

---

## Priority 2 -- Moderate Impact

### 2.1 Toggle switch component pattern duplicated 3 times

**Files:**
- `src/components/AutostartToggle.tsx:37-56` -- toggle switch markup
- `src/components/ThemeToggle.tsx:28-47` -- identical toggle switch markup
- `src/components/sections/ProfilesSection.tsx:90-105` -- inline toggle switch (ALL CAPS)

All three render the same `<button role="switch">` with identical sizing, transition, and knob markup. Only the state variable and colors differ.

**Recommendation:** Extract a reusable `Toggle` component:

```tsx
interface ToggleProps {
  checked: boolean;
  onChange: () => void;
}

export function Toggle({ checked, onChange }: ToggleProps) {
  return (
    <button onClick={onChange} role="switch" aria-checked={checked} className={...}>
      <span className={...} />
    </button>
  );
}
```

Then `AutostartToggle`, `ThemeToggle`, and the ALL CAPS toggle all use `<Toggle checked={...} onChange={...} />`.

**Estimated reduction:** ~30 lines, consistent toggle behavior everywhere

---

### 2.2 Card selection pattern duplicated between `ProfileSwitcher` and `RecordingModeToggle`

**Files:**
- `src/components/ProfileSwitcher.tsx:27-61`
- `src/components/RecordingModeToggle.tsx:26-73`

These two components are structurally identical: a `flex gap-3` container with clickable cards that show a name, description, and selected/unselected border styling. The only differences are data shape and the `onSelect` handler.

**Recommendation:** Extract a generic `CardSelect<T>` component or at minimum extract the card styling logic. This is a lower priority because the duplication is in presentation (Tailwind classes), not logic.

**Estimated reduction:** ~20 lines

---

### 2.3 `start_persistent_stream_with_device()` is a trivial wrapper

**File:** `src-tauri/src/audio.rs:202-206`

```rust
pub fn start_persistent_stream_with_device(
    device: cpal::Device,
) -> Result<AudioCapture, Box<dyn std::error::Error + Send + Sync>> {
    build_stream_from_device(device)
}
```

This function is a one-line pass-through to `build_stream_from_device`. It exists alongside `start_persistent_stream()` which adds default device lookup. Both are public.

**Recommendation:** Make `build_stream_from_device` public and rename it to `start_stream` or similar. Inline the default-device lookup into the one caller that needs it (`start_persistent_stream` or the caller in `lib.rs`). Remove the wrapper.

**Estimated reduction:** ~10 lines, one less indirection layer

---

### 2.4 `transcribe_test_file` and `force_cpu_transcribe` use `std::thread::spawn` + `mpsc::channel` instead of `spawn_blocking`

**File:** `src-tauri/src/lib.rs:868-952`

These test commands use the manual pattern `std::thread::spawn + mpsc::channel` for blocking work, while `pipeline.rs:117` and `set_model` (line 773) correctly use `tauri::async_runtime::spawn_blocking`. The manual thread + channel approach is more verbose and accomplishes the same thing.

**Recommendation:** Migrate to `spawn_blocking`:

```rust
// Before (8 lines):
let (tx, rx) = std::sync::mpsc::channel();
std::thread::spawn(move || {
    let _ = tx.send(transcribe::transcribe_audio(&ctx, &audio_f32, ""));
});
let result = rx.recv()
    .map_err(|e| format!("Inference thread failed: {}", e))?
    .map_err(|e| e.to_string())?;

// After (4 lines):
let result = tauri::async_runtime::spawn_blocking(move || {
    transcribe::transcribe_audio(&ctx, &audio_f32, "")
}).await
.map_err(|e| format!("Inference thread failed: {}", e))??;
```

**Estimated reduction:** ~12 lines across both commands

---

### 2.5 Consider removing `force_cpu_transcribe` entirely

**File:** `src-tauri/src/lib.rs:908-952`

The doc comment says: "Phase 2 verification command -- will be removed or hidden in later phases." The codebase is now at Phase 7 (distribution). This is dead test scaffolding.

Similarly, `transcribe_test_file` (lines 866-906) and `save_test_wav` (lines 308-327) are development-only test commands that are registered in the invoke handler for production builds.

**Recommendation:** Gate behind `#[cfg(debug_assertions)]` or remove. Also remove `start_recording` and `stop_recording` commands (lines 286-303) if they are only used for manual testing -- the hotkey handler manages recording state directly.

**Estimated reduction:** ~80 lines if removed; ~0 lines if gated (but cleaner invoke handler)

---

### 2.6 `Pill.tsx` exit animation logic duplicated between `pill-hide` and `pill-result` handlers

**File:** `src/Pill.tsx:38-48` and `66-74`

```typescript
// pill-hide handler (lines 39-48)
setAnimState("exiting");
exitTimerRef.current = setTimeout(() => {
    appWindow.hide();
    setAnimState("hidden");
    setDisplayState("hidden");
    exitTimerRef.current = null;
}, 200);

// pill-result handler (lines 67-73) -- identical
setAnimState("exiting");
exitTimerRef.current = setTimeout(() => {
    appWindow.hide();
    setAnimState("hidden");
    setDisplayState("hidden");
    exitTimerRef.current = null;
}, 200);
```

**Recommendation:** Extract to a local function:

```typescript
function startExitAnimation() {
    clearAllTimers();
    setAnimState("exiting");
    exitTimerRef.current = setTimeout(() => {
        appWindow.hide();
        setAnimState("hidden");
        setDisplayState("hidden");
        exitTimerRef.current = null;
    }, 200);
}
```

**Estimated reduction:** ~8 lines

---

## Priority 3 -- Low Impact / Nitpicks

### 3.1 `pipeline.rs:89` -- dead variable

```rust
let _ = sample_count; // used for logging above; suppress unused warning
```

`sample_count` is computed on line 53 but never actually used -- the logging on lines 84-88 uses `samples.len()`, not `sample_count`. The `let _ = sample_count` suppression is misleading.

**Recommendation:** Remove `sample_count` from the destructure on line 49, change to `let (_, samples) = { ... }` or just fetch `samples` directly.

**Estimated reduction:** 2 lines

### 3.2 `pipeline.rs:157` -- redundant double-check on trimmed string

```rust
let trimmed = transcription.trim_start();
if trimmed.is_empty() || trimmed.chars().all(|c| c.is_whitespace()) {
```

After `trim_start()`, if `trimmed` is non-empty, it is guaranteed to start with a non-whitespace character. However, it could still contain only trailing whitespace (e.g., `" \n"` becomes `"\n"`). A simpler formulation: `if trimmed.trim().is_empty()`.

**Estimated reduction:** Negligible lines, improved clarity

### 3.3 `App.tsx:62-64` -- verbose null check

```typescript
if (savedSelectedModel !== null && savedSelectedModel !== undefined) {
    setSelectedModel(savedSelectedModel);
}
```

Can be simplified to `if (savedSelectedModel != null)` (loose equality covers both null and undefined), matching the pattern used for other settings on lines 47-60.

**Estimated reduction:** 1 line

### 3.4 `cancel_stale_vad_worker` -- overly cautious pattern

**File:** `src-tauri/src/pipeline.rs:222-238`

```rust
let maybe_handle: Option<crate::vad::VadWorkerHandle> = {
    let vad_state = app.state::<crate::VadWorkerState>();
    let result = match vad_state.0.lock() {
        Ok(mut guard) => guard.take(),
        Err(_) => None,
    };
    result
};
```

The `let result = ...; result` pattern with its explanatory comment about E0597 is a workaround for MutexGuard lifetime. Simpler: `vad_state.0.lock().ok().and_then(|mut g| g.take())`.

**Estimated reduction:** 5 lines

### 3.5 `ModelSelector.tsx:110` -- clickable div should be a button

The model row uses a `<div>` with `role="button"`, `tabIndex`, and `onKeyDown` -- this is a manual reimplementation of `<button>` behavior. Using a `<button>` would eliminate 3 lines of ARIA/keyboard handling.

**Estimated reduction:** 3 lines, better accessibility

### 3.6 `ProfilesSection.tsx:21` -- redundant profile sync on mount

```typescript
// Sync backend with frontend's active profile before reading corrections
await invoke('set_active_profile', { profileId: activeProfileId });
```

This re-sends `set_active_profile` on every `activeProfileId` change, but `handleProfileSelect` on line 33 already calls `set_active_profile`. The mount effect fires on initial render, which duplicates the work done at app startup in `lib.rs` setup. Consider removing the sync call and loading corrections directly.

**Estimated reduction:** 1 line, eliminates redundant IPC call

### 3.7 `lib.rs` -- `ProfileInfo` struct naming mismatch

Backend `ProfileInfo` has `is_active: bool` (line 406) but frontend `ProfileInfo` from `ProfileSwitcher.tsx` has `active: bool` (line 7). The `is_active` vs `active` naming difference may be a silent deserialization issue or indicates dead data. Worth aligning.

### 3.8 `list_models()` calls `detect_gpu()` on every invocation

**File:** `src-tauri/src/lib.rs:683`

`detect_gpu()` initializes NVML on each call. This is a lightweight operation but could be cached once at startup as managed state.

---

## Summary Table

| # | Description | Files | Est. Lines Saved | Priority |
|---|------------|-------|------------------|----------|
| 1.1 | Extract hotkey handler to shared function | lib.rs | ~120 | P1 |
| 1.2 | Extract settings.json read/write helpers | lib.rs | ~80-100 | P1 |
| 1.3 | Deduplicate DownloadEvent + formatMB | FirstRun.tsx, ModelSelector.tsx | ~30 | P1 |
| 1.4 | Deduplicate models_dir() | transcribe.rs, download.rs | ~5 | P1 |
| 1.5 | Centralize model catalog metadata | lib.rs, download.rs, transcribe.rs | ~40 | P1 |
| 2.1 | Extract Toggle component | AutostartToggle, ThemeToggle, ProfilesSection | ~30 | P2 |
| 2.2 | Extract CardSelect pattern | ProfileSwitcher, RecordingModeToggle | ~20 | P2 |
| 2.3 | Remove trivial stream wrapper | audio.rs | ~10 | P2 |
| 2.4 | Use spawn_blocking in test commands | lib.rs | ~12 | P2 |
| 2.5 | Remove/gate Phase 2 test commands | lib.rs | ~80 | P2 |
| 2.6 | Extract pill exit animation | Pill.tsx | ~8 | P2 |
| 3.1 | Remove dead sample_count variable | pipeline.rs | 2 | P3 |
| 3.2 | Simplify trimmed whitespace check | pipeline.rs | 1 | P3 |
| 3.3 | Simplify null check | App.tsx | 1 | P3 |
| 3.4 | Simplify cancel_stale_vad_worker | pipeline.rs | 5 | P3 |
| 3.5 | Use button instead of div role=button | ModelSelector.tsx | 3 | P3 |
| 3.6 | Remove redundant profile sync | ProfilesSection.tsx | 1 | P3 |

**Total estimated reduction: ~250-300 lines** across 10+ files, primarily concentrated in `lib.rs`.

---

## Files NOT Needing Simplification

These files are clean, focused, and appropriately sized:

- `src/main.tsx` / `src/pill-main.tsx` -- minimal entrypoints
- `src/lib/store.ts` -- clean singleton pattern
- `src/components/Sidebar.tsx` -- simple, data-driven
- `src/components/ProcessingDots.tsx` -- pure presentation
- `src/components/DictionaryEditor.tsx` -- well-structured CRUD
- `src/components/HotkeyCapture.tsx` -- clear keyboard normalization logic
- `src/components/FrequencyBars.tsx` -- imperative DOM for perf (justified)
- `src/components/sections/GeneralSection.tsx` -- thin composition layer
- `src/components/sections/AppearanceSection.tsx` -- thin composition layer
- `src/components/sections/MicrophoneSection.tsx` -- clean and focused
- `src/components/sections/ModelSection.tsx` -- clean orchestration
- `src-tauri/src/main.rs` -- 6 lines, nothing to simplify
- `src-tauri/src/corrections.rs` -- clean regex engine
- `src-tauri/src/corrections_tests.rs` -- appropriate test coverage
- `src-tauri/src/inject.rs` -- well-documented sequence
- `src-tauri/src/tray.rs` -- focused tray management
- `src-tauri/src/profiles.rs` -- clean profile definitions
- `src-tauri/src/vad.rs` -- complex but necessary streaming logic
- `src-tauri/src/download.rs` -- clean streaming download with validation

---

## Recommended Implementation Order

1. **1.1** (hotkey dedup) -- highest ROI, eliminates critical maintenance hazard
2. **1.5 + 1.4** (model catalog + models_dir) -- eliminates 5-way duplication
3. **1.2** (settings helpers) -- reduces lib.rs by ~100 lines
4. **2.5** (remove test commands) -- dead code removal, simplifies invoke handler
5. **1.3** (download types) -- frontend type safety
6. **2.1** (Toggle component) -- UI consistency
7. **2.6 + 3.x** (remaining small items) -- polish pass
