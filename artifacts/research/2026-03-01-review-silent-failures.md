# Silent Failure Audit — Full Codebase Review

**Date:** 2026-03-01
**Scope:** All source files in `src/` (TypeScript/React) and `src-tauri/src/` (Rust)
**Files reviewed:** 34 source files

---

## Summary

| Severity | Count |
|----------|-------|
| CRITICAL | 6     |
| HIGH     | 12    |
| MEDIUM   | 11    |
| LOW      | 5     |
| **Total** | **34** |

The codebase has a consistent pattern of silently swallowing errors in the audio pipeline's real-time path (understandable for lock contention avoidance), but this pattern leaks into non-real-time code where errors should be surfaced. The frontend has several fire-and-forget async calls that swallow invoke failures without user feedback. The tray icon module has an intentional "silently ignore failures" policy that means tray icon corruption is invisible. Several `.unwrap()` calls on Mutex locks will panic and crash the app on poisoned mutexes rather than handling the error.

---

## CRITICAL Findings

### C1. Audio callback silently drops samples on lock contention

**File:** `src-tauri/src/audio.rs:155-162`

```rust
if let Ok(mut rs) = resampling_cb.try_lock() {
    let resampled = rs.push(&mono);
    if !resampled.is_empty() {
        if let Ok(mut buf) = buffer_cb.try_lock() {
            buf.extend_from_slice(&resampled);
        }
    }
}
```

**Severity:** CRITICAL

**Issue:** Two nested `try_lock()` calls silently discard audio samples when either lock is contended. There is no logging, no counter, no metric. If lock contention is high (e.g., long whisper inference holding the buffer lock), entire seconds of audio could be silently lost without any indication.

**Hidden errors:** Lock poisoning (panicked thread), sustained contention from another thread holding the lock too long.

**User impact:** User speaks but transcription is missing words or entire phrases. No log entry, no error, no way to diagnose. The user blames whisper accuracy when the real problem is dropped audio frames.

**Recommendation:** This is a real-time audio callback, so blocking is correctly avoided. However, the failure should be tracked. Add an `AtomicU64` dropped-frame counter to `AudioCapture` and log the count at `flush_and_stop()` time. Example:

```rust
// In the callback:
if let Ok(mut rs) = resampling_cb.try_lock() {
    let resampled = rs.push(&mono);
    if !resampled.is_empty() {
        if let Ok(mut buf) = buffer_cb.try_lock() {
            buf.extend_from_slice(&resampled);
        } else {
            dropped_frames_cb.fetch_add(1, Ordering::Relaxed);
        }
    }
} else {
    dropped_frames_cb.fetch_add(1, Ordering::Relaxed);
}

// In flush_and_stop():
let dropped = self.dropped_frames.load(Ordering::Relaxed);
if dropped > 0 {
    log::warn!("Audio: {} callback frames dropped due to lock contention", dropped);
}
```

---

### C2. Resampler errors silently discard audio chunks

**File:** `src-tauri/src/audio.rs:42-44`

```rust
if let Ok(out) = self.resampler.process(&chunk, None) {
    output.extend_from_slice(&out[0]);
}
```

**File:** `src-tauri/src/audio.rs:64-68`

```rust
if let Ok(out) = self.resampler.process(&chunk, None) {
    let out_samples = (remaining * 16000 + self.chunk_size - 1) / self.chunk_size;
    let take = out_samples.min(out[0].len());
    output.extend_from_slice(&out[0][..take]);
}
```

**Severity:** CRITICAL

**Issue:** If the FFT resampler returns an error (corrupted state, invalid chunk size, internal overflow), the error is silently swallowed. No logging, no tracking. The `push()` method returns a shorter-than-expected output and the `flush()` method returns nothing. The caller has no way to know data was lost.

**Hidden errors:** `rubato::ResamplerError` variants including `WrongNumberOfInputFrames`, `WrongNumberOfChannels`, internal FFT errors.

**User impact:** Identical to C1 -- audio silently lost, blamed on whisper accuracy. Worse: a resampler in a corrupted state may silently produce garbage output for all subsequent chunks in the session, producing an entire recording of corrupt audio.

**Recommendation:** At minimum, log the error. In the callback path, use a counter like C1. In `flush()`, which is not a real-time path, log directly:

```rust
// In push() (callback path):
match self.resampler.process(&chunk, None) {
    Ok(out) => output.extend_from_slice(&out[0]),
    Err(_) => { /* increment atomic counter */ }
}

// In flush() (non-callback path):
match self.resampler.process(&chunk, None) {
    Ok(out) => { /* ... */ }
    Err(e) => log::error!("Resampler flush failed: {} -- final audio segment lost", e),
}
```

