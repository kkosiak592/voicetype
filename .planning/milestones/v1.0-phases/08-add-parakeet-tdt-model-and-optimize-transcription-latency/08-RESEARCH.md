# Phase 8: Add Parakeet TDT Model and Optimize Transcription Latency - Research

**Researched:** 2026-03-01
**Domain:** Rust ONNX inference (parakeet-rs), Tauri managed state, pipeline micro-optimizations
**Confidence:** MEDIUM-HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Engine coexistence
- Both Parakeet and Whisper are selectable for GPU users — not a replacement, a choice
- Whisper remains the default engine for GPU users (accuracy-first)
- CPU users continue to use Whisper small-en only (Parakeet requires CUDA)
- Engine selector lives in the settings dialog under the model sidebar section
- Hot-swap vs restart on engine switch: Claude's discretion

#### Model management & first-run
- Parakeet appears as a third model card in FirstRun.tsx alongside Large v3 Turbo and Small English
- GPU users see three cards; CPU users see only the Small English card
- Which card is "Recommended" for GPU users: Claude's discretion (should align with default engine decision)
- Multiple models can coexist on disk or one-at-a-time: Claude's discretion
- Model hosting source (HuggingFace vs other): Claude's discretion

#### Vocabulary biasing
- Corrections engine always applies regardless of active engine — profiles still work
- initial_prompt is skipped for Parakeet (not supported), active for Whisper
- ALL CAPS mode still applies for Parakeet
- How to handle the vocabulary biasing gap (warn user, corrections-only, etc.): Claude's discretion
- Whether to pre-populate Parakeet-specific corrections: Claude's discretion

#### Micro-optimizations
- VAD gate removal for hold-to-talk vs keep-always: Claude's discretion
- Injection sleep timing reduction (aggressive, moderate, or keep): Claude's discretion
- WhisperState reuse vs fresh-per-call: Claude's discretion
- Timing instrumentation (permanent vs debug-only): Claude's discretion

### Claude's Discretion
- Hot-swap vs restart on engine switch
- FirstRun recommended card for GPU users
- Model download coexistence strategy (multiple on disk or replace)
- Model hosting source
- Vocabulary biasing gap UX (warn on switch, silent, etc.)
- Pre-populating Parakeet-specific correction entries
- VAD gate behavior per recording mode
- Injection sleep timing values
- WhisperState reuse decision
- Timing log permanence
- Pre-warm clipboard at startup

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope
</user_constraints>

---

## Summary

This phase adds NVIDIA Parakeet TDT as a second selectable transcription engine alongside Whisper, targeting sub-500ms post-release latency on GPU. The parakeet-rs crate (v0.3.3, released Feb 2026) provides native Rust bindings via ONNX Runtime, confirmed active and matching the required pattern: `ParakeetTDT::from_pretrained(dir, Some(config))` + `transcribe_samples(audio, 16000, 1, None)`. The model files come from HuggingFace as separate ONNX encoder/decoder files in an int8-quantized form (~660 MB total), making it a feasible download.

The existing codebase is well-structured for this addition. The `WhisperStateMutex` pattern in lib.rs (Arc<Mutex<Option<T>>>) is the exact pattern needed for a `ParakeetStateMutex`. The `#[cfg(feature = "whisper")]` gating in pipeline.rs shows how to add engine dispatch. `download.rs` already handles streaming downloads with SHA256 — extending `model_info()` with a Parakeet entry is the integration point.

Micro-optimizations are complementary: the injection sleeps in inject.rs are already at 30ms/50ms (reduced from 75ms/120ms in a prior quick task), leaving modest room for further reduction. The post-hoc VAD gate in pipeline.rs (~20-30ms) can be replaced with a simple sample-count check for hold-to-talk mode, which is the safe optimization since hold-to-talk intent is explicit.

