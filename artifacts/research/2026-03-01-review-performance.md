# Performance Review — VoiceType

**Date:** 2026-03-01
**Scope:** All source files in `src/` (React/TypeScript) and `src-tauri/src/` (Rust)
**Focus:** Latency-critical paths, memory, CPU, re-renders, allocations, bundle size

---

## Critical (Latency-impacting on hot path)

### P1. VAD allocates a Vec per chunk via `.to_vec()` in both gate and worker

**Files:**
- `src-tauri/src/vad.rs:63` — `vad.predict(chunk.to_vec())`
- `src-tauri/src/vad.rs:168` — `vad.predict(samples)` (where `samples` is already a `Vec<f32>` from `.to_vec()` at line 158)

**Issue:** `vad_gate_check()` iterates over the entire audio buffer in 512-sample chunks. Each chunk calls `chunk.to_vec()`, allocating a new 2KB `Vec<f32>` (512 * 4 bytes). For a 5-second recording at 16kHz, that is ~156 allocations. The streaming VAD worker does the same at line 158.

The `voice_activity_detector::VoiceActivityDetector::predict()` function accepts `Vec<f32>` (not a slice), so the allocation may be unavoidable given the current API. However, this should be verified — if the underlying ONNX runtime accepts slices, a PR or fork could eliminate this.

**Severity:** Medium
**Impact:** ~156 allocations per 5s recording in the post-hoc gate. Each is small (2KB) but adds GC pressure. In the streaming worker, one allocation per 32ms is more concerning for sustained recording sessions.

---

### P2. `compute_rms()` reads from the entire buffer tail under a Mutex lock

**File:** `src-tauri/src/pill.rs:94-101`

**Issue:** `compute_rms()` takes the last 512 samples from the buffer. The buffer itself is `Arc<Mutex<Vec<f32>>>`. The level stream loop (`start_level_stream`) calls `buffer.try_lock()` every 33ms. This is fine for short recordings, but the buffer grows unboundedly during recording. The `try_lock` avoids blocking the audio callback, which is correct. However, `compute_rms` reads `buf.len()` under the lock, which requires dereferencing the entire `Vec` metadata. The actual work (`tail.iter().map()`) only touches 512 samples, so the lock duration is short. This is acceptable.

**Severity:** Low
**Impact:** Negligible — lock is held for microseconds. Noted for completeness.

---

### P3. Audio callback creates a new `Vec<f32>` for mono downmix every callback invocation

**File:** `src-tauri/src/audio.rs:148-151`

```rust
let mono: Vec<f32> = data
    .chunks(channels)
    .map(|frame| frame.iter().sum::<f32>() / channels as f32)
    .collect();
```

**Issue:** The cpal audio callback runs at high frequency (~every 5-10ms). Each invocation allocates a `Vec<f32>` for the mono downmix. For stereo input at 48kHz with 480-sample buffers, this allocates ~960 bytes per callback. The allocation then flows into `ResamplingState::push()` which does `self.staging.extend_from_slice(&mono)`, then drops the `mono` Vec.

A pre-allocated buffer in `ResamplingState` (or a ring buffer approach) would avoid this allocation entirely. The mono samples could be computed in-place or directly into the staging buffer.

**Severity:** Medium
**Impact:** ~100-200 small allocations per second during recording. On Windows, heap allocations in audio callbacks can cause priority inversion with the OS memory manager. The `try_lock` pattern already mitigates the worst case (callback never blocks), but allocation itself can occasionally stall.

---

### P4. `ResamplingState::push()` allocates inner Vec per chunk

**File:** `src-tauri/src/audio.rs:41`

```rust
let chunk: Vec<Vec<f32>> = vec![self.staging.drain(..self.chunk_size).collect()];
```

**Issue:** Each resampling chunk creates a `Vec<Vec<f32>>` (one inner Vec of 1024 f32s). The rubato `FftFixedIn::process()` API requires `&[Vec<f32>]` input, so this may be unavoidable. However, the inner Vec could be pre-allocated and reused across calls by keeping a persistent scratch buffer in `ResamplingState`.

**Severity:** Medium
**Impact:** One 4KB allocation per ~64ms of audio during recording. Adds to audio callback allocation pressure from P3.

---

### P5. `flush()` clones the staging buffer unnecessarily

**File:** `src-tauri/src/audio.rs:58`

```rust
let mut padded = self.staging.clone();
```

**Issue:** `flush()` clones the entire remaining staging buffer, then clears the original. This could be replaced with `std::mem::take(&mut self.staging)` to move the buffer instead of cloning, then resize the taken buffer in place.

**Severity:** Low
**Impact:** One allocation at end of each recording. Small — `staging` typically has < 1024 samples at flush time.

---

### P6. `list_models()` and `check_first_run()` call `detect_gpu()` on every invocation

**File:** `src-tauri/src/lib.rs:683`, `src-tauri/src/lib.rs:723`