---

### C3. Mutex lock failures silently skip buffer operations

**File:** `src-tauri/src/audio.rs:228-238`

```rust
pub fn clear_buffer(&self) {
    if let Ok(mut buf) = self.buffer.lock() {
        buf.clear();
    }
    if let Ok(mut rs) = self.resampling.lock() {
        rs.staging.clear();
    }
}
```

**Severity:** CRITICAL

**Issue:** If the buffer or resampling mutex is poisoned (which happens when a thread panics while holding the lock), `clear_buffer()` silently does nothing. The next recording will contain stale audio from the previous session prepended to the new recording.

This is called at the start of every recording (`audio.clear_buffer()` in the hotkey handler). A poisoned mutex means every subsequent recording includes leftover audio from before the panic, producing garbled transcriptions.

**Hidden errors:** `PoisonError` from a previously panicked thread.

**User impact:** Every recording after a panic produces incorrect transcriptions because it includes audio from a previous session. The user has no idea why transcriptions are wrong and no error is ever shown.

**Recommendation:** Use `unwrap_or_else(|e| e.into_inner())` to recover from poisoned mutexes (as done in `vad.rs:156`), or at least log the failure:

```rust
pub fn clear_buffer(&self) {
    match self.buffer.lock() {
        Ok(mut buf) => buf.clear(),
        Err(e) => {
            log::error!("Audio buffer mutex poisoned during clear: {} -- recovering", e);
            e.into_inner().clear();
        }
    }
    match self.resampling.lock() {
        Ok(mut rs) => rs.staging.clear(),
        Err(e) => {
            log::error!("Resampling mutex poisoned during clear: {} -- recovering", e);
            e.into_inner().staging.clear();
        }
    }
}
```

---

### C4. `flush_and_stop()` silently returns 0 on mutex failure

**File:** `src-tauri/src/audio.rs:242-259`

```rust
pub fn flush_and_stop(&self) -> usize {
    self.recording.store(false, Ordering::Relaxed);
    if let Ok(mut rs) = self.resampling.lock() {
        let tail = rs.flush();
        if !tail.is_empty() {
            if let Ok(mut buf) = self.buffer.lock() {
                buf.extend_from_slice(&tail);
            }
        }
    }
    self.buffer
        .lock()
        .map(|b| b.len())
        .unwrap_or(0)
}
```

**Severity:** CRITICAL

**Issue:** Three separate silent failure points: (1) resampling lock fails -- tail samples lost; (2) buffer lock fails inside resampling block -- tail samples lost; (3) final buffer lock fails -- returns 0 as if no audio was recorded. None of these log anything.

The `.unwrap_or(0)` at the end is particularly dangerous: it tells the caller "0 samples recorded" when the real situation is "mutex is poisoned and I cannot access the buffer." The pipeline then discards a valid recording.

**User impact:** User speaks, releases hotkey, recording appears to have captured 0 samples. Pipeline discards it with "audio too short" message. User sees an error flash on the pill but has no idea their audio was captured and then lost due to a mutex issue.

**Recommendation:**

```rust
pub fn flush_and_stop(&self) -> Result<usize, String> {
    self.recording.store(false, Ordering::Relaxed);

    match self.resampling.lock() {
        Ok(mut rs) => {
            let tail = rs.flush();
            if !tail.is_empty() {
                match self.buffer.lock() {
                    Ok(mut buf) => buf.extend_from_slice(&tail),
                    Err(e) => log::error!("Buffer mutex poisoned during flush: {} -- {} tail samples lost", e, tail.len()),
                }
            }
        }
        Err(e) => log::error!("Resampling mutex poisoned during flush: {}", e),
    }

    self.buffer
        .lock()
        .map(|b| b.len())
        .map_err(|e| format!("Buffer mutex poisoned: {}", e))
}
```

---

### C5. `get_buffer()` returns empty vec on mutex failure

**File:** `src-tauri/src/audio.rs:262-267`

```rust
pub fn get_buffer(&self) -> Vec<f32> {
    self.buffer
        .lock()
        .map(|b| b.clone())
        .unwrap_or_default()
}
```

**Severity:** CRITICAL

**Issue:** `.unwrap_or_default()` returns an empty `Vec<f32>` if the mutex is poisoned. The pipeline receives zero samples and interprets it as "no speech recorded" rather than "system error." This masks a poisoned mutex as an empty recording.

