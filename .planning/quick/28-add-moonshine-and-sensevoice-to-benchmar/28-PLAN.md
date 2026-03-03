---
phase: quick-28
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/Cargo.toml
  - src-tauri/src/bin/benchmark.rs
autonomous: true
requirements: [BENCH-MOONSHINE, BENCH-SENSEVOICE]

must_haves:
  truths:
    - "Moonshine tiny and base models are benchmarked with latency and WER when bench_extra feature is enabled"
    - "SenseVoice model is benchmarked with latency and WER when bench_extra feature is enabled"
    - "Existing whisper and parakeet benchmarks are unaffected when bench_extra feature is NOT enabled"
    - "Benchmark binary compiles with --features whisper,parakeet alone (no regression)"
  artifacts:
    - path: "src-tauri/Cargo.toml"
      provides: "transcribe-rs dependency behind bench_extra feature flag"
      contains: "bench_extra"
    - path: "src-tauri/src/bin/benchmark.rs"
      provides: "Moonshine and SenseVoice benchmark sections"
      contains: "moonshine"
  key_links:
    - from: "src-tauri/Cargo.toml"
      to: "src-tauri/src/bin/benchmark.rs"
      via: "bench_extra feature flag gates transcribe-rs import and benchmark sections"
      pattern: 'cfg\(feature\s*=\s*"bench_extra"\)'
---

<objective>
Add Moonshine (tiny, base) and SenseVoice models to the standalone benchmark binary via the transcribe-rs crate, gated behind an opt-in `bench_extra` Cargo feature flag.

Purpose: Expand benchmark coverage to include Moonshine and SenseVoice models for latency/WER comparison against existing Whisper and Parakeet models — without affecting the main app build.

Output: Updated Cargo.toml with `bench_extra` feature, updated benchmark.rs with three new model benchmark sections (moonshine-tiny, moonshine-base, sensevoice).
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src-tauri/Cargo.toml
@src-tauri/src/bin/benchmark.rs
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add transcribe-rs dependency and bench_extra feature flag</name>
  <files>src-tauri/Cargo.toml</files>
  <action>
Add a new feature flag `bench_extra` and the `transcribe-rs` optional dependency to `src-tauri/Cargo.toml`:

1. In the `[features]` section, add:
```toml
# Enable Moonshine and SenseVoice benchmarks via transcribe-rs (benchmark binary only).
# Run: cargo run --bin benchmark --features whisper,parakeet,bench_extra --release
bench_extra = ["dep:transcribe-rs"]
```

2. In `[dependencies]`, add:
```toml
# Moonshine + SenseVoice inference via transcribe-rs (optional — benchmark only).
# Uses ort = "2.0.0-rc.10" (same version as parakeet-rs and voice_activity_detector — no conflict).
transcribe-rs = { version = "0.1.5", features = ["moonshine", "sense_voice"], optional = true }
```

3. Do NOT add `bench_extra` to the `default` feature list.

4. Do NOT modify the benchmark `[[bin]]` section's `required-features` — keep it as `["whisper", "parakeet"]`. The bench_extra feature is opt-in on top of the existing required features.
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text/src-tauri && cargo check --features whisper,parakeet 2>&1 | tail -5</automated>
  </verify>
  <done>Cargo.toml has bench_extra feature flag and transcribe-rs optional dependency. Existing `cargo check --features whisper,parakeet` still compiles (no regression).</done>
</task>

<task type="auto">
  <name>Task 2: Add Moonshine and SenseVoice benchmark sections to benchmark.rs</name>
  <files>src-tauri/src/bin/benchmark.rs</files>
  <action>
Add Moonshine (tiny + base) and SenseVoice benchmark sections to `benchmark.rs`, gated behind `#[cfg(feature = "bench_extra")]`. Follow the exact same pattern as existing whisper and parakeet sections.

**1. Add imports at the top of the file (after existing imports):**

```rust
#[cfg(feature = "bench_extra")]
use transcribe_rs::{
    engine::TranscriptionEngine,
    moonshine::{MoonshineModelParams, ModelVariant as MoonshineVariant},
    sense_voice::{SenseVoiceModelParams, SenseVoiceLanguage},
};
```

**2. Add model discovery section (after parakeet discovery, before WAV fixtures section):**

In the `main()` function, after the parakeet `FOUND/MISSING` prints (around line 334), add a `#[cfg(feature = "bench_extra")]` block that:
- Defines the expected model directory paths under `models_dir`:
  - `moonshine-tiny-ONNX` directory for Moonshine tiny
  - `moonshine-base-ONNX` directory for Moonshine base
  - `sensevoice-small` directory for SenseVoice
- Checks existence of each directory and prints FOUND/MISSING
- Stores found paths in variables (e.g., `moonshine_tiny_path`, `moonshine_base_path`, `sensevoice_path`)
- When `bench_extra` is not enabled, print `(bench_extra feature disabled — skipping moonshine/sensevoice models)`

**3. Add Moonshine benchmark section (after the parakeet benchmark section, before `print_summary`):**

Wrap in `#[cfg(feature = "bench_extra")]` block. For EACH Moonshine variant (tiny, then base):