**Primary recommendation:** Implement Parakeet TDT via parakeet-rs 0.3.3 as an additive engine (not a Whisper replacement), using `ParakeetStateMutex` parallel to the existing `WhisperStateMutex`. Use int8-quantized ONNX model files from `istupakov/parakeet-tdt-0.6b-v2-onnx` on HuggingFace. Keep Whisper as the default for GPU users (accuracy-first per locked decision). Apply VAD gate bypass for hold-to-talk mode and modest injection sleep reduction as complementary optimizations.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| parakeet-rs | 0.3.3 | NVIDIA Parakeet TDT inference via ONNX Runtime | Native Rust crate, no FFI, CUDA EP support via feature flag, actively maintained (21 releases, last Feb 2026) |
| whisper-rs | 0.15 (existing) | Whisper inference — kept as coexisting engine | No changes needed; existing WhisperStateMutex pattern reused |
| ort (via parakeet-rs) | bundled | ONNX Runtime execution | Bundled by parakeet-rs — no separate dependency needed |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| reqwest | 0.12 (existing) | Streaming model download | Already used in download.rs — extend model_info() with Parakeet entry |
| sha2 | 0.10 (existing) | SHA256 checksum validation | Already used — reuse for Parakeet ONNX files |
| nvml-wrapper | 0.10 (existing) | GPU detection | Already used — no change needed |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| parakeet-rs (TDT ONNX) | sherpa-onnx | sherpa-onnx requires C FFI bindings — no native Rust crate; significantly more build complexity |
| parakeet-rs (TDT ONNX) | distil-large-v3 (Whisper) | Drop-in whisper-rs model swap, ~1.5-2x speedup but still autoregressive — won't reliably hit <500ms |
| istupakov ONNX repo | boldvoice ONNX repo | boldvoice repo is private (401 response); istupakov is public with verified file sizes |
| int8 ONNX (~660 MB) | fp32 ONNX (~3.17 GB) | fp32 is 5x larger with no practical accuracy gain on this use case |

**Installation:**
```toml
# Cargo.toml — add to [dependencies]
parakeet-rs = { version = "0.3", features = ["cuda"] }
```

```bash
# No npm install needed — pure Rust backend addition
# ONNX Runtime is bundled by parakeet-rs via ort crate
```

---

## Architecture Patterns

### Recommended Project Structure

The phase adds new files alongside existing engine files:

```
src-tauri/src/
├── transcribe.rs              # Existing Whisper wrapper — reference pattern
├── transcribe_parakeet.rs     # NEW: Parakeet TDT inference wrapper
├── pipeline.rs                # MODIFY: add engine dispatch (Whisper vs Parakeet)
├── lib.rs                     # MODIFY: ParakeetStateMutex, get_active_engine, set_engine
├── download.rs                # MODIFY: extend model_info() with Parakeet entry
└── inject.rs                  # MODIFY: reduce sleep timings (optional)

src/components/
├── FirstRun.tsx               # MODIFY: add Parakeet as third model card (GPU-only)
└── ModelSelector.tsx          # MODIFY: add engine selector UI (engine ≠ model)
```

### Pattern 1: Parallel Managed State (ParakeetStateMutex)

**What:** Mirror the `WhisperStateMutex` pattern for Parakeet. Both live in managed state simultaneously; pipeline reads the active engine enum to decide which to call.

**When to use:** Both engines loaded simultaneously — no restart on switch (hot-swap). Allows instant engine switching without reinitializing the other engine.

**Example:**
```rust
// lib.rs — parallel to existing WhisperStateMutex
use parakeet_rs::{ParakeetTDT, ExecutionConfig, ExecutionProvider};

pub struct ParakeetStateMutex(pub std::sync::Mutex<Option<Arc<ParakeetTDT>>>);

#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TranscriptionEngine {
    Whisper,
    Parakeet,
}

pub struct ActiveEngine(pub std::sync::Mutex<TranscriptionEngine>);
```

### Pattern 2: Engine Dispatch in pipeline.rs

