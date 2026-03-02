# Phase 7: Distribution - Research

**Researched:** 2026-03-01
**Domain:** Tauri 2 NSIS packaging, HTTP download with progress, GPU detection, first-run UX
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Drop the medium model entirely — only two models: Large v3 Turbo (GPU) and Small English (CPU)
- Remove medium from `list_models()` in lib.rs and `model_id_to_path()`
- Model cards show file size and quality label (e.g., "Large v3 Turbo — 398 MB — Best accuracy")
- On first launch with no model, show a first-run setup flow (blocks normal usage until a model is downloaded)
- Brief context: one sentence explaining that offline transcription needs a model file, then GPU detection result + model options + download
- Show both models with the recommended one highlighted based on GPU detection — user confirms which to download
- Users can download additional models later from the Model section in settings
- Standard NSIS installer, details (wizard vs one-click, shortcuts, silent support) are Claude's discretion
- Auto-start with Windows enabled by default (tauri-plugin-autostart already in dependencies)
- Installer must be under 5 MB (models excluded)

### Claude's Discretion
- First-run UI approach (dedicated window vs settings banner — pick best fit for existing code)
- Download progress display (progress bar style, speed/ETA, cancel button)
- Error recovery on failed downloads (auto-retry with resume vs manual retry — standard practice)
- Whether to prompt GPU users who chose small model to upgrade later
- SHA256 checksum validation (required per success criteria)
- Installer flow details (location chooser, shortcuts, silent install)
- Launch after install behavior

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| DIST-01 | On first run, app downloads the selected whisper model with a progress indicator | Tauri Channel API for streaming progress from Rust; reqwest streaming with bytes_stream(); SHA256 post-download validation |
| DIST-02 | App auto-detects GPU capability and recommends appropriate model size | `detect_gpu()` already implemented in transcribe.rs using NVML; first-run UI reads detection result and pre-selects recommendation |
| DIST-03 | App is packaged as a single Windows NSIS installer (models excluded, under 5 MB) | Tauri 2 `bundle.targets: ["nsis"]`; models are never bundled (downloaded at runtime); NSIS config in tauri.conf.json |
</phase_requirements>

---

## Summary

Phase 7 has three interlocking deliverables: (1) a first-run UI that gates app usage, detects GPU, shows model options, and downloads the selected model with progress, (2) SHA256 checksum validation on downloaded model files, and (3) a single NSIS installer under 5 MB. The existing codebase already has the hardest parts in place — `detect_gpu()` in transcribe.rs (NVML-based), the `ModelSelector` component with `downloaded` boolean, the Tauri event/channel infrastructure, and `tauri-plugin-autostart` already in Cargo.toml dependencies.

The download progress pipeline maps directly to an existing pattern in the project: Rust backend emits data, Tauri carries it to React frontend. The difference is we use Tauri `Channel<T>` instead of `emit_to()` events for download progress — Channel is the officially recommended mechanism for ordered streaming data (download progress, subprocess output). The reqwest crate (already transitively available through Tauri) handles HTTP streaming with `bytes_stream()`. SHA256 is computed incrementally per-chunk using the `sha2` crate during download rather than a second pass after completing.

The NSIS installer requires no third-party tools beyond `cargo tauri build`. Models are never placed in `bundle.resources` — they are always downloaded at runtime — so the installer contains only the binary + WebView2 bootstrapper, comfortably under 5 MB. Code signing is optional for internal use but required to eliminate SmartScreen warnings for end users; unsigned binaries pass Windows Defender antivirus scan but trigger SmartScreen on first download.

