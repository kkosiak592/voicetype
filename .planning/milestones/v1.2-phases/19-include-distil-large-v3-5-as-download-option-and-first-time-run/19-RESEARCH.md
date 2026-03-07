# Phase 19: Include distil-large-v3.5 as Download Option and First-Time Run - Research

**Researched:** 2026-03-03
**Domain:** Whisper GGML model integration — download.rs, FirstRun.tsx, list_models IPC, check_first_run IPC
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- Add distil-large-v3.5 alongside existing models — do NOT remove large-v3-turbo or any other model
- Final lineup: distil-large-v3.5, large-v3-turbo, parakeet-tdt, small-en (4 models total)
- No migration handling for existing users (still in dev mode)
- Ship fp16 GGML as-is from HuggingFace (1.52 GB) — no quantization
- Different HF repo than current models: `distil-whisper/distil-large-v3.5-ggml` (current is `ggerganov/whisper.cpp`)
- Card name: "Distil Large v3.5"
- Show to all users regardless of GPU detection (works on CPU, just slower)
- Not GPU-only — `gpuOnly: false` in FirstRun.tsx
- Show all applicable models (up to 4 cards for GPU users)
- Keep parakeet-tdt as recommended for GPU users (unchanged)
- Keep small-en as recommended for CPU-only users (unchanged)
- distil-v3.5 appears as a regular (non-recommended) option for now

### Claude's Discretion

- Local filename for the downloaded distil-v3.5 GGML file (upstream is generic `ggml-model.bin`)
- Requirement text on the first-run card (e.g., "GPU recommended" vs "Works on any hardware")
- How to handle the different HF repo URL in download.rs (per-model URL in model_info vs separate function)
- Card quality/description text

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope
</user_constraints>

## Summary

Phase 19 adds distil-large-v3.5 (fp16 GGML, 1.52 GB) as a fourth downloadable model. The model comes from a different HuggingFace repo than the current Whisper models (`distil-whisper/distil-large-v3.5-ggml` vs `ggerganov/whisper.cpp`), so `download_url()` in `download.rs` must be changed from a hardcoded URL builder to a per-model URL lookup. All other plumbing (streaming download, SHA256 validation, progress events) is reused as-is.

Five code locations require changes: `download.rs` (model_info + URL routing), `lib.rs` (model_id_to_path, list_models, check_first_run), and `FirstRun.tsx` (MODELS array). The `ModelSelector.tsx` and `App.tsx` require no changes — App.tsx's `onComplete` handler already calls `invoke('set_model', { modelId: downloadedModelId })` for all non-Parakeet models, which correctly activates distil-v3.5 after first-run download.

The only blocking unknown is the SHA256 checksum for `ggml-model.bin`: HuggingFace does not publish it on the repo page. It must be computed by downloading the file once before implementation can be completed.

**Primary recommendation:** Refactor `download_url()` to a per-model URL lookup by embedding the URL in `model_info()`, then add distil-v3.5 as a standard Whisper model entry across all five locations.

## Standard Stack

No new libraries are needed. All changes are within existing patterns.

### Core (already in use)
| Component | Version | Purpose | Status |
|-----------|---------|---------|--------|
| `download.rs` | existing | Streaming download + SHA256 validation | Extend only |
| `whisper-rs` | 0.15 | GGML model loading | No change — distil GGML loads identically |
| `reqwest` | existing | HTTP client for downloads | No change |
| `sha2` | existing | SHA256 checksum validation | No change |
| `tauri::ipc::Channel` | existing | Streaming progress events | No change |
| React + Tailwind | existing | FirstRun card UI | Extend only |

### No New Dependencies
This phase adds zero new Cargo crates or npm packages.

## Architecture Patterns

### Current Model Registration Pattern

There are **five places** where a Whisper model must be registered. All five must be updated:

