---
phase: quick-33
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/patches/transcribe-rs/src/engines/moonshine/streaming_engine.rs
  - src-tauri/patches/transcribe-rs/src/engines/moonshine/streaming_model.rs
  - src-tauri/src/bin/benchmark.rs
  - test-fixtures/BENCHMARK.md
autonomous: true
requirements: [QUICK-33]
must_haves:
  truths:
    - "StreamingModelParams accepts execution_providers field (same shape as MoonshineModelParams)"
    - "StreamingModel::new() threads providers through to load_session()"
    - "load_session() uses provided providers instead of hardcoded CPUExecutionProvider"
    - "benchmark.rs declares path vars and bench sections for moonshine-streaming-tiny/small/medium"
    - "Streaming engine sections pass bench_extra_providers and do NOT call vad_chunk_audio"
    - "BENCHMARK.md lists the three new streaming models with directory names and sources"
  artifacts:
    - path: src-tauri/patches/transcribe-rs/src/engines/moonshine/streaming_engine.rs
      provides: StreamingModelParams with execution_providers field
    - path: src-tauri/patches/transcribe-rs/src/engines/moonshine/streaming_model.rs
      provides: StreamingModel::new() and load_session() accepting providers
    - path: src-tauri/src/bin/benchmark.rs
      provides: three new moonshine-streaming benchmark sections
    - path: test-fixtures/BENCHMARK.md
      provides: documentation for three new streaming models
  key_links:
    - from: streaming_engine.rs load_model_with_params
      to: streaming_model.rs StreamingModel::new()
      via: params.execution_providers passed as argument
    - from: streaming_model.rs StreamingModel::new()
      to: load_session()
      via: providers arg threaded into each of the 5 session loads
    - from: benchmark.rs bench_extra block
      to: MoonshineStreamingEngine + StreamingModelParams
      via: same pattern as MoonshineEngine + MoonshineModelParams
---

<objective>
Add GPU execution provider support to the streaming Moonshine engine and wire three new Moonshine v2 streaming model variants (tiny, small, medium) into the benchmark binary.

Purpose: The streaming engine currently hardcodes CPUExecutionProvider. To benchmark GPU performance parity with the v1 engine, execution_providers must be injectable. The benchmark then uses that to benchmark moonshine-streaming-tiny/small/medium under CPU and CUDA.

Output:
- streaming_engine.rs — StreamingModelParams gains execution_providers field
- streaming_model.rs — StreamingModel::new() + load_session() accept and apply providers
- benchmark.rs — three new model sections (streaming-tiny/small/medium), no VAD chunking
- BENCHMARK.md — documents new models in the Extended table
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md

<interfaces>
<!-- Existing pattern from model.rs MoonshineModel::new() — execution_providers threaded as Option<Vec<ExecutionProviderDispatch>> -->
<!-- MoonshineModelParams (engine.rs line 89):
pub struct MoonshineModelParams {
    pub max_length: usize,
    pub num_threads: usize,
    pub execution_providers: Option<Vec<ort::execution_providers::ExecutionProviderDispatch>>,
}
-->

<!-- model.rs init_session() fallback pattern (line 98-109):
fn init_session(path: &Path, providers: Option<&[ort::execution_providers::ExecutionProviderDispatch]>) -> Result<Session, MoonshineError> {
    let default_providers = vec![CPUExecutionProvider::default().build()];
    let providers = match providers {
        Some(p) => p.to_vec(),
        None => default_providers,
    };
    Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .with_execution_providers(providers)?
        ...
}
-->

<!-- streaming_model.rs load_session() current hardcode (line 83-85):
let session = builder
    .with_execution_providers([CPUExecutionProvider::default().build()])?
    .commit_from_file(&path)?;
-->

<!-- benchmark.rs path discovery pattern (lines 513-533):
#[cfg(feature = "bench_extra")]
let moonshine_tiny_path: Option<PathBuf> = {
    let p = models_dir.join("moonshine-tiny-ONNX");
    if p.exists() && p.is_dir() {
        println!("  FOUND    moonshine-tiny");
        Some(p)
    } else {
        println!("  MISSING  moonshine-tiny ({})", p.display());
        None
    }
};
-->