**Issue:** `detect_gpu()` calls `Nvml::init()` which loads the NVML shared library and queries the GPU driver. This is a relatively expensive operation (potentially 10-50ms) involving dynamic library loading and driver IPC. Both `list_models()` and `check_first_run()` are Tauri commands called from the frontend. `list_models()` is called each time the Model section is opened. `check_first_run()` is called on every app launch.

The GPU detection result never changes during a session. It should be detected once at startup and cached in managed state.

**Severity:** High
**Impact:** 10-50ms of unnecessary latency each time the user opens the Model settings section. On `check_first_run()`, adds to startup latency. The NVML library load can be especially slow on first call.

---

### P7. `get_buffer()` clones the entire audio buffer

**File:** `src-tauri/src/audio.rs:262-267`

```rust
pub fn get_buffer(&self) -> Vec<f32> {
    self.buffer
        .lock()
        .map(|b| b.clone())
        .unwrap_or_default()
}
```

**Issue:** Called from `run_pipeline()` at `pipeline.rs:54`. For a 5-second recording at 16kHz, this clones 80,000 f32s = 320KB. For a 60-second toggle-mode recording, this clones 3.84MB. The clone happens while holding the Mutex lock, which means the audio callback is blocked during the copy (the callback uses `try_lock`, so it would drop samples rather than block, but samples are still lost).

A better approach: `std::mem::take()` on the buffer contents under lock (moves the Vec, gives an empty Vec back). The pipeline consumes the buffer and doesn't need the original to survive.

**Severity:** High
**Impact:** 320KB-3.84MB allocation + copy on the hot path between recording stop and transcription start. Adds directly to user-perceived latency. The lock duration scales linearly with buffer size.

---

### P8. `transcribe_audio` clones the result string for logging

**File:** `src-tauri/src/transcribe.rs:175`

```rust
result.clone()
```

**Issue:** When the result string is <= 80 chars, the entire string is cloned just for the `log::info!` format argument. This is a minor allocation but occurs on the hot path right after transcription completes.

**Severity:** Low
**Impact:** One small string clone. Negligible.

---

## Moderate (Settings/UI performance)

### P9. Settings JSON is read from disk repeatedly for each `read_saved_*` function at startup

**Files:**
- `src-tauri/src/lib.rs:75-86` — `read_saved_hotkey()`
- `src-tauri/src/lib.rs:90-110` — `read_saved_mode()`
- `src-tauri/src/lib.rs:331-350` — `read_saved_profile_id()`
- `src-tauri/src/lib.rs:354-377` — `read_saved_corrections()`
- `src-tauri/src/lib.rs:381-399` — `read_saved_all_caps()`
- `src-tauri/src/lib.rs:563-572` — `read_saved_mic()`
- `src-tauri/src/lib.rs:642-651` — `read_saved_model_id()`

**Issue:** Each function independently reads `settings.json` from disk and parses it. During startup, this file is read and parsed at least 5 times (hotkey, mode, profile_id, corrections, all_caps, mic, model_id). Each call does `read_to_string` + `serde_json::from_str`.

A single read-and-parse at startup, passing the parsed `serde_json::Value` to each extraction function, would reduce 5+ disk reads to 1.

**Severity:** Medium
**Impact:** ~5-10ms of redundant disk I/O at startup. Not on the recording hot path, but adds to app startup time.

---

### P10. Tauri commands that persist settings read-modify-write settings.json without caching

**Files:**
- `src-tauri/src/lib.rs:121-132` — `set_recording_mode()`
- `src-tauri/src/lib.rs:431-484` — `set_active_profile()`
- `src-tauri/src/lib.rs:497-533` — `save_corrections()`
- `src-tauri/src/lib.rs:537-559` — `set_all_caps()`
- `src-tauri/src/lib.rs:596-637` — `set_microphone()`
- `src-tauri/src/lib.rs:756-799` — `set_model()`

**Issue:** Every settings mutation reads the entire settings.json, parses it, modifies one key, serializes, and writes back. This is a standard pattern for small JSON files but involves ~3 syscalls (read, parse, write) per setting change. The tauri-plugin-store already exists in the frontend for the same purpose, creating two competing persistence mechanisms.

**Severity:** Low
**Impact:** Settings changes are infrequent (user-initiated). No latency impact on recording/transcription.

---

### P11. `DictionaryEditor` calls `onChange` (which triggers `save_corrections` IPC) on every blur event

**File:** `src/components/DictionaryEditor.tsx:46-48`

```typescript
function handleBlur() {
    onChange(rowsToRecord(rows));
}
```

**Issue:** Each time the user tabs out of any input field in the corrections dictionary, `onChange` fires, which calls `save_corrections` IPC to the backend. This rebuilds the entire `CorrectionsEngine` (compiling regexes for every correction rule) and writes to disk. For a dictionary with 12 rules, this compiles 12 regexes on every blur.