**User impact:** Valid audio recording silently replaced with nothing. Pipeline says "insufficient speech" and user sees an error flash.

**Recommendation:** Return `Result<Vec<f32>, String>` and propagate the error, or at minimum use `unwrap_or_else` with logging as in C3.

---

### C6. 13 `.unwrap()` calls on `Mutex::lock()` will crash the app on poisoned mutex

**File:** Multiple locations in `src-tauri/src/lib.rs` and `src-tauri/src/pipeline.rs`

Locations (non-exhaustive):
- `src-tauri/src/pipeline.rs:51` -- `audio_mutex.0.lock().unwrap()`
- `src-tauri/src/pipeline.rs:95` -- `profile.0.lock().unwrap()`
- `src-tauri/src/pipeline.rs:105` -- `whisper_mutex.0.lock().unwrap()`
- `src-tauri/src/pipeline.rs:168` -- `engine.0.lock().unwrap()`
- `src-tauri/src/pipeline.rs:175` -- `profile.0.lock().unwrap()`
- `src-tauri/src/lib.rs:169` -- `audio_mutex.0.lock().unwrap()` (hotkey handler)
- `src-tauri/src/lib.rs:196` -- `audio_mutex.0.lock().unwrap()` (toggle mode)
- `src-tauri/src/lib.rs:287` -- `state.0.lock().unwrap()` (start_recording)
- `src-tauri/src/lib.rs:298` -- `state.0.lock().unwrap()` (stop_recording)
- `src-tauri/src/lib.rs:310` -- `state.0.lock().unwrap()` (save_test_wav)
- `src-tauri/src/lib.rs:414` -- `state.0.lock().unwrap()` (get_profiles)
- `src-tauri/src/lib.rs:468` -- `profile_state.0.lock().unwrap()` (set_active_profile)
- `src-tauri/src/lib.rs:500` -- `state.0.lock().unwrap()` (save_corrections)
- `src-tauri/src/lib.rs:966-967` -- `w.show().unwrap()`, `w.set_focus().unwrap()` (single-instance handler)

**Severity:** CRITICAL

**Issue:** If any thread panics while holding one of these mutexes, the mutex becomes poisoned. Every subsequent `.unwrap()` on that mutex panics, crashing the Tauri async runtime or the hotkey handler thread. Since the hotkey handler runs on the global shortcut thread, a panic there likely crashes the entire application.

The single-instance handler at line 966-967 will crash the running instance if `show()` or `set_focus()` fails (e.g., window was destroyed).

**User impact:** App crashes with no user-visible error message. On Windows, this typically shows a generic "VoiceType has stopped working" dialog.

**Recommendation:** In Tauri command handlers (which can return `Result`), replace `.unwrap()` with `.map_err()`. In the hotkey handler (which cannot return Result), use `.unwrap_or_else()` with logging and early return. For the single-instance handler:

```rust
.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
    if let Some(w) = app.get_webview_window("settings") {
        if let Err(e) = w.show() {
            log::error!("Failed to show settings window: {}", e);
        }
        if let Err(e) = w.set_focus() {
            log::error!("Failed to focus settings window: {}", e);
        }
    }
}))
```

---

## HIGH Findings

### H1. Tray icon failures are completely silent by design

**File:** `src-tauri/src/tray.rs:29-41`

```rust
/// Failures are silently ignored -- tray icon is non-critical feedback.
pub fn set_tray_state(app: &tauri::AppHandle, state: TrayState) {
    // ...
    if let Some(tray) = app.tray_by_id("tray") {
        if let Ok(image) = tauri::image::Image::from_bytes(icon_bytes) {
            let _ = tray.set_icon(Some(image));
        }
        let _ = tray.set_tooltip(Some(tooltip));
    }
}
```

**Severity:** HIGH

**Issue:** Three nested silent failures: (1) tray not found by ID (returns None), (2) icon bytes fail to decode (returns Err), (3) set_icon/set_tooltip fail (discarded with `let _`). The comment says "non-critical feedback" but the tray icon is the user's only indication of app state when the settings window is hidden.

**User impact:** If the tray icon breaks (e.g., icon PNG is corrupted in the binary), the user has no way to know whether the app is recording, processing, or idle. They continue speaking with no visual feedback.

**Recommendation:** Log at `warn` level so the failure is at least visible in logs:

