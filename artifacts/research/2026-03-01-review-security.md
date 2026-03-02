# Security Review: VoiceType Desktop App

**Date:** 2026-03-01
**Scope:** Full codebase -- Tauri v2 desktop app (Rust backend + React/TypeScript frontend)
**Reviewer:** Claude Opus 4.6

## Summary

The codebase has a small attack surface by design: it is an offline desktop app with no network services, no user accounts, no database, and no remote API calls except downloading model files from a hardcoded HuggingFace URL. The frontend is served from local bundled assets, not a remote origin.

**3 findings identified.** 1 high severity, 2 medium severity. No critical vulnerabilities found.

The app does not have: command injection, SQL injection, SSRF, unsafe deserialization from untrusted sources, exposed secrets, or authentication/authorization flaws (no auth system exists). The `unsafe` blocks are narrow and well-justified.

---

## Findings

### HIGH-1: Content Security Policy Disabled (`csp: null`)

**Severity:** HIGH
**File:** `src-tauri/tauri.conf.json:35`
**OWASP Category:** A03:2021 Injection (XSS)

```json
"security": {
  "csp": null
}
```

**Issue:** The Content Security Policy is explicitly set to `null`, which disables all CSP protections. In Tauri v2, setting `"csp": null` means no CSP header is injected into the webview. This removes a defense-in-depth layer against XSS.

**Risk assessment:** The practical risk is **moderate rather than critical** because:
- The frontend loads only from local bundled assets (no remote HTML).
- All user-facing text is rendered via React (which escapes by default).
- There is no `dangerouslySetInnerHTML`, no `innerHTML` usage in React components, and no user-controlled HTML rendering.
- The pill window's `FrequencyBars.tsx` uses `document.createElement` to create bar elements, but only sets numeric style properties -- no user-controlled content is inserted into DOM.
- There is no navigation to external URLs or dynamic script loading.

However, CSP is a defense-in-depth measure. If any future code introduces a DOM-based XSS vector (e.g., rendering user-supplied text from transcription results in the UI without escaping), the absence of CSP means there is no secondary barrier preventing script execution.

**Additionally:** `withGlobalTauri: true` (line 33) exposes the full Tauri IPC bridge (`window.__TAURI__`) to the webview JavaScript context. Combined with no CSP, if an attacker somehow injects JavaScript (e.g., via a malicious browser extension that can access webview contexts, or a future code change), they would have access to all registered Tauri commands.

**Recommendation:** Set a restrictive CSP. Since this app loads only local assets and does one external HTTP request (model download from Rust, not from JS):

```json
"security": {
  "csp": "default-src 'self'; style-src 'self' 'unsafe-inline'; script-src 'self'; connect-src ipc: http://ipc.localhost"
}
```

Adjust `style-src 'unsafe-inline'` if Tailwind/inline styles require it. This prevents any injected script from loading external resources or executing inline scripts.

---

### MED-1: `transcribe_test_file` and `force_cpu_transcribe` Accept Arbitrary File Paths

**Severity:** MEDIUM
**Files:**
- `src-tauri/src/lib.rs:868` (`transcribe_test_file`)
- `src-tauri/src/lib.rs:919` (`force_cpu_transcribe`)
**OWASP Category:** A01:2021 Broken Access Control (Path Traversal / Arbitrary File Read)

```rust
#[tauri::command]
async fn transcribe_test_file(
    app: tauri::AppHandle,
    path: String,    // <-- Arbitrary user-controlled path
) -> Result<String, String> {
    // ...
    let (audio_f32, _sample_rate) = read_wav_to_f32(&path)?;
    // ...
}

#[tauri::command]
async fn force_cpu_transcribe(path: String) -> Result<String, String> {
    // ...
    let (audio_f32, _sample_rate) = read_wav_to_f32(&path)?;
    // ...
}
```

**Issue:** Both commands accept a `path: String` parameter from the frontend and pass it directly to `hound::WavReader::open()` via `read_wav_to_f32()` with no path validation or sandboxing. A caller can specify any file path on the filesystem.

**Risk assessment:** The practical risk is **moderate** because:
- Tauri IPC commands are only callable from the local webview, not from external sources.
- `hound::WavReader::open()` will attempt to parse any file as WAV. Most non-WAV files will fail parsing and return an error, so this is not a general-purpose file read primitive -- the attacker cannot exfiltrate arbitrary file content.
- However, these appear to be development/testing commands that are still registered in the production `invoke_handler` (`lib.rs:994-997`). They expose unnecessary attack surface.

**Recommendation:**
1. Remove `transcribe_test_file` and `force_cpu_transcribe` from the production `invoke_handler`, or gate them behind a `#[cfg(debug_assertions)]` flag.
2. If they must remain, validate that `path` is within the app data directory:
```rust
let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
let canonical = std::fs::canonicalize(&path).map_err(|e| e.to_string())?;
if !canonical.starts_with(&data_dir) {
    return Err("Path must be within app data directory".to_string());
}
```

---

### MED-2: `unsafe impl Sync` for Types Wrapping `cpal::Stream`

**Severity:** MEDIUM (correctness/soundness, not exploitable remotely)
**File:** `src-tauri/src/audio.rs:95-106`
**Category:** Memory Safety (Rust Soundness)