**Primary recommendation:** Implement the first-run setup as a conditional panel rendered within the existing `settings` window on startup — not a new window — gated by checking whether any model file exists. Download via `reqwest` with `bytes_stream()` piped through a `tauri::ipc::Channel` to the frontend. Build NSIS with `cargo tauri build --bundles nsis`.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `reqwest` | 0.12.x (transitive via Tauri) | HTTP download with streaming | De facto async HTTP client for Rust; `bytes_stream()` yields `Bytes` chunks for per-chunk progress |
| `sha2` | 0.10.x | SHA256 checksum computation | RustCrypto official crate; `Digest` trait; incremental `update()` per chunk |
| `tauri::ipc::Channel` | built-in Tauri 2 | Stream download progress to frontend | Officially recommended for ordered streaming data (docs.tauri.app); faster than event system |
| `futures-util` | 0.3.x (transitive) | `StreamExt` trait for `while let Some(chunk) = stream.next().await` | Required to consume `bytes_stream()` |
| `tauri-plugin-autostart` | 2.x | Register app in Windows startup | Already in Cargo.toml |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `tokio::fs` | via tokio (already present) | Async file writes during download | Write chunks to temp file, rename to final path on completion |
| `tauri::async_runtime::spawn` | built-in | Run download on async runtime without blocking UI | Same pattern as existing `run_pipeline()` |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `tauri::ipc::Channel` | `app.emit_to()` events | Events are lower throughput, unordered; Channel is correct for streaming progress |
| `sha2` crate | `ring` crate | Both work; `sha2` is RustCrypto standard, simpler API for this use case |
| Incremental SHA256 during download | SHA256 after download | Post-download approach requires reading full file again; incremental is single-pass |
| First-run in `settings` window | Separate dedicated window | Separate window needs new HTML entry point + Vite config; settings window already exists |

**Installation:**
```toml
# Add to Cargo.toml [dependencies]
sha2 = "0.10"
reqwest = { version = "0.12", features = ["stream"] }
```

Note: `futures-util` is already available as a transitive dependency. Verify before adding explicitly.

---

## Architecture Patterns

### Recommended Project Structure

```
src-tauri/src/
├── download.rs          # New: HTTP download + SHA256 + Channel progress
├── transcribe.rs        # Modify: remove medium model
├── lib.rs               # Modify: first-run gate, remove medium model, register download command
src/
├── components/
│   ├── FirstRun.tsx     # New: first-run setup flow (GPU detection result + model picker + download progress)
│   ├── ModelSelector.tsx  # Modify: add download button/progress for non-downloaded models
│   └── sections/
│       └── ModelSection.tsx  # Modify: wire download trigger for already-installed users
├── App.tsx              # Modify: check first-run state, show FirstRun before normal settings
```

### Pattern 1: Tauri Channel for Download Progress

**What:** Rust sends typed events to TypeScript via an `ipc::Channel` parameter injected into a Tauri command. The channel delivers events in order.
**When to use:** Any Rust command that needs to stream data to the frontend over time.

```rust
// Source: https://v2.tauri.app/develop/calling-frontend/
use tauri::ipc::Channel;
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
enum DownloadEvent {
    Started { url: String, total_bytes: u64 },
    Progress { downloaded_bytes: u64, total_bytes: u64 },
    Finished,
    Error { message: String },
}

#[tauri::command]
async fn download_model(
    model_id: String,
    on_event: Channel<DownloadEvent>,
) -> Result<(), String> {
    // ... download implementation ...
}
```

```typescript
// Source: https://v2.tauri.app/develop/calling-frontend/
import { invoke, Channel } from '@tauri-apps/api/core';

const onEvent = new Channel<DownloadEvent>();
onEvent.onmessage = (message) => {
    if (message.event === 'progress') {
        const pct = (message.data.downloadedBytes / message.data.totalBytes) * 100;
        setProgress(pct);
    }
};

await invoke('download_model', { modelId: 'large-v3-turbo', onEvent });
```

### Pattern 2: Incremental SHA256 During Streaming Download

**What:** Feed each downloaded chunk into a running SHA256 digest. Compare final digest to expected value before moving file to final location.
**When to use:** Any file download requiring integrity verification.