**1. `download.rs` — `model_info()`**
Returns `(filename, sha256_hex, size_bytes)` for a model_id. Currently hardcoded to two models:
```rust
// src-tauri/src/download.rs (current)
fn model_info(model_id: &str) -> Option<(&'static str, &'static str, u64)> {
    match model_id {
        "large-v3-turbo" => Some(("ggml-large-v3-turbo-q5_0.bin", "394221...", 601_882_624)),
        "small-en" => Some(("ggml-small.en-q5_1.bin", "bfdff4...", 199_229_440)),
        _ => None,
    }
}
```
Must be extended with distil-v3.5. The SHA256 must be computed by downloading the file.

**2. `download.rs` — `download_url()`**
Currently hardcodes the ggerganov repo for all models:
```rust
fn download_url(filename: &str) -> String {
    format!("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}", filename)
}
```
Distil-v3.5 lives in a different repo. This function must become model-aware.

Recommended approach: embed the full URL in `model_info()` as a fourth field, then remove `download_url()`. This keeps all model metadata in one place with one source of truth.

**3. `lib.rs` — `model_id_to_path()`**
Maps model_id to the local file path for `set_model()`:
```rust
fn model_id_to_path(model_id: &str) -> Result<PathBuf, String> {
    let filename = match model_id {
        "large-v3-turbo" => "ggml-large-v3-turbo-q5_0.bin",
        "small-en" => "ggml-small.en-q5_1.bin",
        _ => return Err(format!("Unknown model id: {}", model_id)),
    };
    Ok(models_dir().join(filename))
}
```
Must add `"distil-large-v3.5" => "ggml-distil-large-v3.5.bin"`.

**4. `lib.rs` — `list_models()`**
Returns `Vec<ModelInfo>` to the frontend settings ModelSelector. Must push a distil-v3.5 entry:
```rust
models.push(ModelInfo {
    id: "distil-large-v3.5".to_string(),
    name: "Distil Large v3.5".to_string(),
    description: "High accuracy — 1.52 GB — GPU accelerated when available".to_string(),
    recommended: false,
    downloaded: dir.join("ggml-distil-large-v3.5.bin").exists(),
});
```

**5. `lib.rs` — `check_first_run()`**
The `needs_setup` flag must include distil-v3.5 as a valid installed model — otherwise users who downloaded distil-v3.5 would still see the first-run screen on next launch:
```rust
let distil_v35_exists = dir.join("ggml-distil-large-v3.5.bin").exists();
// ...
needs_setup: !large_exists && !small_exists && !parakeet_fp32_exists && !distil_v35_exists,
```

**6. `FirstRun.tsx` — MODELS array**
Add a new card definition. Since distil-v3.5 is not GPU-only (`gpuOnly: false`), all users see it:
```typescript
{
  id: 'distil-large-v3.5',
  name: 'Distil Large v3.5',
  size: '1.52 GB',
  quality: 'High accuracy, fast',
  requirement: 'GPU recommended, works on any hardware',
  gpuOnly: false,
}
```
The `handleDownload()` function in FirstRun.tsx routes non-Parakeet models through `invoke('download_model', { modelId, onEvent })` — distil-v3.5 uses this path with no changes to the routing logic.

### Files That Do NOT Change

- **`ModelSelector.tsx`** — special-cases only `parakeet-tdt-v2-fp32`. All other Whisper models use the generic download path via `invoke('download_model', ...)`. Distil-v3.5 automatically falls through to the generic path.
- **`App.tsx`** — the `onComplete` handler already calls `invoke('set_model', { modelId: downloadedModelId })` for all models. When distil-v3.5 download completes in FirstRun.tsx, `onComplete('distil-large-v3.5')` is called, which correctly activates the model via `set_model`. No change needed.
- **`transcribe.rs`** — distil GGML loads via the identical `WhisperContext::new_with_params()` call. Architecture differences are encoded in the GGML binary metadata.

### Local Filename Decision (Claude's Discretion)