<!-- benchmark.rs bench section pattern (lines 816-912):
if let Some(ref mpath) = moonshine_tiny_path {
    println!("\n=== moonshine-tiny (provider={}) ===", ...);
    let load_start = Instant::now();
    let mut engine = MoonshineEngine::new();
    let mut params = MoonshineModelParams::tiny();
    params.execution_providers = bench_extra_providers.clone();
    match engine.load_model_with_params(mpath.as_path(), params) { ... }
    println!("  Load time: {}ms", load_start.elapsed().as_millis());

    for (wav_path, clip_label) in &clip_paths {
        // read WAV, vad_chunk_audio if > 30s, ITERATIONS loop
        // transcribe_samples per chunk, combine text
        // push BenchResult { model: "moonshine-tiny", ... }
    }
}
-->

<!-- benchmark.rs imports (lines 20-28):
#[cfg(feature = "bench_extra")]
use transcribe_rs::{
    TranscriptionEngine,
    engines::moonshine::{MoonshineEngine, MoonshineModelParams},
    engines::sense_voice::{SenseVoiceEngine, SenseVoiceModelParams},
};
-->
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add execution_providers to StreamingModelParams and thread through StreamingModel</name>
  <files>
    src-tauri/patches/transcribe-rs/src/engines/moonshine/streaming_engine.rs
    src-tauri/patches/transcribe-rs/src/engines/moonshine/streaming_model.rs
  </files>
  <action>
**streaming_engine.rs** — Add `execution_providers` field to `StreamingModelParams`:

```rust
pub struct StreamingModelParams {
    pub max_tokens_per_second: f32,
    pub num_threads: usize,
    pub execution_providers: Option<Vec<ort::execution_providers::ExecutionProviderDispatch>>,
}

impl Default for StreamingModelParams {
    fn default() -> Self {
        Self {
            max_tokens_per_second: 6.5,
            num_threads: 0,
            execution_providers: None,
        }
    }
}
```

In `load_model_with_params`, update the `StreamingModel::new()` call to pass providers:

```rust
self.model = Some(StreamingModel::new(model_path, params.num_threads, params.execution_providers)?);
```

**streaming_model.rs** — Update `StreamingModel::new()` signature to accept providers:

```rust
pub fn new(
    model_dir: &Path,
    num_threads: usize,
    providers: Option<Vec<ort::execution_providers::ExecutionProviderDispatch>>,
) -> Result<Self, MoonshineError> {
```

Pass providers into each of the 5 `load_session()` calls:

```rust
let frontend = Self::load_session(model_dir, "frontend", num_threads, providers.as_deref())?;
let encoder  = Self::load_session(model_dir, "encoder",  num_threads, providers.as_deref())?;
let adapter  = Self::load_session(model_dir, "adapter",  num_threads, providers.as_deref())?;
let cross_kv = Self::load_session(model_dir, "cross_kv", num_threads, providers.as_deref())?;
let decoder_kv = Self::load_session(model_dir, "decoder_kv", num_threads, providers.as_deref())?;
```

Update `load_session()` signature and body — replace the hardcoded `CPUExecutionProvider` with the fallback pattern from `model.rs`:

```rust
fn load_session(
    model_dir: &Path,
    name: &str,
    num_threads: usize,
    providers: Option<&[ort::execution_providers::ExecutionProviderDispatch]>,
) -> Result<Session, MoonshineError> {
    // ... path resolution unchanged ...

    let mut builder = Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)?;

    if num_threads > 0 {
        builder = builder.with_intra_threads(num_threads)?;
    }

    let default_providers = vec![CPUExecutionProvider::default().build()];
    let ep_list = match providers {
        Some(p) => p.to_vec(),
        None => default_providers,
    };

    let session = builder
        .with_execution_providers(ep_list)?
        .commit_from_file(&path)?;

    Ok(session)
}
```

Remove the now-unused `use ort::execution_providers::CPUExecutionProvider;` import only if nothing else in the file uses it directly — keep it if the default_providers fallback still references it (it does, so keep the import).
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text/src-tauri && cargo check --features bench_extra 2>&1 | tail -20</automated>
  </verify>
  <done>cargo check --features bench_extra passes with no errors. StreamingModelParams has execution_providers field. StreamingModel::new() and load_session() accept providers param.</done>
</task>

