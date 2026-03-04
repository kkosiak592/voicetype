---
phase: quick-29
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/patches/transcribe-rs/Cargo.toml
  - src-tauri/patches/transcribe-rs/src/engines/moonshine/model.rs
  - src-tauri/patches/transcribe-rs/src/engines/moonshine/engine.rs
  - src-tauri/patches/transcribe-rs/src/engines/sense_voice/model.rs
  - src-tauri/patches/transcribe-rs/src/engines/sense_voice/engine.rs
  - src-tauri/Cargo.toml
  - src-tauri/src/bin/benchmark.rs
autonomous: true
requirements: [QUICK-29]

must_haves:
  truths:
    - "Moonshine benchmark runs with CUDA execution provider when NVIDIA GPU is detected"
    - "SenseVoice benchmark runs with CUDA execution provider when NVIDIA GPU is detected"
    - "CPU fallback still works when no GPU is available"
    - "Existing whisper and parakeet benchmarks are unaffected"
  artifacts:
    - path: "src-tauri/patches/transcribe-rs/"
      provides: "Local patch of transcribe-rs with configurable execution providers"
    - path: "src-tauri/Cargo.toml"
      provides: "Patch entry for transcribe-rs + cuda/directml features"
    - path: "src-tauri/src/bin/benchmark.rs"
      provides: "GPU-accelerated Moonshine and SenseVoice benchmarks"
  key_links:
    - from: "src-tauri/Cargo.toml"
      to: "src-tauri/patches/transcribe-rs/"
      via: "[patch.crates-io] transcribe-rs path override"
      pattern: 'transcribe-rs.*path.*patches/transcribe-rs'
    - from: "src-tauri/src/bin/benchmark.rs"
      to: "transcribe_rs::engines::moonshine::MoonshineModelParams"
      via: "execution_providers field on model params"
      pattern: "execution_providers"
---

<objective>
Patch transcribe-rs locally to expose execution provider configuration (CUDA/DirectML/CPU), then update benchmark.rs to use GPU acceleration for Moonshine and SenseVoice models.

Purpose: Make the benchmark an apples-to-apples GPU comparison across all models (Whisper, Parakeet, Moonshine, SenseVoice).
Output: Local transcribe-rs patch at src-tauri/patches/transcribe-rs/, updated Cargo.toml, updated benchmark.rs.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src-tauri/Cargo.toml
@src-tauri/src/bin/benchmark.rs
@src-tauri/patches/parakeet-rs/src/execution.rs (reference pattern for EP config)
@src-tauri/patches/parakeet-rs/Cargo.toml (reference pattern for feature flags)

<interfaces>
<!-- From parakeet-rs patch — the execution provider pattern to replicate -->

From src-tauri/patches/parakeet-rs/src/execution.rs:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExecutionProvider {
    #[default]
    Cpu,
    #[cfg(feature = "cuda")]
    Cuda,
    #[cfg(feature = "directml")]
    DirectML,
}

pub struct ModelConfig {
    pub execution_provider: ExecutionProvider,
    pub intra_threads: usize,
    pub inter_threads: usize,
}

