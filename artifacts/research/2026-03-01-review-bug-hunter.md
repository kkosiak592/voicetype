# Bug Hunter Review - Voice-to-Text Codebase

**Date:** 2026-03-01
**Scope:** All source files in `src/` (React/TypeScript) and `src-tauri/src/` (Rust)
**Files reviewed:** 13 Rust files, 21 TypeScript files

| Severity | Count |
|----------|-------|
| Critical | 2     |
| High     | 5     |
| Medium   | 9     |
| Low      | 7     |
| **Total**| **23**|

---

## Critical

### BUG-01: Audio capture not registered on mic failure causes runtime panics
**File:** `src-tauri/src/lib.rs:1219-1233`

When `start_persistent_stream()` fails (no microphone, device unavailable), the code logs a warning but does NOT call `app.manage(audio::AudioCaptureMutex(...))`. Every subsequent Tauri command and hotkey handler that accesses `State<AudioCaptureMutex>` will panic because the managed state was never registered. The comment on line 1230 acknowledges this: "commands will panic on missing state."

For a desktop app that starts at login, this means a user who unplugs their mic before launch gets repeated panics on every hotkey press with no recovery path.

**Fix:** Register a dummy/sentinel `AudioCaptureMutex`, or wrap state in `Option` and handle gracefully.

---

### BUG-02: `save_corrections` only adds entries, never removes deleted corrections
**File:** `src-tauri/src/lib.rs:497-533`

The `save_corrections` command receives a full `corrections` map from the frontend `DictionaryEditor`. On line 502:
```rust
guard.corrections.extend(corrections.clone());
```
`HashMap::extend` only adds/updates keys -- it never removes keys absent from the incoming map. When a user deletes a correction entry in the UI, the deleted entry persists in the in-memory profile and continues to be applied to all transcriptions. The entry only disappears after a full app restart.

The persisted JSON at line 527 correctly saves only the user-provided map, creating a split-brain: in-memory has stale entries while disk is correct.

**Fix:** Replace with full merge from profile defaults + new user map, or clear and rebuild:
```rust
guard.corrections = base_profile_corrections.clone();
guard.corrections.extend(corrections.clone());
```

---

## High

### BUG-03: `allCaps` toggle state not loaded from backend on ProfilesSection mount
**File:** `src/components/sections/ProfilesSection.tsx:15,43-47`

`allCaps` state initializes to `false` on line 15. On mount, `loadInitial()` syncs profiles and corrections but never reads the current `all_caps` value from the backend. The toggle always starts OFF regardless of persisted value. If the user previously enabled ALL CAPS, navigates away, and returns, the toggle shows OFF but backend still applies uppercase. Toggling then inverts the user's intent.

**Fix:** Add a `get_all_caps` command or include `all_caps` in the profile data returned by `get_profiles`, and initialize the toggle from it.

---

### BUG-04: `set_model` hardcodes GPU mode by model ID, ignoring actual GPU availability
**File:** `src-tauri/src/lib.rs:766-770`

```rust
let mode = if model_id == "large-v3-turbo" {
    crate::transcribe::ModelMode::Gpu
} else {
    crate::transcribe::ModelMode::Cpu
};
```

If a user without an NVIDIA GPU selects the large model (manual file copy, GPU removed after setup, or driver update broke CUDA), `load_whisper_context` is called with `use_gpu(true)`. Depending on whisper-rs build, this either fails cryptically or silently falls back to CPU with degraded performance. Same hardcoded logic at startup (line 1249).

**Fix:** Use `detect_gpu()` as a validation check, or attempt GPU load with graceful CPU fallback.

---

### BUG-05: Resampler `flush()` output sample count formula is wrong
**File:** `src-tauri/src/audio.rs:66`

```rust
let out_samples = (remaining * 16000 + self.chunk_size - 1) / self.chunk_size;
```

This ceiling-division formula uses `chunk_size` (1024, the input chunk size) where it should use the input sample rate. For 48kHz native rate with 500 remaining samples:
- Expected output: ~167 samples (500 * 16000/48000)
- Formula produces: 7813 samples (500 * 16000 / 1024)

The `min(out[0].len())` clamp on line 67 prevents a panic, but the "trim padded output" logic becomes a no-op, appending a few ms of silence from zero-padded input. `in_rate` is not stored on `ResamplingState`.