<task type="auto">
  <name>Task 2: Add moonshine-streaming-tiny/small/medium to benchmark binary and BENCHMARK.md</name>
  <files>
    src-tauri/src/bin/benchmark.rs
    test-fixtures/BENCHMARK.md
  </files>
  <action>
**benchmark.rs** — In the `#[cfg(feature = "bench_extra")]` use block at the top (lines 20-25), add the streaming types to the existing moonshine import:

```rust
#[cfg(feature = "bench_extra")]
use transcribe_rs::{
    TranscriptionEngine,
    engines::moonshine::{MoonshineEngine, MoonshineModelParams, MoonshineStreamingEngine, StreamingModelParams},
    engines::sense_voice::{SenseVoiceEngine, SenseVoiceModelParams},
};
```

**Path discovery** — After the `sensevoice_path` block (around line 544), add three new path discovery blocks inside the same `#[cfg(feature = "bench_extra")]` scope (or each guarded individually). Directory names: `moonshine-streaming-tiny`, `moonshine-streaming-small`, `moonshine-streaming-medium`.

```rust
#[cfg(feature = "bench_extra")]
let moonshine_streaming_tiny_path: Option<PathBuf> = {
    let p = models_dir.join("moonshine-streaming-tiny");
    if p.exists() && p.is_dir() {
        println!("  FOUND    moonshine-streaming-tiny");
        Some(p)
    } else {
        println!("  MISSING  moonshine-streaming-tiny ({})", p.display());
        None
    }
};
#[cfg(feature = "bench_extra")]
let moonshine_streaming_small_path: Option<PathBuf> = {
    let p = models_dir.join("moonshine-streaming-small");
    if p.exists() && p.is_dir() {
        println!("  FOUND    moonshine-streaming-small");
        Some(p)
    } else {
        println!("  MISSING  moonshine-streaming-small ({})", p.display());
        None
    }
};
#[cfg(feature = "bench_extra")]
let moonshine_streaming_medium_path: Option<PathBuf> = {
    let p = models_dir.join("moonshine-streaming-medium");
    if p.exists() && p.is_dir() {
        println!("  FOUND    moonshine-streaming-medium");
        Some(p)
    } else {
        println!("  MISSING  moonshine-streaming-medium ({})", p.display());
        None
    }
};
```

**Benchmark sections** — Inside the `#[cfg(feature = "bench_extra")] { ... }` block, after the `moonshine-base` section and before the `sensevoice-small` section, add three bench sections. Key differences from the v1 engine pattern:
- Use `MoonshineStreamingEngine::new()` and `StreamingModelParams::default()` (no tiny()/base() constructors)
- Set `params.execution_providers = bench_extra_providers.clone();`
- **No VAD chunking**: the streaming engine handles long audio natively. Do NOT call `vad_chunk_audio`. Pass the full audio directly as `vec![audio]` and iterate the single element.
- Model strings: `"moonshine-streaming-tiny"`, `"moonshine-streaming-small"`, `"moonshine-streaming-medium"`

Template for each section (shown for tiny; repeat for small and medium):