```rust
// Line 95
unsafe impl Sync for AudioCapture {}

// Line 106
unsafe impl Sync for AudioCaptureMutex {}
```

**Issue:** `cpal::Stream` is `Send` but not `Sync`. The `unsafe impl Sync` on `AudioCapture` asserts to the compiler that `&AudioCapture` can be shared across threads safely. The safety argument in the comment is that `_stream` is never accessed through a shared reference -- only the `Arc<AtomicBool>` and `Arc<Mutex<...>>` fields are.

**Risk assessment:** The current code is **likely sound** because:
- The `_stream` field is indeed never read after construction (it exists only to keep the stream alive via RAII).
- All cross-thread access goes through the `Arc`-wrapped atomic/mutex fields.
- The outer `AudioCaptureMutex` wraps the whole thing in a `Mutex`, so replacement operations are exclusive.

However, `unsafe impl Sync for AudioCaptureMutex` on line 106 is **redundant and misleading** -- `Mutex<AudioCapture>` already gets `Sync` automatically if `AudioCapture: Send`, which it is (since `cpal::Stream: Send`). Since `AudioCapture` already has `unsafe impl Sync`, the `Mutex<AudioCapture>` is automatically `Sync`. The second `unsafe impl` is unnecessary.

The real concern is maintainability: if a future developer adds a method that accesses `_stream` through a shared reference (e.g., `&self`), the soundness guarantee breaks silently.

**Recommendation:** No immediate action required, but consider:
1. Remove the redundant `unsafe impl Sync for AudioCaptureMutex`.
2. Add a `#[allow(dead_code)]` annotation on `_stream` to make the "keep alive only" intent explicit.
3. Optionally, wrap `_stream` in a newtype that does not expose `&cpal::Stream`.

---

## Areas Reviewed (No Issues Found)

### IPC Command Surface
All 17 registered Tauri commands were reviewed. Commands use Tauri's type-safe deserialization (serde). No command performs shell execution, SQL queries, or processes untrusted HTML/JS. The `download_model` command uses a hardcoded allowlist of model IDs (`model_info()` returns `None` for unknown IDs), preventing SSRF or download of arbitrary URLs.

### Download Security
`download.rs` downloads model files only from `https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{filename}` where `filename` is one of two hardcoded strings. SHA-256 checksums are validated after download. Temporary files are cleaned up on checksum failure. The download URL construction does not interpolate user input.

### Input Injection via Clipboard
`inject.rs` uses `arboard` (clipboard) + `enigo` (keyboard simulation) to paste transcribed text. The injected text originates from whisper transcription output, not from external/untrusted user input. The text is only formatted (trimmed, optionally uppercased) and corrections-applied before injection. There is no vector for injecting control characters that would execute commands -- `Ctrl+V` paste into the focused application is the intended behavior.

### Corrections Engine (Regex)
`corrections.rs` uses `regex::escape()` on all user-provided correction keys before embedding them in regex patterns. This prevents ReDoS (Regular Expression Denial of Service) from user-supplied patterns. The regex is constructed as `(?i)\b{escaped}\b` which is safe.

### Frontend XSS
All React components use JSX expressions (`{variable}`) which are auto-escaped by React. No `dangerouslySetInnerHTML` usage found. No `innerHTML` manipulation with user-controlled content. The `FrequencyBars` component creates DOM elements programmatically but only sets numeric CSS properties.

### Secrets and Credentials
No API keys, passwords, tokens, or credentials found in the codebase. No `.env` files. The app is fully offline (no authentication, no remote API calls other than model download).

### File System Access
Settings are persisted to `settings.json` in the Tauri app data directory (`%APPDATA%/VoiceType/`). WAV files are saved to a subdirectory of app data. Model files are stored in `%APPDATA%/VoiceType/models/`. No user-controlled path construction for settings or model storage (except MED-1 test commands).

### Network Security
All HTTP requests use HTTPS (reqwest with `https://huggingface.co`). No HTTP endpoints are opened. No WebSocket servers. The app does not listen on any network port.

### Tauri Capabilities (Permissions)
The capability files (`default.json`, `desktop.json`) grant minimal permissions: window show/hide/position, store read/write, autostart, and global shortcut. No file system, shell, or HTTP permissions are granted to the webview. The Tauri v2 capability system appropriately restricts what the frontend can do.

### NSIS Installer
Configured with `installMode: currentUser` which does not require admin elevation. This is the correct security posture for a user-space application.

### Clipboard Exposure Window
`inject.rs:30-39` temporarily places transcription text on the clipboard for ~80ms. Any clipboard monitoring software running on the system could capture this. This is inherent to the clipboard-based injection approach and is the standard pattern for text injection on Windows. No practical alternative exists without platform-specific `SendInput`/`WM_CHAR` APIs.

### Mutex Poisoning
Multiple locations use `.lock().unwrap()` on Mutex guards (throughout `lib.rs`, `pipeline.rs`). If any thread panics while holding a Mutex, subsequent lock attempts will panic (poison), potentially crashing the application. The `vad.rs:156` correctly uses `.unwrap_or_else(|e| e.into_inner())` to recover from poisoned mutexes. This is a reliability concern, not a security vulnerability.