```rust
if let Some(tray) = app.tray_by_id("tray") {
    match tauri::image::Image::from_bytes(icon_bytes) {
        Ok(image) => {
            if let Err(e) = tray.set_icon(Some(image)) {
                log::warn!("Failed to set tray icon: {}", e);
            }
        }
        Err(e) => log::warn!("Failed to decode tray icon bytes: {}", e),
    }
    if let Err(e) = tray.set_tooltip(Some(tooltip)) {
        log::warn!("Failed to set tray tooltip: {}", e);
    }
} else {
    log::warn!("Tray icon not found by id 'tray' -- state indicator unavailable");
}
```

---

### H2. `emit_to("pill", ...)` failures are silently discarded everywhere

**Files:** `src-tauri/src/pipeline.rs` (lines 65, 79, 111, 145, 160, 200, 204, 208, 251, 252), `src-tauri/src/lib.rs` (lines 178, 203, 236, 260, 261, 280, 1093, 1118, 1151, 1175), `src-tauri/src/vad.rs` (lines 251, 256, 257, 280)

Example pattern:
```rust
app.emit_to("pill", "pill-result", "error").ok();
app.emit_to("pill", "pill-state", "processing").ok();
app.emit_to("pill", "pill-hide", ()).ok();
```

**Severity:** HIGH

**Issue:** Every `emit_to` call in the codebase uses `.ok()` to discard the Result. If the pill window doesn't exist, was closed, or the event system is broken, these all fail silently. The pill window is the primary user feedback during recording/processing.

**User impact:** If the pill window fails to receive events, the user gets no visual feedback about recording state, processing state, or errors. The pill might remain stuck in "recording" state forever or never appear at all.

**Recommendation:** Log at `debug` level (not `warn`, since the pill window might legitimately not exist during startup):

```rust
if let Err(e) = app.emit_to("pill", "pill-state", "processing") {
    log::debug!("Failed to emit pill-state: {}", e);
}
```

---

### H3. Frontend `loadSettings()` has a bare `catch {}` that fabricates status

**File:** `src/App.tsx:34-37`

```typescript
} catch {
    // check_first_run unavailable (whisper feature not compiled) -- skip first-run gate
    setFirstRunStatus({ needsSetup: false, gpuDetected: false, recommendedModel: '' });
}
```

**Severity:** HIGH

**Issue:** This catch block catches ALL errors from `invoke('check_first_run')`, not just "command not found." Network errors, serialization errors, backend panics, or any other failure will be silently treated as "whisper feature not compiled." The fabricated `needsSetup: false` skips the first-run setup flow even when it should have been shown.

**Hidden errors caught:** Backend panic, IPC serialization failure, Tauri runtime errors, any `invoke` infrastructure error.

**User impact:** If `check_first_run` fails for any reason other than the command not existing, the user is silently dropped into the main app without downloading a model. Transcription will then fail with "whisper model not loaded" every time they try to dictate.

**Recommendation:** Inspect the error to distinguish "command not registered" from other failures:

```typescript
try {
    const status = await invoke<FirstRunStatus>('check_first_run');
    setFirstRunStatus(status);
} catch (e) {
    const msg = String(e);
    if (msg.includes('not found') || msg.includes('did not handle')) {
        // Command not registered (whisper feature not compiled)
        setFirstRunStatus({ needsSetup: false, gpuDetected: false, recommendedModel: '' });
    } else {
        console.error('check_first_run failed unexpectedly:', e);
        // Still allow app to load but log the error
        setFirstRunStatus({ needsSetup: false, gpuDetected: false, recommendedModel: '' });
    }
}
```

---

### H4. `loadSettings()` in App.tsx is fire-and-forget with no error handling

**File:** `src/App.tsx:29-70`

```typescript
useEffect(() => {
    async function loadSettings() {
        // ... all the store.get() calls with no try/catch ...
        const store = await getStore();
        const savedHotkey = await store.get<string>('hotkey');
        // ... 5 more store.get() calls ...
        setLoaded(true);
    }
    loadSettings();
}, []);
```

**Severity:** HIGH

**Issue:** `loadSettings()` is called as a fire-and-forget async function. If `getStore()` throws (file corruption, permissions error), or any `store.get()` throws, the function aborts before `setLoaded(true)`. The app is stuck forever on the "Loading..." screen with no error message.

**User impact:** App shows "Loading..." forever. User has to force-quit. No error message explains what went wrong.

**Recommendation:** Wrap in try/catch with fallback to defaults:

```typescript
loadSettings().catch((err) => {
    console.error('Failed to load settings:', err);
    setLoaded(true); // Show app with defaults rather than infinite loading
});
```

---

### H5. MicrophoneSection `loadDevices()` has no error handling