The upstream file is `ggml-model.bin` — a generic name that would collide if multiple distil GGML repos used the same naming convention. **Use `ggml-distil-large-v3.5.bin` as the local filename.** This is unique, human-readable, and matches the style of existing files (`ggml-large-v3-turbo-q5_0.bin`, `ggml-small.en-q5_1.bin`).

### Requirement Text Decision (Claude's Discretion)

First-run card requirement text: **"GPU recommended, works on any hardware"**. Accurate (distil-v3.5 runs on CPU but benefits from GPU acceleration) and consistent with the locked decision "not GPU-only."

### Description Text for ModelSelector (Claude's Discretion)

`list_models()` description: **"High accuracy — 1.52 GB — GPU accelerated when available"**. Matches the style of the small-en entry.

### check_first_run — recommended_model Field

The `recommended_model` field returned by `check_first_run()` drives which card shows the "Recommended" badge in FirstRun.tsx. Per the locked decision, recommendations are unchanged: parakeet for GPU users, small-en for CPU users. No change to `recommended_model` logic.

### Anti-Patterns to Avoid

- **Generic local filename**: Do not use `ggml-model.bin` as the local filename — it is the upstream name and will cause silent overwrites if multiple distil models are added later.
- **Adding distil-v3.5 to download_url() as a filename match**: `download_url()` currently takes a filename, not a model_id, so you cannot distinguish which repo to use from the filename alone. Refactor to per-model URL in model_info instead of patching download_url().
- **Forgetting check_first_run()**: The `needs_setup` predicate must include distil-v3.5. Missing this means users who chose distil-v3.5 as their first model see first-run again on every launch.
- **Forgetting model_id_to_path()**: If `model_id_to_path()` is not updated alongside `list_models()`, clicking distil-v3.5 in the settings ModelSelector will return `"Unknown model id"`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| SHA256 of downloaded file | Custom hash loop | Existing `sha2::Sha256` in download_model() | Already integrated and tested |
| Progress events | Polling or custom WebSocket | Existing `Channel<DownloadEvent>` + `on_event.send()` | Proven pattern with temp file cleanup on error |
| Model switching without restart | Mutex swap with reload | Existing `WhisperStateMutex` + `set_model()` IPC | Already handles reload-on-switch |

## Common Pitfalls

### Pitfall 1: SHA256 Not Pre-Published
**What goes wrong:** `model_info()` requires a SHA256 to validate the download, but HuggingFace's repo page for `distil-whisper/distil-large-v3.5-ggml` does not display a SHA256 for `ggml-model.bin`.
**Why it happens:** HuggingFace shows commit hashes and LFS metadata but not file-level SHA256 digests on the UI.
**How to avoid:** Download the file once locally and compute `certutil -hashfile ggml-model.bin SHA256` (Windows) or `sha256sum ggml-model.bin` (Linux). This is a mandatory first step before hardcoding the hash. Rename to `ggml-distil-large-v3.5.bin` after computing.
**Warning signs:** If the SHA256 placeholder `""` is used, download will succeed but checksum validation will immediately fail — the temp file will be deleted and the download will appear to error.

### Pitfall 2: File Size Estimate for Progress Bar
**What goes wrong:** `model_info()` returns `expected_size_bytes` used as the progress bar denominator before HTTP headers arrive. If off, progress may briefly show > 100%.
**Why it happens:** The 1,519,525,364 byte figure comes from HuggingFace metadata — may not be byte-exact.
**How to avoid:** `download_model()` already falls back to `response.content_length()` when available. The `expected_size_bytes` is only used for the `Started` event. Use `1_519_525_364` as the estimate — it will correct after the first chunk.

### Pitfall 3: Missing distil-v3.5 from needs_setup Predicate
**What goes wrong:** User downloads distil-v3.5 as their first model, completes first-run, restarts app — first-run screen shows again.
**Why it happens:** `check_first_run()` only checks `large_exists || small_exists || parakeet_fp32_exists`. Distil-v3.5 is a new file path not in that predicate.
**How to avoid:** Add `let distil_v35_exists = dir.join("ggml-distil-large-v3.5.bin").exists();` and include it in the `needs_setup` AND condition.