**What:** Read the active engine from managed state, branch to either `transcribe_audio()` (Whisper) or `transcribe_parakeet()` (Parakeet) in `run_pipeline`.

**When to use:** Every transcription call — this is the central dispatch point.

**Example:**
```rust
// pipeline.rs — replace the single whisper block with engine dispatch
let engine = {
    let state = app.state::<crate::ActiveEngine>();
    let guard = state.0.lock().unwrap_or_else(|e| e.into_inner());
    *guard
};

let transcription = match engine {
    TranscriptionEngine::Whisper => {
        // existing whisper block — unchanged
        let ctx = { /* clone Arc from WhisperStateMutex */ };
        let initial_prompt = { /* read from ActiveProfile */ };
        tauri::async_runtime::spawn_blocking(move || {
            crate::transcribe::transcribe_audio(&ctx, &samples, &initial_prompt)
        }).await??
    }
    TranscriptionEngine::Parakeet => {
        // new parakeet block
        let parakeet = {
            let state = app.state::<crate::ParakeetStateMutex>();
            let guard = state.0.lock().unwrap_or_else(|e| e.into_inner());
            match guard.as_ref() {
                Some(p) => p.clone(),
                None => { /* error: model not loaded */ return; }
            }
        };
        tauri::async_runtime::spawn_blocking(move || {
            crate::transcribe_parakeet::transcribe_with_parakeet(&parakeet, &samples)
        }).await??
    }
};
```

### Pattern 3: transcribe_parakeet.rs — Parakeet Inference Wrapper

**What:** Thin wrapper around `ParakeetTDT::transcribe_samples()`, mirroring transcribe.rs structure.

**When to use:** Called from pipeline.rs engine dispatch on every Parakeet transcription.

**Example:**
```rust
// transcribe_parakeet.rs
// Source: parakeet-rs 0.3 docs + verified from GitHub README
use parakeet_rs::{ParakeetTDT, ExecutionConfig, ExecutionProvider, Transcriber};
use std::path::Path;
use std::time::Instant;

pub fn load_parakeet(model_dir: &str, use_cuda: bool) -> Result<ParakeetTDT, String> {
    let config = if use_cuda {
        Some(ExecutionConfig::new()
            .with_execution_provider(ExecutionProvider::Cuda))
    } else {
        None  // CPU fallback — not used in practice (Parakeet requires GPU)
    };
    ParakeetTDT::from_pretrained(model_dir, config)
        .map_err(|e| format!("Failed to load Parakeet model from '{}': {}", model_dir, e))
}

pub fn transcribe_with_parakeet(parakeet: &ParakeetTDT, audio: &[f32]) -> Result<String, String> {
    let start = Instant::now();
    let result = parakeet.transcribe_samples(audio.to_vec(), 16000, 1, None)
        .map_err(|e| e.to_string())?;
    let text = result.text.trim().to_string();
    log::info!(
        "Parakeet transcription completed in {}ms: '{}'",
        start.elapsed().as_millis(),
        if text.len() > 80 { format!("{}...", &text[..80]) } else { text.clone() }
    );
    Ok(text)
}
```

### Pattern 4: Model Download Extension (download.rs)

**What:** Add Parakeet int8 ONNX files to `model_info()`. The files are split (encoder + decoder + vocab) requiring multi-file download or a HuggingFace directory approach.

**Key complexity:** The Parakeet model is multiple files, not a single binary like Whisper's `.bin`. The download system must handle downloading multiple files into a subdirectory.

**Parakeet int8 ONNX file set (istupakov/parakeet-tdt-0.6b-v2-onnx):**
| File | Size |
|------|------|
| encoder-model.int8.onnx | 652 MB |
| decoder_joint-model.int8.onnx | 9 MB |
| nemo128.onnx | 140 KB |
| vocab.txt | 9 KB |
| config.json | 97 bytes |

Total: ~661 MB for the int8 set.