**File:** `src/components/sections/MicrophoneSection.tsx:15-20`

```typescript
async function loadDevices() {
    const deviceList = await invoke<string[]>('list_input_devices');
    setDevices(deviceList);
    setLoading(false);
}
loadDevices();
```

**Severity:** HIGH

**Issue:** If `invoke('list_input_devices')` fails (audio subsystem error, backend crash), the promise rejects with no catch handler. The `loading` state stays `true` forever and the component shows an infinite loading skeleton.

**User impact:** Microphone section shows an infinite loading animation. User cannot switch microphones and has no idea why.

**Recommendation:**

```typescript
async function loadDevices() {
    try {
        const deviceList = await invoke<string[]>('list_input_devices');
        setDevices(deviceList);
    } catch (err) {
        console.error('Failed to list input devices:', err);
        setDevices(['System Default']); // Fallback so user can still interact
    } finally {
        setLoading(false);
    }
}
```

---

### H6. `handleDeviceChange()` in MicrophoneSection has no error handling

**File:** `src/components/sections/MicrophoneSection.tsx:23-28`

```typescript
async function handleDeviceChange(deviceName: string) {
    onSelectedMicChange(deviceName);
    const store = await getStore();
    await store.set('selectedMic', deviceName);
    await invoke('set_microphone', { deviceName });
}
```

**Severity:** HIGH

**Issue:** `invoke('set_microphone')` can fail if the device is unplugged, exclusive mode locked, or the audio subsystem errors. The error is completely unhandled. Worse, `onSelectedMicChange()` is called first, so the UI shows the new mic as selected even though the backend failed to switch. The UI and backend are now desynchronized.

**User impact:** User selects a mic, UI shows it as selected, but audio continues capturing from the old device. Transcriptions may be silent or from the wrong mic. No error is shown.

**Recommendation:**

```typescript
async function handleDeviceChange(deviceName: string) {
    try {
        await invoke('set_microphone', { deviceName });
        onSelectedMicChange(deviceName);
        const store = await getStore();
        await store.set('selectedMic', deviceName);
    } catch (err) {
        // TODO: show error to user in the UI
        console.error('Failed to switch microphone:', err);
    }
}
```

---

### H7. `handleProfileSelect()` in ProfilesSection has no error handling

**File:** `src/components/sections/ProfilesSection.tsx:33-41`

```typescript
async function handleProfileSelect(id: string) {
    onActiveProfileChange(id);
    const store = await getStore();
    await store.set('activeProfile', id);
    await invoke('set_active_profile', { profileId: id });
    const correctionMap = await invoke<Record<string, string>>('get_corrections');
    setCorrections(correctionMap);
}
```

**Severity:** HIGH

**Issue:** Four async operations with no error handling. If `set_active_profile` fails (unknown profile ID, filesystem error persisting settings), the UI shows the new profile as active but the backend is still on the old profile. Corrections shown in the editor are from the new profile but applied corrections during transcription are from the old one.

Same issue: `onActiveProfileChange(id)` is called before the backend confirms success, causing UI/backend desynchronization on failure.

**User impact:** User thinks they switched profiles, but transcription uses the old profile's corrections and initial prompt.

**Recommendation:** Call `onActiveProfileChange(id)` only after backend confirms success, wrap in try/catch.

---

### H8. `handleSelect()` in ProfileSwitcher swallows invoke errors

**File:** `src/components/ProfileSwitcher.tsx:21-25`

```typescript
async function handleSelect(profileId: string) {
    if (profileId === activeId) return;
    await invoke('set_active_profile', { profileId });
    onSelect(profileId);
}
```

**Severity:** HIGH

**Issue:** If `invoke('set_active_profile')` rejects, the error propagates as an unhandled promise rejection. No user feedback, no error state. The event handler is called from an `onClick`, which swallows the promise.

**User impact:** Profile switch silently fails. User clicks a profile card, nothing visually changes (since `onSelect` is never called), and no error message is shown.

**Recommendation:** Add try/catch with user feedback.

---

### H9. `handleAllCapsToggle()` in ProfilesSection has no error handling

**File:** `src/components/sections/ProfilesSection.tsx:43-47`

```typescript
async function handleAllCapsToggle() {
    const next = !allCaps;
    setAllCaps(next);
    await invoke('set_all_caps', { enabled: next });
}
```

**Severity:** HIGH

**Issue:** State is set optimistically before the backend call. If `invoke` fails, the UI shows ALL CAPS as toggled but the backend has the old value. No error handling, no rollback.