### Pitfall 4: Card Grid with 4 Models
**What goes wrong:** FirstRun.tsx uses `lg:grid-cols-3` for 3+ visible models. With 4 models, one card wraps to a second row on large screens, potentially looking unintentional.
**Why it happens:** The grid class is set based on `visibleModels.length >= 3`, which doesn't distinguish 3 vs 4.
**How to avoid:** Add an `xl:grid-cols-4` breakpoint when `visibleModels.length === 4`, or accept the 3+1 layout as-is. Per locked decisions, showing all 4 cards is correct behavior; the layout question is cosmetic.

### Pitfall 5: download_model() URL Field Extraction After model_info Refactor
**What goes wrong:** After expanding `model_info` to 4-tuple `(filename, url, sha256, size)`, the `download_model()` function must destructure all four fields. If only 3 are destructured, the Rust compiler will catch this at compile time.
**Why it happens:** Tuple destructuring changes when the tuple arity changes.
**How to avoid:** Update the destructuring in `download_model()` when changing `model_info()`. The compiler will flag any mismatch.

## Code Examples

Verified patterns from existing codebase:

### Current download_model() — how model_info flows into URL (before refactor)
```rust
// src-tauri/src/download.rs (current code — shows what to change)
pub async fn download_model(model_id: String, on_event: Channel<DownloadEvent>) -> Result<(), String> {
    let (filename, expected_sha256, expected_size_bytes) =
        model_info(&model_id).ok_or_else(|| format!("Unknown model id: {}", model_id))?;
    let url = download_url(filename);  // <-- must change: download_url() is deleted
    // ...
}
```

### Recommended refactor — model_info embeds URL
```rust
// After refactor — model_info returns (filename, url, sha256, size)
fn model_info(model_id: &str) -> Option<(&'static str, &'static str, &'static str, u64)> {
    match model_id {
        "large-v3-turbo" => Some((
            "ggml-large-v3-turbo-q5_0.bin",
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin",
            "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2",
            601_882_624,
        )),
        "small-en" => Some((
            "ggml-small.en-q5_1.bin",
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en-q5_1.bin",
            "bfdff4894dcb76bbf647d56263ea2a96645423f1669176f4844a1bf8e478ad30",
            199_229_440,
        )),
        "distil-large-v3.5" => Some((
            "ggml-distil-large-v3.5.bin",
            "https://huggingface.co/distil-whisper/distil-large-v3.5-ggml/resolve/main/ggml-model.bin",
            "<sha256-to-be-computed-on-implementation>",  // MUST be filled before shipping
            1_519_525_364,
        )),
        _ => None,
    }
}

// download_model() destructures all four:
let (filename, url, expected_sha256, expected_size_bytes) =
    model_info(&model_id).ok_or_else(|| format!("Unknown model id: {}", model_id))?;
// download_url() function is removed entirely
```

### FirstRun.tsx MODELS array addition
```typescript
// src/components/FirstRun.tsx — add to MODELS array (before large-v3-turbo is fine)
{
  id: 'distil-large-v3.5',
  name: 'Distil Large v3.5',
  size: '1.52 GB',
  quality: 'High accuracy, fast',
  requirement: 'GPU recommended, works on any hardware',
  gpuOnly: false,
}
```

### list_models() addition in lib.rs
```rust
// src-tauri/src/lib.rs — in list_models(), after existing whisper entries
models.push(ModelInfo {
    id: "distil-large-v3.5".to_string(),
    name: "Distil Large v3.5".to_string(),
    description: "High accuracy — 1.52 GB — GPU accelerated when available".to_string(),
    recommended: false,
    downloaded: dir.join("ggml-distil-large-v3.5.bin").exists(),
});
```