**Directory layout:**
```
%APPDATA%/VoiceType/models/
├── ggml-large-v3-turbo-q5_0.bin    # Whisper large
├── ggml-small.en-q5_1.bin          # Whisper small
└── parakeet-tdt-v2/                 # NEW directory
    ├── encoder-model.int8.onnx
    ├── decoder_joint-model.int8.onnx
    ├── nemo128.onnx
    ├── vocab.txt
    └── config.json
```

`ParakeetTDT::from_pretrained(model_dir, config)` takes the directory path — this matches the layout above.

### Pattern 5: VAD Gate Bypass for Hold-to-Talk

**What:** Replace the Silero VAD post-hoc scan in `run_pipeline()` with a simple sample count check when in hold-to-talk mode. Toggle mode keeps full VAD (used for auto-stop timing).

**When to use:** Hold-to-talk mode only — user intent is explicit.

**Example:**
```rust
// pipeline.rs — current hold-to-talk VAD gate (~20-30ms)
// Replace:
if !vad::vad_gate_check(&samples) { ... }

// With (hold-to-talk only):
let mode = app.state::<crate::RecordingMode>().get();
let should_transcribe = match mode {
    Mode::HoldToTalk => samples.len() >= 4800,  // 300ms minimum at 16kHz, ~0ms
    Mode::Toggle => vad::vad_gate_check(&samples), // keep VAD for toggle
};
if !should_transcribe { ... }
```

### Anti-Patterns to Avoid

- **Sharing ParakeetTDT across threads without Arc:** `ParakeetTDT` is not `Sync` unless wrapped in Arc. Mirror the `Arc<WhisperContext>` pattern.
- **Calling `transcribe_samples` on the async runtime thread:** Parakeet inference is blocking ONNX Runtime. Always use `spawn_blocking`.
- **Downloading ONNX files as a single archive:** The model files come individually from HuggingFace. Download each file separately with SHA256 validation.
- **Not warming up Parakeet on first call:** ONNX Runtime performs graph optimization on first inference, causing the first call to take 2-5x longer. Run a warm-up transcription on empty or minimal audio at startup.
- **Removing post-hoc VAD for toggle mode:** Toggle mode VAD gate is a correctness requirement (silence detection for auto-stop); only bypass it for hold-to-talk.
- **Nesting Mutex locks:** The existing codebase correctly uses lock-then-clone-Arc-then-drop-guard for WhisperStateMutex. Replicate this for ParakeetStateMutex — never hold two locks simultaneously.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| ONNX Runtime inference | Custom ONNX loader | parakeet-rs 0.3.3 | Handles session management, execution provider init, CUDA device selection, memory layout — all correctly |
| Multi-file progress download | Custom zip/tar extractor | Extend existing download.rs per-file pattern | download.rs already handles streaming + SHA256 + atomic rename — call it per file |
| Parakeet CUDA warmup | Custom timing logic | Call transcribe_samples once at startup with tiny buffer | ONNX Runtime graph optimization is automatic; just trigger it early |
| Engine persistence | New config system | Extend settings.json `engine` key via existing read/write_settings | Same pattern as recording_mode, active_profile_id |

**Key insight:** The download.rs multi-file challenge is the only genuine new pattern. Everything else reuses existing app infrastructure.

---

## Common Pitfalls

### Pitfall 1: ONNX Model Directory — Multi-File vs Single-File

**What goes wrong:** Calling `download_model` with a single model_id won't work for Parakeet — it's 5 files in a directory, not 1 binary like Whisper.

**Why it happens:** The existing `download_model` command downloads one URL to one file. Parakeet requires downloading 5 separate HuggingFace LFS URLs.

**How to avoid:** Implement a `download_parakeet_model` command (or extend the existing one to handle "parakeet-tdt-v2" as a special multi-file case) that downloads all 5 files sequentially with a single progress channel, writing to `models/parakeet-tdt-v2/`. Emit progress events accumulating across all files.