```rust
// Source: sha2 crate docs + RustCrypto conventions
use sha2::{Sha256, Digest};
use futures_util::StreamExt;

async fn download_with_checksum(
    url: &str,
    dest: &std::path::Path,
    expected_sha256: &str,
    on_event: &Channel<DownloadEvent>,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let response = client.get(url).send().await.map_err(|e| e.to_string())?;
    let total = response.content_length().unwrap_or(0);

    on_event.send(DownloadEvent::Started {
        url: url.to_string(),
        total_bytes: total,
    }).ok();

    let mut hasher = Sha256::new();
    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    // Write to temp path first — never leave a partial file at dest
    let tmp_path = dest.with_extension("tmp");
    let mut file = tokio::fs::File::create(&tmp_path)
        .await
        .map_err(|e| e.to_string())?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        hasher.update(&chunk);
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
            .await
            .map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;
        on_event.send(DownloadEvent::Progress {
            downloaded_bytes: downloaded,
            total_bytes: total,
        }).ok();
    }

    // Verify checksum before committing
    let actual = format!("{:x}", hasher.finalize());
    if actual != expected_sha256 {
        let _ = tokio::fs::remove_file(&tmp_path).await;
        return Err(format!("Checksum mismatch: expected {expected_sha256}, got {actual}"));
    }

    // Atomic rename — file appears at dest only if complete and valid
    tokio::fs::rename(&tmp_path, dest)
        .await
        .map_err(|e| e.to_string())?;

    on_event.send(DownloadEvent::Finished).ok();
    Ok(())
}
```

### Pattern 3: First-Run Gate in App.tsx

**What:** On startup, invoke `check_first_run` Tauri command. If no model file exists, render `<FirstRun>` instead of normal settings UI.
**When to use:** Blocking setup flows where app must not proceed until prerequisite is met.

```typescript
// Rust: check_first_run command returns { needs_setup: bool, gpu_detected: bool, recommended_model: string }
// Frontend: gate on this before showing normal settings UI

const [firstRun, setFirstRun] = useState<FirstRunStatus | null>(null);

useEffect(() => {
    invoke<FirstRunStatus>('check_first_run').then(setFirstRun);
}, []);

if (firstRun === null) return <LoadingScreen />;
if (firstRun.needsSetup) {
    return <FirstRun status={firstRun} onComplete={() => setFirstRun({ needsSetup: false })} />;
}
// Normal settings UI...
```

### Pattern 4: NSIS Bundle Configuration

**What:** Configure Tauri to build NSIS-only (not WiX), with currentUser install scope to avoid UAC prompt, and auto-start on Windows default on.
**When to use:** Windows distribution target.

```json
// tauri.conf.json
{
  "bundle": {
    "active": true,
    "targets": ["nsis"],
    "windows": {
      "nsis": {
        "installMode": "currentUser",
        "displayLanguageSelector": false,
        "startMenuFolder": "VoiceType"
      }
    }
  }
}
```

### Anti-Patterns to Avoid

- **Writing model file directly to final path during download:** If download is interrupted, leaves a partial corrupted file that looks valid to `path.exists()`. Always write to `.tmp` and rename atomically on success.
- **SHA256 in a second pass after download:** Requires reading the whole file again. Compute incrementally per-chunk.
- **Blocking the Tauri main thread for download:** Download is long-running; always spawn on `tauri::async_runtime`.
- **Adding model files to `bundle.resources`:** Models are 190–574 MB. The context decided explicitly against bundling. Never add them.
- **Creating a new Tauri window for first-run:** The existing `settings` window is sufficient. A new window requires new HTML entry point, new `tauri.conf.json` window definition, new capability entries.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| HTTP streaming download | Manual `TcpStream` or `WinHTTP` | `reqwest` + `bytes_stream()` | Handles redirects, TLS, chunked encoding, error recovery |
| SHA256 computation | Custom bit-rotation | `sha2` crate (RustCrypto) | Constant-time, audited, correct padding |
| Streaming to frontend | Custom WebSocket | `tauri::ipc::Channel` | Built-in, ordered, serialized, no extra deps |
| Windows autostart registry | Manual `HKCU\Run` writes | `tauri-plugin-autostart` | Already in Cargo.toml; handles enable/disable/query |
| NSIS script | Custom `.nsi` from scratch | `cargo tauri build --bundles nsis` | Tauri generates a complete, signed NSIS script |

**Key insight:** Every capability needed for this phase already has a solution in the project's existing dependency tree or the Tauri plugin ecosystem. New code is glue logic only.

---

## Common Pitfalls

### Pitfall 1: Partial Download File Left at Final Path
**What goes wrong:** Download fails mid-stream. `path.exists()` returns true. Next startup skips download and tries to load a corrupt model file. `load_whisper_context()` panics or returns a cryptic error.
**Why it happens:** Writing directly to the final destination path during download.
**How to avoid:** Always write to `dest.with_extension("tmp")`. Only `rename()` to final path after checksum passes.
**Warning signs:** `WhisperContext::new_with_params` returns error mentioning "magic" or "format" on a file that "exists".