impl ModelConfig {
    pub(crate) fn apply_to_session_builder(&self, builder: SessionBuilder) -> Result<SessionBuilder>;
}
```

From transcribe-rs upstream — the hardcoded CPU pattern to replace:

```rust
// moonshine/model.rs line 98-106 and sense_voice/model.rs line 124-131:
fn init_session(path: &Path) -> Result<Session, Error> {
    let providers = vec![CPUExecutionProvider::default().build()];
    let session = Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .with_execution_providers(providers)?
        .with_parallel_execution(true)?
        .commit_from_file(path)?;
    Ok(session)
}
```

From benchmark.rs — existing parakeet GPU config pattern:

```rust
let config = if parakeet_provider == "cuda" {
    Some(ExecutionConfig::new().with_execution_provider(ExecutionProvider::Cuda))
} else {
    None
};
let mut parakeet = ParakeetTDT::from_pretrained(&*parakeet_path.to_string_lossy(), config)?;
```

From benchmark.rs — current Moonshine usage (lines 632-634):

```rust
let mut engine = MoonshineEngine::new();
engine.load_model_with_params(mpath.as_path(), MoonshineModelParams::tiny())?;
```

From benchmark.rs — current SenseVoice usage (lines 771-772):

```rust
let mut engine = SenseVoiceEngine::new();
engine.load_model_with_params(spath.as_path(), SenseVoiceModelParams::default())?;
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Create local transcribe-rs patch with configurable execution providers</name>
  <files>
    src-tauri/patches/transcribe-rs/ (entire crate copy)
    src-tauri/patches/transcribe-rs/Cargo.toml
    src-tauri/patches/transcribe-rs/src/engines/moonshine/model.rs
    src-tauri/patches/transcribe-rs/src/engines/moonshine/engine.rs
    src-tauri/patches/transcribe-rs/src/engines/sense_voice/model.rs
    src-tauri/patches/transcribe-rs/src/engines/sense_voice/engine.rs
    src-tauri/Cargo.toml
  </files>
  <action>
    1. Copy the full transcribe-rs v0.2.8 source from the cargo registry to `src-tauri/patches/transcribe-rs/`:
       ```
       cp -r ~/.cargo/registry/src/index.crates.io-*/transcribe-rs-0.2.8/* src-tauri/patches/transcribe-rs/
       ```

    2. Edit `src-tauri/patches/transcribe-rs/Cargo.toml`:
       - Add feature flags matching parakeet-rs pattern:
         ```toml
         cuda = ["ort/cuda"]
         directml = ["ort/directml"]
         ```
       - Keep all existing features unchanged.

    3. Edit `src-tauri/patches/transcribe-rs/src/engines/moonshine/model.rs`:
       - Change `MoonshineModel::new()` signature to accept an optional providers vec:
         `pub fn new(model_dir: &Path, variant: ModelVariant, providers: Option<Vec<ort::execution_providers::ExecutionProviderDispatch>>) -> Result<Self, MoonshineError>`
       - Change `init_session` to accept providers:
         `fn init_session(path: &Path, providers: Option<&[ort::execution_providers::ExecutionProviderDispatch]>) -> Result<Session, MoonshineError>`
       - In `init_session`, if providers is Some, use those; otherwise default to `vec![CPUExecutionProvider::default().build()]`.
       - Pass providers through from `new()` to both encoder and decoder `init_session` calls.

    4. Edit `src-tauri/patches/transcribe-rs/src/engines/moonshine/engine.rs`:
       - Add `pub execution_providers: Option<Vec<ort::execution_providers::ExecutionProviderDispatch>>` field to `MoonshineModelParams`.
       - Update `Default` impl for `MoonshineModelParams` to set `execution_providers: None`.
       - Update `tiny()` and `base()` constructors to include `execution_providers: None`.
       - In `load_model_with_params`, pass `params.execution_providers` to `MoonshineModel::new()`.

    5. Edit `src-tauri/patches/transcribe-rs/src/engines/sense_voice/model.rs`:
       - Change `SenseVoiceModel::new()` signature to accept providers:
         `pub fn new(model_dir: &Path, quantized: bool, providers: Option<Vec<ort::execution_providers::ExecutionProviderDispatch>>) -> Result<Self, SenseVoiceError>`
       - Change `init_session` to accept providers (same pattern as moonshine).
       - Pass providers through.

    6. Edit `src-tauri/patches/transcribe-rs/src/engines/sense_voice/engine.rs`:
       - Add `pub execution_providers: Option<Vec<ort::execution_providers::ExecutionProviderDispatch>>` field to `SenseVoiceModelParams`.
       - Update `Default` impl for `SenseVoiceModelParams` to set `execution_providers: None`.
       - Update `fp32()` and `int8()` constructors to include `execution_providers: None`.
       - In `load_model_with_params`, pass `params.execution_providers` to `SenseVoiceModel::new()`.

    7. Edit `src-tauri/Cargo.toml`:
       - Change `transcribe-rs` dependency line to add cuda and directml features:
         `transcribe-rs = { version = "0.2.8", features = ["moonshine", "sense_voice", "cuda", "directml"], optional = true }`
       - Add patch entry under `[patch.crates-io]`:
         `transcribe-rs = { path = "patches/transcribe-rs" }`

    IMPORTANT: Do NOT change any other transcribe-rs files beyond the 4 model.rs/engine.rs files. The rest of the crate stays untouched.

    IMPORTANT: The `ort::execution_providers::ExecutionProviderDispatch` type is what `.build()` returns on CUDAExecutionProvider, CPUExecutionProvider, etc. This is the correct type for the providers vec.
  </action>
  <verify>
    <automated>cd src-tauri && cargo check --features whisper,parakeet,bench_extra --release 2>&1 | tail -5</automated>
  </verify>
  <done>
    - Local transcribe-rs patch exists at src-tauri/patches/transcribe-rs/
    - MoonshineModelParams and SenseVoiceModelParams have execution_providers field
    - Cargo.toml has [patch.crates-io] entry for transcribe-rs with cuda,directml features
    - `cargo check` passes with all features enabled
  </done>