**Warning signs:** `from_pretrained` returning "file not found" errors even after apparent download completion.

### Pitfall 2: ONNX Runtime CUDA Compatibility

**What goes wrong:** `ExecutionProvider::Cuda` fails at runtime with "CUDA EP not registered" or "CUDA version mismatch" — Parakeet falls back to CPU silently or errors.

**Why it happens:** ONNX Runtime's CUDA Execution Provider requires a specific CUDA toolkit version. The parakeet-rs crate bundles ONNX Runtime, but the CUDA EP requires the system CUDA toolkit to match the compiled ORT version.

**How to avoid:** Detect EP initialization failure explicitly. Log whether CUDA EP was successfully registered at startup. The existing GPU detection via nvml-wrapper is necessary but not sufficient — CUDA EP initialization can still fail even if NVML succeeds.

**Warning signs:** First Parakeet inference takes >500ms (indicates CPU fallback, not GPU); check logs for ORT CUDA EP registration messages.

### Pitfall 3: Arc<ParakeetTDT> — Thread Safety

**What goes wrong:** Compiler error: "`ParakeetTDT` cannot be shared between threads safely."

**Why it happens:** `ParakeetTDT` may implement `Send` but not `Sync`. Wrapping in `Arc` requires `Sync`. Verify by attempting to compile.

**How to avoid:** Use `Arc<Mutex<ParakeetTDT>>` for the managed state if `ParakeetTDT` is not `Sync`. This means taking a lock, running inference, then releasing — slightly different from the Whisper pattern where `WhisperContext` is `Sync` and shares across threads.

**Warning signs:** Compile error "Sync is not implemented for ParakeetTDT."

### Pitfall 4: First-Run Flow — GPU Card Visibility

**What goes wrong:** Parakeet card appears for CPU users, causing confusion when they try to download a model that requires CUDA.

**Why it happens:** The `MODELS` array in FirstRun.tsx is currently static — no conditional rendering based on `gpuDetected` prop.

**How to avoid:** Filter `MODELS` array in FirstRun.tsx render based on `gpuDetected`. If `!gpuDetected`, only show the `small-en` card. Parakeet card shows only when `gpuDetected === true`.

**Warning signs:** CPU users see a Parakeet card during first-run setup; download fails at CUDA EP init.

### Pitfall 5: Hot-Swap State Races

**What goes wrong:** User switches engine mid-pipeline. New engine selected in UI while previous inference is running in spawn_blocking.

**Why it happens:** `ActiveEngine` state is read at the start of `run_pipeline`, but the switch happens after that read. This is actually safe — the engine read is captured before inference starts. The concern is that `ParakeetStateMutex` is None when the user switches to Parakeet before downloading.

**How to avoid:** Read `ActiveEngine` and validate the corresponding model is loaded at pipeline entry. If Parakeet is active but `ParakeetStateMutex` is None, fall back to Whisper with a log warning. Don't error — silently use Whisper.

**Warning signs:** "Parakeet model not loaded" errors in logs when engine is set to Parakeet.

### Pitfall 6: Injection Timing Regression

**What goes wrong:** Reducing clipboard propagation delay below 30ms causes paste failures in some applications (particularly Electron apps like VS Code, Notion).

**Why it happens:** Windows clipboard is process-local until the next message pump cycle. Some apps delay clipboard reads.

**How to avoid:** The current 30ms/50ms values are already reduced from 75ms/120ms (prior quick task). Any further reduction should be tested empirically. Recommendation: keep current values (30ms/50ms) unless benchmarking shows injection is still a bottleneck after Parakeet inference savings. The injection savings (~35ms max) are not worth risking paste failures.

**Warning signs:** Text not appearing in VS Code, Notepad++, or browser address bars after timing reduction.

### Pitfall 7: Parakeet Audio Length Limit