**User impact:** User toggles ALL CAPS, UI shows it as enabled, but transcribed text comes through in normal case.

**Recommendation:** Await the invoke first, then set state:

```typescript
async function handleAllCapsToggle() {
    const next = !allCaps;
    try {
        await invoke('set_all_caps', { enabled: next });
        setAllCaps(next);
    } catch (err) {
        console.error('Failed to set ALL CAPS:', err);
    }
}
```

---

### H10. `handleCorrectionsChange()` in ProfilesSection has no error handling

**File:** `src/components/sections/ProfilesSection.tsx:49-52`

```typescript
async function handleCorrectionsChange(updated: Record<string, string>) {
    setCorrections(updated);
    await invoke('save_corrections', { corrections: updated });
}
```

**Severity:** HIGH

**Issue:** Same optimistic update pattern. UI shows new corrections, backend may have failed to save them. Next app restart reverts to old corrections with no indication.

---

### H11. `handleToggle()` in AutostartToggle has no error handling

**File:** `src/components/AutostartToggle.tsx:16-29`

```typescript
async function handleToggle() {
    const next = !enabled;
    if (next) {
        await enable();
    } else {
        await disable();
    }
    const store = await getStore();
    await store.set('autostart', next);
    setEnabled(next);
}
```

**Severity:** HIGH

**Issue:** `enable()` and `disable()` from `@tauri-apps/plugin-autostart` can throw (registry access denied, policy restrictions). No error handling. If `enable()` fails, the UI still shows autostart as enabled.

**User impact:** User enables autostart, UI confirms it, but the app does not actually start at login. User discovers this days later.

**Recommendation:** Wrap in try/catch, only update state on success.

---

### H12. `isEnabled()` promise rejection unhandled in AutostartToggle

**File:** `src/components/AutostartToggle.tsx:10-13`

```typescript
useEffect(() => {
    isEnabled().then((val) => {
        setEnabled(val);
        setLoading(false);
    });
}, []);
```

**Severity:** HIGH

**Issue:** No `.catch()` handler. If `isEnabled()` rejects, the component stays in loading state forever, showing an infinite pulse animation.

**Recommendation:** Add `.catch()`:

```typescript
isEnabled()
    .then((val) => setEnabled(val))
    .catch((err) => console.error('Failed to check autostart status:', err))
    .finally(() => setLoading(false));
```

---

## MEDIUM Findings

### M1. `handleToggle()` in ThemeToggle has no error handling

**File:** `src/components/ThemeToggle.tsx:11-26`

```typescript
async function handleToggle() {
    const next: 'light' | 'dark' = isDark ? 'light' : 'dark';
    if (next === 'dark') {
        document.documentElement.classList.add('dark');
    } else {
        document.documentElement.classList.remove('dark');
    }
    const store = await getStore();
    await store.set('theme', next);
    onChange(next);
}
```

**Severity:** MEDIUM

**Issue:** If `store.set()` throws, the DOM class is already changed (theme visually applied) but not persisted. The user sees the new theme until next restart, then it reverts. No error shown.

**Recommendation:** The DOM change is acceptable as a visual response, but persist error should be caught and logged.

---

### M2. `handleSelect` in RecordingModeToggle has no error handling

**File:** `src/components/RecordingModeToggle.tsx:27-38`

```typescript
async function handleSelect(mode: 'hold' | 'toggle') {
    if (mode === value) return;
    await invoke('set_recording_mode', { mode });
    const store = await getStore();
    await store.set('recordingMode', mode);
    onChange(mode);
}
```

**Severity:** MEDIUM

**Issue:** If `invoke('set_recording_mode')` fails, UI continues with old mode visually (since `onChange` is never called), but no error is shown. User has no idea the mode switch failed.

---

### M3. `loadInitial()` in ProfilesSection is fire-and-forget

**File:** `src/components/sections/ProfilesSection.tsx:19-29`

```typescript
async function loadInitial() {
    await invoke('set_active_profile', { profileId: activeProfileId });
    const [profileList, correctionMap] = await Promise.all([
        invoke<ProfileInfo[]>('get_profiles'),
        invoke<Record<string, string>>('get_corrections'),
    ]);
    setProfiles(profileList);
    setCorrections(correctionMap);
    setLoading(false);
}
loadInitial();
```

**Severity:** MEDIUM

**Issue:** No error handling. If any of the three invoke calls fail, the component stays in loading state forever.

---

### M4. `loadModels()` in ModelSection has no error handling

**File:** `src/components/sections/ModelSection.tsx:19-22`