### check_first_run() — needs_setup update
```rust
// src-tauri/src/lib.rs — in check_first_run()
let large_exists = dir.join("ggml-large-v3-turbo-q5_0.bin").exists();
let small_exists = dir.join("ggml-small.en-q5_1.bin").exists();
let parakeet_fp32_exists = crate::download::parakeet_fp32_model_exists();
let distil_v35_exists = dir.join("ggml-distil-large-v3.5.bin").exists();  // NEW

FirstRunStatus {
    needs_setup: !large_exists && !small_exists && !parakeet_fp32_exists && !distil_v35_exists,
    // ... rest unchanged
}
```

### App.tsx onComplete — already correct, no change needed
```typescript
// src/App.tsx (current — no change required)
onComplete={async (downloadedModelId) => {
    // ... saves to store ...
    await invoke('set_model', { modelId: downloadedModelId });
    // This already works for distil-large-v3.5 — set_model() just needs
    // model_id_to_path() to recognize the new model_id
}}
```

## State of the Art

| Old Approach | Current Approach | Impact for This Phase |
|--------------|------------------|-----------------------|
| Single HF repo for all Whisper models | Multi-repo (ggerganov for existing, distil-whisper for new) | Must refactor download_url() to per-model URLs |
| 2 Whisper models in model_info | 3 Whisper models | Add distil-v3.5 entry |
| download_url() hardcodes repo | URL embedded in model_info tuple | Cleaner, avoids per-caller URL routing |

**What does NOT change:**
- whisper-rs model loading — distil GGML loads identically via `WhisperContext::new_with_params()`
- Streaming download mechanics — same `Channel<DownloadEvent>` pattern
- SHA256 validation — same `sha2::Sha256` approach
- Parakeet handling — completely separate code path, untouched
- App.tsx onComplete — already calls `set_model()` for all non-Parakeet models

## Open Questions

1. **SHA256 of ggml-model.bin**
   - What we know: File is 1,519,525,364 bytes. HF metadata does not expose SHA256 on the repo UI.
   - What's unclear: The exact SHA256 hash value.
   - Recommendation: First implementation task is to download the file and compute the hash: `certutil -hashfile ggml-model.bin SHA256` (Windows). Rename to `ggml-distil-large-v3.5.bin` after computing. This must happen before the Rust code can be shipped.

2. **Card grid layout with 4 models (cosmetic)**
   - What we know: `gridClass` in FirstRun.tsx uses `lg:grid-cols-3` for 3+ models. With 4 cards, one card wraps to a second row on large screens.
   - What's unclear: Whether 3+1 layout looks acceptable or needs a 4-column breakpoint.
   - Recommendation: Ship with existing grid logic. If it looks off during manual testing, the planner can add `xl:grid-cols-4` when `visibleModels.length === 4`. This is a one-line CSS change, not a design decision.

## Sources

### Primary (HIGH confidence)
- Existing codebase — `download.rs`, `lib.rs`, `FirstRun.tsx`, `ModelSelector.tsx`, `App.tsx` (read directly)
- Prior feasibility research artifact — `artifacts/research/2026-03-03-distil-whisper-ggml-compatibility-feasibility.md` (GGML compatibility, benchmark data, HF repo details, no SHA256 available)

### Secondary (MEDIUM confidence)
- HuggingFace `distil-whisper/distil-large-v3.5-ggml` tree page — file size confirmed as 1.52 GB / 1,519,525,364 bytes

### Tertiary (LOW confidence)
- None

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all changes are within existing patterns; no new dependencies
- Architecture: HIGH — all integration points identified from direct code reading; change surface is small and precisely understood
- SHA256 hash: NOT KNOWN — must be computed at implementation time; only blocking unknown
- Pitfalls: HIGH — identified from code reading, not speculation

**Research date:** 2026-03-03
**Valid until:** 2026-04-03 (stable codebase, no fast-moving dependencies)