**What goes wrong:** ONNX Runtime errors or silent failure for recordings longer than ~8-10 minutes.

**Why it happens:** CTC/TDT models have a fixed sequence length limit. The parakeet-rs README explicitly warns: "TDT model has sequence length limitations (~8-10 minutes max)."

**How to avoid:** The app already has a 60-second safety cap (MAX_RECORDING_FRAMES in vad.rs). This is well within the 8-10 minute limit. No additional protection needed.

**Warning signs:** Only relevant if the 60s safety cap is ever removed.

---

## Code Examples

Verified patterns from parakeet-rs 0.3 docs and GitHub README:

### Loading Parakeet TDT with CUDA
```rust
// Source: github.com/altunenes/parakeet-rs README + docs.rs/parakeet-rs
use parakeet_rs::{ParakeetTDT, ExecutionConfig, ExecutionProvider};

let config = ExecutionConfig::new()
    .with_execution_provider(ExecutionProvider::Cuda);
let mut parakeet = ParakeetTDT::from_pretrained("./parakeet-tdt-v2", Some(config))?;
```

### Transcribing Audio Samples
```rust
// Source: github.com/altunenes/parakeet-rs examples/raw.rs
use parakeet_rs::Transcriber;

// audio: Vec<f32> at 16kHz mono (same format as Whisper input)
let result = parakeet.transcribe_samples(audio, 16000, 1, None)?;
let text = result.text.trim().to_string();
```

### Extending model_info() for Multi-File Download (download.rs)
```rust
// Source: existing download.rs pattern — extended for multi-file model
// Parakeet requires a directory with 5 files — download each file separately

// Conceptual extension — actual implementation needs a multi-file variant
// Each file gets its own SHA256 + progress tracking
const PARAKEET_FILES: &[(&str, &str, u64)] = &[
    (
        "encoder-model.int8.onnx",
        "<sha256>",   // verify from HuggingFace
        652_000_000,
    ),
    (
        "decoder_joint-model.int8.onnx",
        "<sha256>",
        9_000_000,
    ),
    (
        "nemo128.onnx",
        "<sha256>",
        140_000,
    ),
    (
        "vocab.txt",
        "<sha256>",
        9_600,
    ),
    (
        "config.json",
        "<sha256>",
        97,
    ),
];
```

### Engine Selector in settings.json
```rust
// settings.json key: "active_engine" : "whisper" | "parakeet"
// Follows existing pattern of read_settings / write_settings
fn read_saved_engine(app: &tauri::App) -> TranscriptionEngine {
    // same pattern as read_saved_mode()
    let json = read_settings_app(app).unwrap_or_default();
    match json.get("active_engine").and_then(|v| v.as_str()) {
        Some("parakeet") => TranscriptionEngine::Parakeet,
        _ => TranscriptionEngine::Whisper,  // default
    }
}
```