**Fix:** Store `in_rate` in `ResamplingState` and compute `(remaining as f64 * 16000.0 / in_rate as f64).ceil() as usize`.

---

### BUG-06: Microphone switch during active recording silently drops audio
**File:** `src-tauri/src/lib.rs:596-637`

`set_microphone` acquires `AudioCaptureMutex`, replaces the entire `AudioCapture` (dropping old stream), and releases the lock. If called during recording:
1. Old `AudioCapture`'s `buffer` Arc still has live references in hotkey handler and VAD worker
2. New `AudioCapture` has a fresh empty buffer
3. `flush_and_stop()` locks the new `AudioCaptureMutex` and gets the new empty buffer
4. Active recording is silently lost

No guard checks pipeline state before allowing mic switch.

**Fix:** Check pipeline state (reject if RECORDING/PROCESSING), or flush old buffer before replacing.

---

### BUG-07: `ProfileSwitcher` description lookup uses wrong key format
**File:** `src/components/ProfileSwitcher.tsx:9-12`

```typescript
const PROFILE_DESCRIPTIONS: Record<string, string> = {
  structural_engineering: 'Engineering terminology bias...',
  general: 'No domain bias, default settings',
};
```

The key `structural_engineering` (underscore) does not match the profile ID `structural-engineering` (hyphen) from `profiles.rs:47`. The lookup `PROFILE_DESCRIPTIONS[profile.id]` returns `undefined`, falling through to `'Custom profile'`.

**Fix:** Change key to `'structural-engineering'`.

---

## Medium

### BUG-08: UTF-8 byte slicing in log truncation can panic
**File:** `src-tauri/src/transcribe.rs:172-176`

```rust
if result.len() > 80 {
    format!("{}...", &result[..80])
}
```

`result[..80]` slices by byte index. If byte 80 falls inside a multi-byte UTF-8 character, this panics with `byte index 80 is not a char boundary`. While the model targets English, whisper can produce Unicode in edge cases (em-dashes, accented names).

**Fix:** Use `result.chars().take(80).collect::<String>()` or `result.char_indices()`.

---

### BUG-09: Same UTF-8 truncation panic risk in pipeline logging
**File:** `src-tauri/src/pipeline.rs:183-186`

Same byte-slicing pattern: `&to_inject[..60]`.

---

### BUG-10: `pill-result` listener does not clear previous exit timer
**File:** `src/Pill.tsx:66-74`

The `pill-result` handler starts an exit animation timeout without calling `clearAllTimers()` first. If `pill-result` fires while a `pill-hide` timer is pending (the backend sends both in `reset_to_idle()`), both timers execute. The first timer's `exitTimerRef.current = null` wipes the ref, preventing `clearAllTimers()` from cancelling the second.

Compare with `pill-hide` (line 39) which correctly calls `clearAllTimers()`.

**Fix:** Add `clearAllTimers()` at the top of the `pill-result` handler.

---

### BUG-11: `list_models` calls `detect_gpu()` on every invocation (NVML init each time)
**File:** `src-tauri/src/lib.rs:681-702`

`detect_gpu()` initializes NVML, queries GPU, and tears down on every call. This runs each time the frontend requests the model list (every settings page visit). If NVML is slow, the UI hangs.

**Fix:** Cache GPU detection result at startup and reuse.

---

### BUG-12: `check_first_run` also calls `detect_gpu()` redundantly
**File:** `src-tauri/src/lib.rs:721-736`

Same as BUG-11. Combined with `list_models`, a settings page load may initialize NVML multiple times.

---

### BUG-13: `handleCancel` in FirstRun does not cancel the backend download
**File:** `src/components/FirstRun.tsx:117-124`

`handleCancel()` sets a client-side flag and resets UI state. The backend `download_model` continues downloading hundreds of MB to disk. The user sees "idle" but download runs to completion, wasting bandwidth and disk I/O. Starting a new download concurrently may cause temp file conflicts.

**Fix:** Implement backend cancellation (e.g., `CancellationToken` or drop-based abort).

---

### BUG-14: Dual-writer conflict on `settings.json` between Tauri Store plugin and manual Rust JSON
**File:** `src-tauri/src/lib.rs` (multiple), `src/lib/store.ts:27`

Both the Tauri Store plugin (frontend, `autoSave: 100ms`) and manual `serde_json` read/write cycles (backend) operate on the same `settings.json` file. If frontend saves a key and backend reads + rewrites within the 100ms debounce window, one write clobbers the other. Classic read-modify-write race with no file-level locking.