A debounce on `onChange` (e.g., 500ms) would batch rapid edits into a single save.

**Severity:** Low
**Impact:** Perceptible UI lag if the corrections dictionary grows large (50+ rules). Current default profile has 12 rules, so regex compilation is fast (~1ms total).

---

### P12. Hotkey handler code is fully duplicated between `setup()` and `rebind_hotkey()`

**Files:**
- `src-tauri/src/lib.rs:1068-1191` — setup handler (~120 lines)
- `src-tauri/src/lib.rs:153-277` — rebind handler (~120 lines)

**Issue:** This is not a runtime performance issue per se, but it doubles the binary code for the hotkey handler. More importantly, if one copy is optimized (e.g., to fix P7), the other must be updated too, creating a maintenance risk for performance regressions.

**Severity:** Low (code size, not runtime)
**Impact:** ~2-4KB of duplicated machine code in the binary.

---

## Low / Informational

### P13. `corrections::CorrectionsEngine::apply()` allocates a new String per rule

**File:** `src-tauri/src/corrections.rs:61-68`

```rust
pub fn apply(&self, text: &str) -> String {
    let mut result = text.to_string();
    for rule in &self.rules {
        let replaced = rule.pattern.replace_all(&result, rule.replacement.as_str());
        result = replaced.into_owned();
    }
    result
}
```

**Issue:** `Regex::replace_all` returns a `Cow<str>`. When no match occurs, it returns `Borrowed` (no allocation). When a match occurs, it returns `Owned` (one allocation). The `into_owned()` call forces allocation on every iteration regardless, because it converts the Cow to an owned String even when no match occurred.

Fix: check if the Cow is borrowed before calling `into_owned()`:
```rust
let replaced = rule.pattern.replace_all(&result, rule.replacement.as_str());
if let std::borrow::Cow::Owned(new) = replaced {
    result = new;
}
```

**Severity:** Low
**Impact:** For 12 rules where ~2 match, this saves ~10 unnecessary string clones of the transcription text. Each clone is tiny (typically < 200 bytes). Total savings: ~2KB allocation per transcription.

---

### P14. FrequencyBars uses imperative DOM manipulation inside React

**File:** `src/components/FrequencyBars.tsx:41-49`

**Issue:** The component creates 24 `<div>` elements via `document.createElement` and manages their styles imperatively in a `requestAnimationFrame` loop. This bypasses React's reconciliation entirely, which is intentional and correct for 60fps animation. The approach is performant — no React re-renders during animation.

**Severity:** Informational (this is good)
**Impact:** No issue. This is the correct approach for high-frequency visual updates.

---

### P15. Pill component registers 5 event listeners that allocate closures

**File:** `src/Pill.tsx:27-79`

**Issue:** Five `appWindow.listen()` calls register event listeners on mount. Each creates a Promise and closure. This is standard Tauri event handling and the listeners are properly cleaned up on unmount.

**Severity:** Informational (no issue)

---

### P16. `force_cpu_transcribe` loads the whisper model on every invocation

**File:** `src-tauri/src/lib.rs:919-952`

**Issue:** This is a test/debug command that loads the entire CPU model from disk, runs inference, and drops it. For a ~190MB model file, this is extremely slow. However, this is explicitly a "Phase 2 verification command" not used in production.

**Severity:** Informational
**Impact:** None in production — dev-only command.

---

### P17. `set_tray_state()` decodes PNG icon bytes on every state change

**File:** `src-tauri/src/tray.rs:36-37`

```rust
if let Ok(image) = tauri::image::Image::from_bytes(icon_bytes) {
    let _ = tray.set_icon(Some(image));
}
```

**Issue:** Every pipeline state change (idle -> recording -> processing -> idle) decodes the PNG icon from the embedded bytes. There are 3 icons, each small (~1-4KB PNG). The decode overhead is negligible but could be avoided by caching the decoded `Image` objects in managed state.

**Severity:** Low
**Impact:** ~0.1ms per state change. Three changes per recording cycle. Negligible.

---

## Summary of Recommended Fixes (Priority Order)

| Priority | Issue | Est. Impact | Effort |
|----------|-------|-------------|--------|
| 1 | P7: Clone entire audio buffer — use `mem::take()` instead | 5-50ms latency reduction | Small |
| 2 | P6: Cache GPU detection result at startup | 10-50ms per Model section open | Small |
| 3 | P3+P4: Pre-allocate mono downmix + resampling buffers in audio callback | Reduce ~200 allocs/sec in audio thread | Medium |
| 4 | P9: Single settings.json read at startup | 5-10ms startup improvement | Small |
| 5 | P5: Use `mem::take` instead of clone in flush() | Minor allocation savings | Trivial |
| 6 | P13: Avoid `into_owned()` when Cow is borrowed | ~10 string clones per transcription | Trivial |
| 7 | P11: Debounce corrections save | UI responsiveness with large dictionaries | Small |