### FirstRun.tsx — Conditional Parakeet Card
```tsx
// Source: existing FirstRun.tsx pattern — filtered MODELS array
const MODELS = [
  { id: 'large-v3-turbo', name: 'Large v3 Turbo', size: '574 MB',
    quality: 'Best accuracy', requirement: 'Requires NVIDIA GPU', gpuOnly: true },
  { id: 'parakeet-tdt-v2', name: 'Parakeet TDT', size: '661 MB',
    quality: 'Fastest (GPU)', requirement: 'Requires NVIDIA GPU', gpuOnly: true },
  { id: 'small-en', name: 'Small (English)', size: '190 MB',
    quality: 'Works on CPU', requirement: 'Works on any CPU', gpuOnly: false },
];

// Filter in render:
const visibleModels = gpuDetected
  ? MODELS
  : MODELS.filter(m => !m.gpuOnly);
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Autoregressive Whisper decoder | Non-autoregressive TDT (Token-and-Duration Transducer) | Parakeet v2 (2024) | Inference time does not scale with audio length — constant ~20-80ms regardless of 1s vs 60s clip |
| Single monolithic model file (.bin) | Split encoder/decoder ONNX files | ONNX model export standard | Multi-file download required; separate files allow independent quantization |
| Post-hoc VAD for all modes | Mode-conditional VAD (hold-to-talk = length check) | This phase | Saves ~20-30ms for hold-to-talk without affecting toggle mode accuracy |
| Single engine (Whisper only) | Engine enum with runtime dispatch | This phase | Users can choose speed vs accuracy tradeoff |

**Deprecated/outdated:**
- The research artifact's initial sketch used `Transcriber::new(config)` — the actual 0.3 API is `ParakeetTDT::from_pretrained(dir, config)`. Use the verified API.
- The research artifact referenced `parakeet-tdt-0.6b-v2` (v2) and `v3` — use v2 (English-only, better tested for English accuracy) not v3 (multilingual), since the app is English-only.

---

## Discretion Recommendations

Based on research, here are concrete recommendations for the Claude's Discretion items:

### Hot-swap vs restart on engine switch
**Recommendation: Hot-swap.** Both `WhisperStateMutex` and `ParakeetStateMutex` can coexist in managed state. Engine switch just updates `ActiveEngine` enum. No restart needed. Whisper context remains loaded when Parakeet is active (memory cost: ~1.5GB VRAM for both). This is acceptable for GPU users.

### FirstRun recommended card for GPU users
**Recommendation: Large v3 Turbo (Whisper) stays Recommended.** The locked decision says "Whisper is the default engine for GPU users (accuracy-first)." Marking Parakeet as Recommended would contradict this. Mark Large v3 Turbo as Recommended, Parakeet as "Fastest", small-en as "CPU".

### Model download coexistence strategy
**Recommendation: Multiple on disk (coexist).** User may want to switch back and forth. Re-downloading 660MB to switch engines would be a terrible UX. The models directory stores both: Whisper .bin files + Parakeet ONNX directory.

### Model hosting source
**Recommendation: `istupakov/parakeet-tdt-0.6b-v2-onnx` on HuggingFace.** Public repo, verified file list, int8-quantized (661MB vs 3.17GB for fp32). SHA256 checksums must be collected from the actual file downloads before hardcoding.

### Vocabulary biasing gap UX
**Recommendation: Silent with info tooltip in settings.** Show a small info icon next to the engine selector: "Parakeet doesn't support vocabulary prompting. Your corrections dictionary still applies." No blocking warning — corrections engine compensates sufficiently.

### VAD gate behavior
**Recommendation: Hold-to-talk uses sample-count check (4800 samples = 300ms minimum). Toggle mode keeps full Silero VAD.** Hold-to-talk intent is explicit; VAD adds latency without benefit.

### Injection sleep timing
**Recommendation: Keep current 30ms/50ms.** Already reduced from 75ms/120ms in a prior quick task. The Parakeet inference savings (~720ms from the dominant bottleneck) make the injection overhead proportionally smaller. Risking paste drops for 35ms savings is not worth it.

### WhisperState reuse
**Recommendation: Keep fresh-per-call** (as per existing transcribe.rs comment: "A fresh WhisperState is created per call — this is thread-safe and the recommended approach"). The RESEARCH.md from Phase 2 documents this explicitly.

### Timing log permanence
**Recommendation: Permanent at INFO level.** Existing transcribe.rs already logs inference duration at INFO level. Match this pattern for Parakeet. No separate debug flag needed — log::info! is gated by env filter in production.

### Pre-warm clipboard
**Recommendation: Skip.** inject.rs creates Clipboard per call (arboard pattern). Pre-warming saves ~1-2ms. Not worth adding startup complexity for this marginal gain.

---

## Open Questions

1. **SHA256 checksums for Parakeet ONNX files**
   - What we know: File sizes confirmed from HuggingFace repo. SHA256s must be collected.
   - What's unclear: Checksums need to be computed from actual downloaded files (HuggingFace may not surface them in the UI).
   - Recommendation: During implementation (Wave 1), download files manually, compute SHA256, then hardcode in download.rs. Alternative: use HuggingFace's `?download=true` API which includes ETag/hash.

2. **Arc<ParakeetTDT> thread-safety**
   - What we know: parakeet-rs 0.3.3 wraps ONNX Runtime sessions which are `Send + Sync` in ort 2.x.
   - What's unclear: Whether parakeet-rs derives or manually implements `Send + Sync` for `ParakeetTDT`.
   - Recommendation: Attempt `Arc<ParakeetTDT>` first (mirroring Whisper). If compiler rejects, use `Arc<Mutex<ParakeetTDT>>` instead.

3. **Parakeet CUDA EP version compatibility with existing CUDA toolkit**
   - What we know: App currently uses CUDA 11.7 (set via CMAKE_CUDA_ARCHITECTURES=61 for whisper-rs). parakeet-rs bundles its own ONNX Runtime.
   - What's unclear: Whether the bundled ORT version in parakeet-rs 0.3.3 expects CUDA 11.x or 12.x.
   - Recommendation: Test at runtime — `ExecutionProvider::Cuda` initialization failure should be caught and logged. If CUDA EP fails, report to user in settings UI ("Parakeet requires CUDA 12.x — update your drivers").

4. **Parakeet first-inference warm-up timing**
   - What we know: ONNX Runtime performs graph optimization on first call. For Whisper, load_whisper_context is called at startup (warm). For Parakeet, `from_pretrained` may do graph compile at load or at first inference.
   - What's unclear: Exact timing of ONNX graph optimization for Parakeet (at load vs at first transcribe_samples call).
   - Recommendation: After initializing `ParakeetTDT`, run one warm-up transcription with a 300ms silence buffer before marking the engine as ready. Do this in a background thread post-startup.

---

## Sources

### Primary (HIGH confidence)
- `github.com/altunenes/parakeet-rs` - README API docs, version 0.3.3 confirmed, examples/raw.rs TDT pattern
- `docs.rs/parakeet-rs` - Public API: ParakeetTDT, ExecutionConfig, ExecutionProvider, Transcriber trait
- Existing codebase (`src-tauri/src/`) - transcribe.rs, pipeline.rs, download.rs, lib.rs, inject.rs, FirstRun.tsx read directly

### Secondary (MEDIUM confidence)
- `huggingface.co/istupakov/parakeet-tdt-0.6b-v2-onnx/tree/main` - File list and sizes confirmed (encoder-model.int8.onnx 652MB, decoder_joint-model.int8.onnx 9MB, nemo128.onnx 140KB, vocab.txt 9KB, config.json 97B)
- `crates.io/crates/parakeet-rs` - Version 0.3.3, CUDA feature flag, multiple execution providers (cuda, tensorrt, webgpu, directml)
- `artifacts/research/2026-03-01-sub-500ms-transcription-latency-technical.md` - Prior research artifact with latency breakdown

### Tertiary (LOW confidence)
- parakeet-rs CUDA EP compatibility with CUDA 11.7 — not explicitly verified; needs runtime testing
- SHA256 checksums for Parakeet ONNX files — not collected; need to compute from actual downloads
- Arc<ParakeetTDT> Sync status — inferred from ONNX Runtime ort 2.x thread safety; needs compile verification

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — parakeet-rs 0.3.3 confirmed, API verified from docs.rs and GitHub
- Architecture: HIGH — existing codebase patterns fully read, integration points confirmed
- Pitfalls: MEDIUM — CUDA EP compatibility and thread safety are LOW confidence and need runtime verification
- Model files: MEDIUM — sizes confirmed, SHA256s need collection during implementation

**Research date:** 2026-03-01
**Valid until:** 2026-03-31 (parakeet-rs is active but not hyper-moving; ONNX model files are stable)