The key names also differ: frontend uses `recordingMode`/`activeProfile`/`selectedMic`, backend uses `recording_mode`/`active_profile_id`/`microphone_device`. However, `set_recording_mode` (Tauri command) correctly writes both, so the backend's copy is kept in sync through the command path. The risk is when either persistence path fails independently.

---

### BUG-15: `Ctrl+V` simulation can leave Ctrl key stuck on error
**File:** `src-tauri/src/inject.rs:33-35`

```rust
enigo.key(Key::Control, Press).map_err(|e| e.to_string())?;
enigo.key(Key::Unicode('v'), Click).map_err(|e| e.to_string())?;
enigo.key(Key::Control, Release).map_err(|e| e.to_string())?;
```

If `Click` returns an error, the early `?` return leaves Control in the pressed state globally. Every subsequent keypress is interpreted as Ctrl+key.

**Fix:** Ensure Release always executes:
```rust
enigo.key(Key::Control, Press).map_err(|e| e.to_string())?;
let result = enigo.key(Key::Unicode('v'), Click);
let _ = enigo.key(Key::Control, Release);
result.map_err(|e| e.to_string())?;
```

---

### BUG-16: `ModelSelector` error state applies to all non-downloaded models, not just the one that failed
**File:** `src/components/ModelSelector.tsx:104`

```typescript
const hasError = downloadingId === null && downloadError !== null && !model.downloaded;
```

When a download fails, `downloadingId` becomes `null` and `downloadError` persists. This condition is `true` for ALL non-downloaded models, so both model cards show the same error message.

**Fix:** Track `errorModelId` separately and check `errorModelId === model.id`.

---

## Low

### BUG-17: `tray.rs` menu handler uses `unwrap()` on window show/focus
**File:** `src-tauri/src/tray.rs:60-61`

```rust
w.show().unwrap();
w.set_focus().unwrap();
```

If the window operation fails (transitional state, Windows API error), these unwraps panic and crash the app from the tray menu handler.

**Fix:** Use `.ok()` or `let _ = w.show();`.

---

### BUG-18: Same `unwrap()` risk in single-instance handler
**File:** `src-tauri/src/lib.rs:966-967`

Same unwrap pattern. Second instance launch could crash the running app.

---

### BUG-19: `DictionaryEditor` uses array index as React key
**File:** `src/components/DictionaryEditor.tsx:84`

`<tr key={i}>` causes React to reuse wrong DOM nodes on row deletion, potentially showing stale values in input fields.

**Fix:** Use stable unique IDs per row.

---

### BUG-20: `FrequencyBars` builds DOM imperatively inside a React-managed container
**File:** `src/components/FrequencyBars.tsx:36-91`

Imperative `container.appendChild(bar)` creates children outside React's reconciliation. Stable with current empty-deps `[]` effect, but fragile if className or props change triggers a re-render of the container.

---

### BUG-21: `bellCurve` division by zero if BAR_COUNT is 1
**File:** `src/components/FrequencyBars.tsx:17`

`(i / (count - 1))` divides by zero when `count === 1`. Not reachable with `BAR_COUNT = 24` but would surface if the constant is changed.

---

### BUG-22: `audio.rs:19` comment is misleading about chunk timing
**File:** `src-tauri/src/audio.rs:19`

Comment says `// ~64ms at 16kHz` but `chunk_size = 1024` is the INPUT chunk size. At 48kHz native rate, 1024 samples = 21.3ms, not 64ms.

---

### BUG-23: `vad.rs` toggle mode can hang for 60 seconds with intermittent non-speech noise
**File:** `src-tauri/src/vad.rs:170-188`

Silence counter only starts after `ever_spoke = true` (line 142-173). Intermittent false-positive speech frames from typing/breathing reset `silence_frames` to 0 (line 172), preventing the 3-second silence threshold from being reached. Recording hangs until the 60-second safety cap. Not a bug per se, but a UX edge case where the user toggles on accidentally and can't easily stop without tapping the hotkey again.

---

## Top 3 Priority Fixes

1. **BUG-01** (Critical): Register fallback audio state on mic failure -- prevents panics
2. **BUG-02** (Critical): Fix `save_corrections` to actually remove deleted entries
3. **BUG-07** (High): Fix profile description key mismatch (`structural_engineering` vs `structural-engineering`)