```typescript
async function loadModels() {
    const modelList = await invoke<ModelInfo[]>('list_models');
    setModels(modelList);
    setLoading(false);
}
```

**Severity:** MEDIUM

**Issue:** Unhandled rejection. Loading skeleton shown forever on failure.

---

### M5. `handleModelSelect()` in ModelSection has no error handling

**File:** `src/components/sections/ModelSection.tsx:25-29`

```typescript
async function handleModelSelect(modelId: string) {
    onSelectedModelChange(modelId);
    const store = await getStore();
    await store.set('selectedModel', modelId);
    await invoke('set_model', { modelId });
}
```

**Severity:** MEDIUM

**Issue:** `invoke('set_model')` can fail (model file missing, whisper load error). `onSelectedModelChange()` is called first, so UI shows new model as selected even on failure. The whisper context is still the old model, but UI says otherwise.

---

### M6. `handleDownloadComplete()` in ModelSection has no error handling

**File:** `src/components/sections/ModelSection.tsx:32-37`

```typescript
async function handleDownloadComplete(modelId: string) {
    const modelList = await invoke<ModelInfo[]>('list_models');
    setModels(modelList);
    await handleModelSelect(modelId);
}
```

**Severity:** MEDIUM

**Issue:** `list_models` or `handleModelSelect` could fail. No catch block. Errors propagate as unhandled rejections.

---

### M7. Pill window event listener setup is fire-and-forget

**File:** `src/Pill.tsx:31-74`

```typescript
appWindow.listen("pill-show", () => {
    clearAllTimers();
    appWindow.show();
    // ...
}).then((u) => unlisteners.push(u));
```

**Severity:** MEDIUM

**Issue:** `appWindow.listen()` returns a Promise. If it rejects, the `.then()` is skipped and the unlistener is never registered. More importantly, `appWindow.show()` (line 33) is called without awaiting or error handling -- if the window is in an invalid state, this throws silently.

**User impact:** Pill window may fail to register event listeners at startup. No events received, no pill shown during recording. User has no recording feedback.

**Recommendation:** Add `.catch()` handlers.

---

### M8. `inject_text` silently clears clipboard on restore failure

**File:** `src-tauri/src/inject.rs:52-55`

```rust
None => {
    // Original was empty or non-text -- clear by setting empty string
    let _ = clipboard.set_text("");
}
```

**Severity:** MEDIUM

**Issue:** If `clipboard.set_text("")` fails (clipboard locked by another process), the error is silently discarded with `let _`. The user's clipboard may retain the transcribed text instead of being cleared.

**User impact:** Minor -- clipboard contains transcribed text instead of being empty. But if the user does Ctrl+V in another app expecting their previous clipboard content, they get the transcription instead.

---

### M9. `cancel_stale_vad_worker` silently ignores poisoned mutex

**File:** `src-tauri/src/pipeline.rs:228-232`

```rust
let result = match vad_state.0.lock() {
    Ok(mut guard) => guard.take(),
    Err(_) => None,
};
```

**Severity:** MEDIUM

**Issue:** Poisoned mutex is silently treated as "no VAD worker." If the VAD worker state mutex is poisoned, a stale VAD worker may continue running and could trigger a second pipeline execution, causing double-transcription and double-injection.

---

### M10. `pill::show_pill()` falls back to primary monitor position without warning

**File:** `src-tauri/src/pill.rs:45-64`

```rust
let monitors = pill_window.available_monitors().unwrap_or_default();
```

**Severity:** MEDIUM

**Issue:** `.unwrap_or_default()` returns an empty Vec if the monitor enumeration fails. The function then falls through to "No monitors detected" and emits `pill-show` without positioning. The pill appears at whatever position it was last at (potentially off-screen).

---

### M11. `transcribe.rs` uses `expect()` for APPDATA which panics on missing env var

**File:** `src-tauri/src/transcribe.rs:17`

```rust
let appdata = std::env::var("APPDATA").expect("APPDATA environment variable not set");
```

**File:** `src-tauri/src/download.rs:45`

```rust
let appdata = std::env::var("APPDATA").expect("APPDATA environment variable not set");
```

**Severity:** MEDIUM

**Issue:** `expect()` panics if APPDATA is not set. While APPDATA is virtually always set on Windows, running in certain CI environments, Docker containers, or under `runas` with a stripped environment will crash the app with an unhelpful panic message.

**Recommendation:** Return `Result` instead of panicking.

---

## LOW Findings

### L1. VAD `cancel()` uses `let _` to discard send result