</task>

<task type="auto">
  <name>Task 2: Update benchmark.rs to pass GPU execution providers to Moonshine and SenseVoice</name>
  <files>src-tauri/src/bin/benchmark.rs</files>
  <action>
    1. Add import for ort execution providers at the top of the `#[cfg(feature = "bench_extra")]` imports block:
       ```rust
       #[cfg(feature = "bench_extra")]
       use ort::execution_providers::{CUDAExecutionProvider, CPUExecutionProvider};
       ```
       (Note: CPUExecutionProvider is already used inside transcribe-rs, but we need it in benchmark.rs to build the providers vec.)

    2. In the `#[cfg(feature = "bench_extra")]` section (around line 627), BEFORE the moonshine-tiny block, create the providers vec based on `parakeet_provider`:
       ```rust
       let bench_extra_providers: Option<Vec<ort::execution_providers::ExecutionProviderDispatch>> =
           if parakeet_provider == "cuda" {
               println!("  [bench_extra] Using CUDA ExecutionProvider for Moonshine/SenseVoice");
               Some(vec![
                   CUDAExecutionProvider::default().with_tf32(true).build(),
                   CPUExecutionProvider::default().build(),
               ])
           } else {
               None  // Use default CPU
           };
       ```

    3. Update moonshine-tiny section (around line 634) — pass providers to MoonshineModelParams:
       ```rust
       let mut params = MoonshineModelParams::tiny();
       params.execution_providers = bench_extra_providers.clone();
       engine.load_model_with_params(mpath.as_path(), params)?;
       ```

    4. Update moonshine-base section (around line 703) — same pattern:
       ```rust
       let mut params = MoonshineModelParams::base();
       params.execution_providers = bench_extra_providers.clone();
       engine.load_model_with_params(mpath.as_path(), params)?;
       ```

    5. Update sensevoice-small section (around line 772) — same pattern:
       ```rust
       let mut params = SenseVoiceModelParams::default();
       params.execution_providers = bench_extra_providers.clone();
       engine.load_model_with_params(spath.as_path(), params)?;
       ```

    6. Update the section headers to show the provider:
       - moonshine-tiny: `println!("\n=== moonshine-tiny (provider={}) ===", if bench_extra_providers.is_some() { "cuda" } else { "cpu" });`
       - moonshine-base: same pattern
       - sensevoice-small: same pattern

    IMPORTANT: The `parakeet_provider` variable is already computed at the top of main() from detect_gpu(). Reuse it — do not add new GPU detection logic.
  </action>
  <verify>
    <automated>cd src-tauri && cargo check --features whisper,parakeet,bench_extra --release 2>&1 | tail -5</automated>
  </verify>
  <done>
    - benchmark.rs creates CUDA providers vec when GPU is detected
    - Moonshine tiny, Moonshine base, and SenseVoice sections all receive GPU providers
    - Section headers print the active provider (cuda/cpu)
    - `cargo check` passes
  </done>
</task>

</tasks>

<verification>
1. `cd src-tauri && cargo check --features whisper,parakeet,bench_extra --release` — compiles without errors
2. `cargo build --bin benchmark --features whisper,parakeet,bench_extra --release` — binary builds successfully
3. Run with `--release` on a machine with NVIDIA GPU: moonshine and sensevoice sections should print "provider=cuda" in their headers
</verification>

<success_criteria>
- Local transcribe-rs patch at src-tauri/patches/transcribe-rs/ with configurable execution providers
- Cargo.toml patches transcribe-rs and enables cuda+directml features
- benchmark.rs passes CUDA providers to all bench_extra models when GPU detected
- Full build succeeds: `cargo build --bin benchmark --features whisper,parakeet,bench_extra --release`
</success_criteria>

<output>
After completion, create `.planning/quick/29-patch-transcribe-rs-for-gpu-execution-pr/29-SUMMARY.md`
</output>