### Pitfall 2: Download Blocks Startup / App Hangs
**What goes wrong:** Model download runs synchronously on the setup thread, blocking Tauri's `setup()` closure. The app window never appears.
**Why it happens:** Using `std::thread::spawn` + `rx.recv()` blocking inside setup, or calling async download without `tauri::async_runtime::spawn`.
**How to avoid:** First-run check happens in setup — it only detects that a model is missing and sets a flag. The download itself is triggered by the frontend via a `download_model` Tauri command, which runs async.
**Warning signs:** App window never opens, or opens but is unresponsive.

### Pitfall 3: medium Model Still Referenced After Removal
**What goes wrong:** Removing medium from `list_models()` but missing the match arm in `model_id_to_path()`, or vice versa. Users with saved `"medium"` in settings.json cause a panic on startup.
**Why it happens:** Medium model is referenced in multiple places: `list_models()`, `model_id_to_path()`, and potentially persisted settings.
**How to avoid:** Remove from both `list_models()` and `model_id_to_path()`. In `read_saved_model_id()`, treat unknown model IDs as None (fall through to auto-detect). Test startup with `settings.json` containing `whisper_model_id: "medium"`.
**Warning signs:** Startup log showing "Unknown model id: medium" error.

### Pitfall 4: NSIS Plugin Signing False Positive
**What goes wrong:** Tauri bundles NSIS plugins that are not code-signed even when the binary is signed. Some antivirus programs flag these unsigned NSIS plugins.
**Why it happens:** Known Tauri 2 issue (#11673). The bundler signs the main binary but not the plugin DLLs embedded in the NSIS installer.
**How to avoid:** Test installer on clean Windows 10 VM with Windows Defender (not third-party AV). Windows Defender specifically is what the success criteria requires. Document the issue; EV certificate reduces (but may not eliminate) false positives from other AV.
**Warning signs:** VirusTotal scan showing detections from AV engines other than Defender; Defender specifically should be clean.

### Pitfall 5: SmartScreen Warning ≠ Defender Detection
**What goes wrong:** Confusing SmartScreen "Unknown publisher" warning with Windows Defender malware detection. Success criteria says "passes Windows Defender scan" — SmartScreen is a separate system.
**Why it happens:** SmartScreen appears on first download/run of any low-reputation binary, signed or not. It is NOT an antivirus detection.
**How to avoid:** Test with `MpCmdRun.exe -Scan -ScanType 3 -File <installer.exe>` for a true Defender scan. SmartScreen warnings are expected without an EV certificate; document this separately.
**Warning signs:** Seeing "Windows protected your PC" dialog and calling it a "Defender detection".

### Pitfall 6: Content-Length Missing from HuggingFace Response
**What goes wrong:** `response.content_length()` returns `None` for HuggingFace URLs. Progress percentage cannot be computed.
**Why it happens:** HuggingFace uses LFS redirect chains and may return chunked transfer encoding without `Content-Length` on the final server.
**How to avoid:** Use hardcoded expected sizes as fallback (large-v3-turbo-q5_0: 574 MB, small.en-q5_1: 190 MB). Show indeterminate progress if `content_length()` returns None, fall back to bytes-downloaded counter.
**Warning signs:** `content_length()` returning None in logs; progress bar stuck at 0%.

---

## Code Examples

### Model File SHA256 Checksums (Verified from HuggingFace)

```rust
// Source: https://huggingface.co/ggerganov/whisper.cpp/blob/main/ggml-large-v3-turbo-q5_0.bin
// Source: https://huggingface.co/ggerganov/whisper.cpp/blob/main/ggml-small.en-q5_1.bin

pub fn model_info(model_id: &str) -> Option<(&'static str, &'static str, u64)> {
    // Returns (filename, sha256, expected_size_bytes)
    match model_id {
        "large-v3-turbo" => Some((
            "ggml-large-v3-turbo-q5_0.bin",
            "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2",
            601_882_624, // 574 MB approx
        )),
        "small-en" => Some((
            "ggml-small.en-q5_1.bin",
            "bfdff4894dcb76bbf647d56263ea2a96645423f1669176f4844a1bf8e478ad30",
            199_229_440, // 190 MB approx
        )),
        _ => None,
    }
}
```

Note: SHA256 values are fetched directly from HuggingFace file pages (HIGH confidence). Byte sizes are approximate — use these for progress fallback only, not as exact validation.

### Updated list_models() (Medium Removed)

```rust
// Modify lib.rs list_models() — remove medium, add file_size_mb display field
#[cfg(feature = "whisper")]
#[tauri::command]
fn list_models() -> Result<Vec<ModelInfo>, String> {
    use crate::transcribe::{detect_gpu, models_dir, ModelMode};
    let gpu_mode = matches!(detect_gpu(), ModelMode::Gpu);
    let dir = models_dir();

    Ok(vec![
        ModelInfo {
            id: "large-v3-turbo".to_string(),
            name: "Large v3 Turbo".to_string(),
            description: "Best accuracy — 574 MB — requires NVIDIA GPU".to_string(),
            recommended: gpu_mode,
            downloaded: dir.join("ggml-large-v3-turbo-q5_0.bin").exists(),
        },
        ModelInfo {
            id: "small-en".to_string(),
            name: "Small (English)".to_string(),
            description: "Fastest — 190 MB — works on any CPU".to_string(),
            recommended: !gpu_mode,
            downloaded: dir.join("ggml-small.en-q5_1.bin").exists(),
        },
    ])
}
```

### check_first_run Tauri Command

```rust
#[derive(serde::Serialize)]
struct FirstRunStatus {
    needs_setup: bool,
    gpu_detected: bool,
    recommended_model: String,
}

#[cfg(feature = "whisper")]
#[tauri::command]
fn check_first_run() -> FirstRunStatus {
    use crate::transcribe::{detect_gpu, models_dir, ModelMode};
    let gpu_mode = matches!(detect_gpu(), ModelMode::Gpu);
    let dir = models_dir();

    let large_exists = dir.join("ggml-large-v3-turbo-q5_0.bin").exists();
    let small_exists = dir.join("ggml-small.en-q5_1.bin").exists();
    let any_model_exists = large_exists || small_exists;

    FirstRunStatus {
        needs_setup: !any_model_exists,
        gpu_detected: gpu_mode,
        recommended_model: if gpu_mode {
            "large-v3-turbo".to_string()
        } else {
            "small-en".to_string()
        },
    }
}
```

### tauri-plugin-autostart Enable on First Run

```rust
// Source: https://v2.tauri.app/plugin/autostart/
// In lib.rs setup() — enable autostart by default after first-run model download
// Call from frontend after successful download via invoke('enable_autostart')

#[tauri::command]
async fn enable_autostart(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let autostart = app.autolaunch();
    autostart.enable().map_err(|e| e.to_string())
}
```

### NSIS Bundle Configuration

```json
// tauri.conf.json — update bundle section
{
  "bundle": {
    "active": true,
    "targets": ["nsis"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "windows": {
      "nsis": {
        "installMode": "currentUser",
        "displayLanguageSelector": false,
        "startMenuFolder": "VoiceType"
      }
    }
  }
}
```

Build command:
```bash
cargo tauri build --bundles nsis
```

Output location: `src-tauri/target/release/bundle/nsis/VoiceType_0.1.0_x64-setup.exe`

### Frontend Download Progress Component Pattern

```typescript
// FirstRun.tsx — simplified structure
import { invoke, Channel } from '@tauri-apps/api/core';

type DownloadEvent =
    | { event: 'started'; data: { url: string; totalBytes: number } }
    | { event: 'progress'; data: { downloadedBytes: number; totalBytes: number } }
    | { event: 'finished' }
    | { event: 'error'; data: { message: string } };

async function startDownload(modelId: string, setProgress: (n: number) => void) {
    const onEvent = new Channel<DownloadEvent>();
    onEvent.onmessage = (msg) => {
        if (msg.event === 'progress') {
            const pct = msg.data.totalBytes > 0
                ? (msg.data.downloadedBytes / msg.data.totalBytes) * 100
                : -1; // indeterminate
            setProgress(pct);
        }
    };
    await invoke('download_model', { modelId, onEvent });
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Tauri events for streaming | `tauri::ipc::Channel` | Tauri 2 stable (Oct 2024) | Channel is ordered, faster, recommended for streaming; events are for simple fire-and-forget |
| WiX installer (.msi) as default | NSIS installer as default | Tauri 2 | NSIS produces smaller installers and has better customization for currentUser install without UAC |
| `targets: "all"` | `targets: ["nsis"]` | Tauri 2 | Use specific target list for production distribution; "all" builds multiple formats unnecessarily |

**Deprecated/outdated:**
- Tauri v1 WiX-first approach: NSIS is preferred in Tauri 2 for Windows distribution.
- `emit_to()` for download progress: Channel API should be used instead.

---

## Open Questions

1. **Exact byte sizes for progress fallback**
   - What we know: HuggingFace shows 574 MB (large-v3-turbo-q5_0) and 190 MB (small.en-q5_1)
   - What's unclear: HuggingFace's reported sizes may differ from actual byte count due to LFS metadata. The content_length in HTTP response is authoritative.
   - Recommendation: Use `response.content_length()` as primary; hardcode approximate bytes only as UI fallback label, not for validation.

2. **Code signing certificate**
   - What we know: Unsigned binaries pass Windows Defender AV scan but trigger SmartScreen "Unknown Publisher" on first download. EV certificate eliminates SmartScreen immediately; OV certificate requires reputation building. Success criteria says "passes Windows Defender scan" — not "no SmartScreen".
   - What's unclear: Whether this app will be signed for v1 distribution; user hasn't specified.
   - Recommendation: Plan 07-03 documents the signing process but signing itself is out of scope for the implementation task. The Defender clean scan success criterion is testable without a certificate.

3. **Cancel/resume on download failure**
   - What we know: HTTP range requests (Resume) require server support; HuggingFace LFS supports `Range` headers.
   - What's unclear: CONTEXT.md delegates error recovery detail to Claude's discretion.
   - Recommendation: Implement simple cancel (abort the reqwest request, delete `.tmp` file) and manual retry (re-invoke `download_model`). Resume adds complexity and is not required by success criteria. Auto-retry once on network error before surfacing error to user.

---

## Sources

### Primary (HIGH confidence)
- https://v2.tauri.app/develop/calling-frontend/ — Channel API, download event pattern, TypeScript consumer code
- https://v2.tauri.app/distribute/windows-installer/ — NSIS configuration options, installMode, hooks
- https://v2.tauri.app/distribute/sign/windows/ — Code signing configuration, SmartScreen behavior
- https://v2.tauri.app/plugin/autostart/ — tauri-plugin-autostart API, permissions
- https://huggingface.co/ggerganov/whisper.cpp/blob/main/ggml-large-v3-turbo-q5_0.bin — SHA256: 394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2
- https://huggingface.co/ggerganov/whisper.cpp/blob/main/ggml-small.en-q5_1.bin — SHA256: bfdff4894dcb76bbf647d56263ea2a96645423f1669176f4844a1bf8e478ad30

### Secondary (MEDIUM confidence)
- https://docs.rs/tauri-utils/latest/tauri_utils/config/struct.NsisConfig.html — NsisConfig struct fields, installMode values
- https://gist.github.com/Tapanhaz/096e299bf060607b572d700e89a62529 — reqwest streaming download pattern with futures_util

### Tertiary (LOW confidence)
- GitHub issue tauri-apps/tauri#11673 — NSIS plugin signing gap; flagged LOW because unresolved issue, not official docs
- WebSearch: HuggingFace `content_length` behavior on LFS redirect chains — unverified, flagged for runtime testing

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — Tauri Channel API and reqwest are verified against official docs; sha2 is RustCrypto standard
- Architecture: HIGH — first-run gate pattern is straightforward; existing code (ModelSelector, list_models, detect_gpu) is all reusable; no new Tauri window needed
- SHA256 checksums: HIGH — fetched directly from HuggingFace file pages
- NSIS configuration: HIGH — tauri.conf.json schema verified via docs.rs
- Code signing / SmartScreen: MEDIUM — behavior documented by Tauri, SmartScreen reputation building is Microsoft-controlled
- Pitfalls: MEDIUM — based on GitHub issues + community patterns, not all verified against official docs

**Research date:** 2026-03-01
**Valid until:** 2026-04-01 (Tauri 2 stable; HuggingFace model checksums valid until model files change)