```rust
#[cfg(feature = "bench_extra")]
{
    // --- Moonshine tiny ---
    if let Some(ref mpath) = moonshine_tiny_path {
        println!("\n=== moonshine-tiny ===");
        let load_start = Instant::now();
        let params = MoonshineModelParams::new(MoonshineVariant::Tiny);
        let mut engine = match TranscriptionEngine::new() {
            Ok(e) => e,
            Err(e) => {
                eprintln!("  ERROR creating engine: {}", e);
                // skip to next model
            }
        };
        match engine.load_model_with_params(&mpath.to_string_lossy(), params) {
            Ok(_) => {},
            Err(e) => {
                eprintln!("  ERROR loading moonshine-tiny: {}", e);
                // skip to next model
            }
        }
        println!("  Load time: {}ms", load_start.elapsed().as_millis());

        for (wav_path, clip_label) in &clip_paths {
            // Same iteration pattern as whisper/parakeet:
            // - Read wav via read_wav_to_f32()
            // - 5 iterations
            // - engine.transcribe_audio(&audio, 16000) for each iteration
            // - Collect latencies, first_text from run 1
            // - compute_wer against reference_for_clip()
            // - Push BenchResult with model="moonshine-tiny"
        }
    }

    // --- Moonshine base --- (same pattern, MoonshineVariant::Base, model="moonshine-base")

    // --- SenseVoice ---
    if let Some(ref spath) = sensevoice_path {
        println!("\n=== sensevoice-small ===");
        let load_start = Instant::now();
        let params = SenseVoiceModelParams::new(SenseVoiceLanguage::English);
        let mut engine = match TranscriptionEngine::new() {
            Ok(e) => e,
            Err(e) => { eprintln!("  ERROR creating engine: {}", e); /* skip */ }
        };
        match engine.load_model_with_params(&spath.to_string_lossy(), params) {
            Ok(_) => {},
            Err(e) => { eprintln!("  ERROR loading sensevoice: {}", e); /* skip */ }
        }
        println!("  Load time: {}ms", load_start.elapsed().as_millis());

        for (wav_path, clip_label) in &clip_paths {
            // Same iteration pattern:
            // - Read wav, 5 iterations
            // - engine.transcribe_audio(&audio, 16000)
            // - Collect latencies, first_text, compute_wer
            // - Push BenchResult with model="sensevoice-small"
        }
    }
}
```

**Key implementation details:**

- The transcribe-rs `TranscriptionEngine::new()` returns `Result<Self>`. Handle errors with `eprintln!` + `continue`/skip, matching existing error handling style.
- `engine.transcribe_audio(samples: &[f32], sample_rate: u32)` returns `Result<TranscriptionResult>` where `TranscriptionResult` has a `.text` field (String).
- Use `engine.transcribe_audio(&audio, 16000)` for each iteration — the audio is borrowed, not cloned (unlike parakeet).
- Create a NEW `TranscriptionEngine` for each model variant (Moonshine tiny, Moonshine base, SenseVoice). The engine holds the loaded model state.
- Model directory names on disk should match HuggingFace repo names: `moonshine-tiny-ONNX`, `moonshine-base-ONNX` for Moonshine; `sensevoice-small` for SenseVoice.
- For error handling on engine/model load failures, use the `continue`-to-next-model pattern. Since these are inside a `#[cfg]` block (not a `for` loop), use an early-return pattern with a nested scope or restructure with `if let Ok(...)` chains to skip to the next model on failure.
- The existing `print_summary(&results)` call at the end of main already handles all models — no changes needed there.
- Do NOT modify any existing code outside the `#[cfg(feature = "bench_extra")]` blocks.
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text/src-tauri && cargo check --features whisper,parakeet,bench_extra 2>&1 | tail -10</automated>
  </verify>
  <done>benchmark.rs compiles with `--features whisper,parakeet,bench_extra`. Moonshine tiny, Moonshine base, and SenseVoice sections exist behind cfg(feature = "bench_extra"). Existing whisper/parakeet benchmarks unchanged. Running `cargo check --features whisper,parakeet` (without bench_extra) still compiles cleanly.</done>
</task>

</tasks>

<verification>
1. `cargo check --features whisper,parakeet` — existing build unaffected (no regression)
2. `cargo check --features whisper,parakeet,bench_extra` — new build with transcribe-rs compiles
3. `grep -c 'bench_extra' src-tauri/Cargo.toml` — returns >= 2 (feature def + dep)
4. `grep -c 'moonshine' src-tauri/src/bin/benchmark.rs` — returns >= 5 (imports + model sections)
5. `grep -c 'sense_voice\|sensevoice\|SenseVoice' src-tauri/src/bin/benchmark.rs` — returns >= 5
</verification>

<success_criteria>
- Cargo.toml has `bench_extra` feature flag with `transcribe-rs` optional dependency
- benchmark.rs has Moonshine tiny, Moonshine base, and SenseVoice sections behind `#[cfg(feature = "bench_extra")]`
- All three new models follow the same benchmarking pattern: load model once, iterate 5 times per clip, collect latency + WER
- `cargo check --features whisper,parakeet` compiles (no regression)
- `cargo check --features whisper,parakeet,bench_extra` compiles (new models)
</success_criteria>

<output>
After completion, create `.planning/quick/28-add-moonshine-and-sensevoice-to-benchmar/28-SUMMARY.md`
</output>