**File:** `src-tauri/src/vad.rs:95`

```rust
let _ = tx.send(());
```

**Severity:** LOW

**Issue:** If the receiver has already been dropped (VAD worker completed), the send fails silently. This is actually correct behavior (no-op cancel of completed work), but worth noting for documentation.

---

### L2. `run_pipeline()` spawned as fire-and-forget in multiple places

**Files:** `src-tauri/src/lib.rs:241-243`, `src-tauri/src/lib.rs:265-267`, `src-tauri/src/lib.rs:1156-1158`, `src-tauri/src/lib.rs:1180-1182`, `src-tauri/src/vad.rs:288-290`

```rust
tauri::async_runtime::spawn(async move {
    pipeline::run_pipeline(app_handle).await;
});
```

**Severity:** LOW

**Issue:** The `JoinHandle` returned by `spawn()` is discarded. If `run_pipeline()` panics, the panic is caught by tokio and logged but the `JoinHandle` error is never observed. The pipeline state machine's `reset_to_idle()` would not be called, leaving the pipeline stuck in PROCESSING forever.

**Recommendation:** Consider logging panics:

```rust
let handle = tauri::async_runtime::spawn(async move {
    pipeline::run_pipeline(app_handle).await;
});
tauri::async_runtime::spawn(async move {
    if let Err(e) = handle.await {
        log::error!("run_pipeline task panicked: {}", e);
        // Could also try to reset pipeline state here
    }
});
```

---

### L3. `list_input_devices` silently skips devices with no description

**File:** `src-tauri/src/lib.rs:583-588`

```rust
for device in devices {
    if let Ok(desc) = device.description() {
        names.push(desc.name().to_string());
    }
}
```

**Severity:** LOW

**Issue:** Devices that fail `.description()` are silently excluded from the list. A valid microphone with a corrupted description string would be invisible to the user.

---

### L4. Download event channel send failures are silently discarded

**File:** `src-tauri/src/download.rs:102, 114, 124, 147, 158, 167, 181, 198`

```rust
let _ = on_event.send(DownloadEvent::Started { ... });
```

**Severity:** LOW

**Issue:** Every `on_event.send()` result is discarded with `let _`. If the frontend Channel is broken (webview crashed, page reloaded during download), progress events are silently lost. The download continues to completion but the frontend never receives the "finished" event and stays stuck showing the progress bar.

**User impact:** If the settings window is closed/refreshed during download, reopening shows stale download state. Minor since downloads are infrequent.

---

### L5. No React error boundary in either window

**File:** `src/main.tsx`, `src/pill-main.tsx`

```typescript
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
        <App />
    </React.StrictMode>
);
```

**Severity:** LOW

**Issue:** Neither the settings window nor the pill window has a React error boundary. An unhandled rendering error (e.g., null reference in a component) will crash the entire React tree and show a blank white window. There is no recovery UI.

**User impact:** Settings window or pill window goes blank with no error message. User must restart the app.

**Recommendation:** Wrap each root in an error boundary component that shows a "Something went wrong" message with a retry button.

---

## Patterns of Concern

### Pattern 1: Optimistic UI Updates
Every frontend settings handler calls `onXyzChange()` or `setState()` BEFORE the backend `invoke()` confirms success. This systematically creates UI/backend desynchronization on failure. The correct pattern is: invoke first, update UI on success, show error on failure.

**Affected components:** ProfileSwitcher, RecordingModeToggle, MicrophoneSection, ModelSection, ProfilesSection (allCaps, corrections), ThemeToggle.

### Pattern 2: Fire-and-Forget Async in useEffect
Multiple components call async functions from `useEffect` without attaching error handlers. The async function is called with no `.catch()`, meaning promise rejections are unhandled.

**Affected components:** App.tsx (`loadSettings`), MicrophoneSection (`loadDevices`), ProfilesSection (`loadInitial`), ModelSection (`loadModels`), AutostartToggle (`isEnabled`).

### Pattern 3: `.ok()` / `let _` on Every Emit/Tray Call
The Rust backend systematically discards errors from `emit_to()` and tray operations. While individually these are low-severity, collectively they mean the entire user feedback layer (pill + tray) could break without any log evidence.

### Pattern 4: Silent Mutex Recovery via `if let Ok()`
Audio operations use `if let Ok(mut guard) = mutex.lock()` which silently skips the operation on poisoned mutexes. This is appropriate in the audio callback (C1) but inappropriate in `clear_buffer()` (C3) and `flush_and_stop()` (C4) where the operation is critical.