```rust
// --- Moonshine streaming tiny ---
if let Some(ref mpath) = moonshine_streaming_tiny_path {
    println!("\n=== moonshine-streaming-tiny (provider={}) ===", if bench_extra_providers.is_some() { "cuda" } else { "cpu" });
    let load_start = Instant::now();
    let mut engine = MoonshineStreamingEngine::new();
    let mut params = StreamingModelParams::default();
    params.execution_providers = bench_extra_providers.clone();
    match engine.load_model_with_params(mpath.as_path(), params) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("  ERROR loading moonshine-streaming-tiny: {}", e);
        }
    }
    println!("  Load time: {}ms", load_start.elapsed().as_millis());

    for (wav_path, clip_label) in &clip_paths {
        println!("  Clip: {}", clip_label);
        let audio = match read_wav_to_f32(wav_path) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("  ERROR reading WAV: {}", e);
                continue;
            }
        };

        // Streaming engine handles long audio natively — no VAD chunking needed
        let chunks: Vec<Vec<f32>> = vec![audio];

        let mut latencies: Vec<u64> = Vec::with_capacity(ITERATIONS);
        let mut first_text = String::new();

        for i in 0..ITERATIONS {
            let t = Instant::now();
            let mut combined_text = String::new();
            let mut had_error = false;

            for (seg_idx, seg) in chunks.iter().enumerate() {
                match engine.transcribe_samples(seg.clone(), None) {
                    Ok(result) => {
                        if !combined_text.is_empty() && !result.text.trim().is_empty() {
                            combined_text.push(' ');
                        }
                        combined_text.push_str(result.text.trim());
                    }
                    Err(e) => {
                        eprintln!("  ERROR during inference run {} seg {}: {}", i + 1, seg_idx, e);
                        had_error = true;
                        break;
                    }
                }
            }

            if had_error { break; }

            let elapsed = t.elapsed().as_millis() as u64;
            latencies.push(elapsed);
            if i == 0 {
                first_text = combined_text.trim().to_string();
                println!("  [run 1] {}ms — \"{}\"", elapsed, truncate(&first_text, 80));
            } else {
                println!("  [run {}] {}ms", i + 1, elapsed);
            }
        }

        if latencies.is_empty() {
            continue;
        }
        let avg = latencies.iter().sum::<u64>() / latencies.len() as u64;
        let min = *latencies.iter().min().unwrap();
        let max = *latencies.iter().max().unwrap();

        let reference = reference_for_clip(clip_label);
        let (wer, subs, ins, dels, ref_n) = compute_wer(reference, &first_text);
        println!("  -> avg={}ms  min={}ms  max={}ms  WER={:.1}% (S={} I={} D={} / {} words)", avg, min, max, wer, subs, ins, dels, ref_n);

        results.push(BenchResult {
            model: "moonshine-streaming-tiny".to_string(),
            clip: clip_label.to_string(),
            avg_ms: avg,
            min_ms: min,
            max_ms: max,
            wer,
            first_text,
        });
    }
}
```

Repeat identically for `moonshine_streaming_small_path` / `"moonshine-streaming-small"` and `moonshine_streaming_medium_path` / `"moonshine-streaming-medium"`.

**BENCHMARK.md** — In the Extended table under `## Models / Extended`, add three rows:

```markdown
| moonshine-streaming-tiny   | `moonshine-streaming-tiny/`   | ~? MB | [usefulsensors/moonshine](https://huggingface.co/usefulsensors/moonshine) streaming ONNX |
| moonshine-streaming-small  | `moonshine-streaming-small/`  | ~? MB | [usefulsensors/moonshine](https://huggingface.co/usefulsensors/moonshine) streaming ONNX |
| moonshine-streaming-medium | `moonshine-streaming-medium/` | ~? MB | [usefulsensors/moonshine](https://huggingface.co/usefulsensors/moonshine) streaming ONNX |
```

Also add a note below the table:

> Moonshine streaming models use the 5-session streaming ONNX pipeline (frontend, encoder, adapter, cross_kv, decoder_kv) and do not require VAD chunking — they handle audio of any length natively via sliding-window encoder.
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text/src-tauri && cargo check --features bench_extra,whisper,parakeet 2>&1 | tail -20</automated>
  </verify>
  <done>cargo check with bench_extra,whisper,parakeet passes with no errors. benchmark.rs has path discovery and bench sections for all three streaming models. BENCHMARK.md lists them in the Extended table.</done>
</task>

</tasks>

<verification>
After both tasks:

1. `cargo check --features bench_extra,whisper,parakeet` in `src-tauri/` compiles clean.
2. Grep confirms `execution_providers` field exists in `StreamingModelParams`.
3. Grep confirms `moonshine-streaming-tiny`, `moonshine-streaming-small`, `moonshine-streaming-medium` appear in benchmark.rs path discovery and bench sections.
4. Grep confirms NO `vad_chunk_audio` call inside the streaming sections.
5. BENCHMARK.md Extended table has all three streaming model rows.
</verification>

<success_criteria>
- StreamingModelParams.execution_providers: Option<Vec<ExecutionProviderDispatch>> exists
- StreamingModel::new() and load_session() accept and apply providers (fallback to CPU if None)
- benchmark.rs has 3 new path variables and 3 new bench sections for streaming models
- Streaming bench sections pass bench_extra_providers, no VAD chunking
- cargo check --features bench_extra,whisper,parakeet passes
- BENCHMARK.md documents all three streaming models
</success_criteria>

<output>
After completion, create `.planning/quick/33-add-moonshine-v2-streaming-models-to-ben/33-SUMMARY.md`
</output>
