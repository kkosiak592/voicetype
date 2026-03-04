---
phase: quick-35
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/src/bin/benchmark.rs
autonomous: true
requirements: [BENCH-FAIRNESS]
must_haves:
  truths:
    - "Whisper models use VAD chunking for clips >30s, same as all other models"
    - "Streaming Moonshine models feed audio incrementally in small frames, not as one buffer"
    - "Streaming benchmark reports time-to-first-partial in addition to total time"
    - "WER results remain unchanged (same final text, just different processing path)"
  artifacts:
    - path: "src-tauri/src/bin/benchmark.rs"
      provides: "Fair benchmark with VAD chunking for Whisper and incremental feed for streaming"
      contains: "vad_chunk_audio"
  key_links:
    - from: "Whisper benchmark section"
      to: "vad_chunk_audio"
      via: "same pattern as Moonshine/SenseVoice/Parakeet sections"
      pattern: "vad_chunk_audio.*audio"
    - from: "Streaming benchmark sections"
      to: "StreamingModel::process_audio_chunk"
      via: "direct low-level API for incremental frame feeding"
      pattern: "process_audio_chunk"
---

<objective>
Fix two benchmark fairness issues in benchmark.rs:
1. Whisper models skip VAD chunking on 60s clips while all other models chunk — add the same VAD pattern.
2. Streaming Moonshine models receive the full audio buffer at once via `transcribe_samples` (same as batch models) instead of being fed incrementally — switch to the low-level `process_audio_chunk` API to simulate real-time mic input and measure time-to-first-partial.

Purpose: Benchmark results should reflect real-world usage patterns for fair model comparison.
Output: Updated benchmark.rs with both fixes.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src-tauri/src/bin/benchmark.rs
@src-tauri/patches/transcribe-rs/src/engines/moonshine/streaming_engine.rs
@src-tauri/patches/transcribe-rs/src/engines/moonshine/streaming_model.rs
@src-tauri/patches/transcribe-rs/src/engines/moonshine/streaming_state.rs

<interfaces>
<!-- Key types and APIs the executor needs -->

From streaming_model.rs (low-level incremental API):
```rust
const CHUNK_SIZE: usize = 1280; // 80ms at 16kHz

pub struct StreamingModel {
    pub config: StreamingConfig,
    // 5 ONNX sessions: frontend, encoder, adapter, cross_kv, decoder_kv
}

impl StreamingModel {
    pub fn create_state(&self) -> StreamingState;
    pub fn process_audio_chunk(&mut self, state: &mut StreamingState, audio_chunk: &[f32]) -> Result<i32, MoonshineError>;
    pub fn encode(&mut self, state: &mut StreamingState, is_final: bool) -> Result<i32, MoonshineError>;
    pub fn compute_cross_kv(&mut self, state: &mut StreamingState) -> Result<(), MoonshineError>;
    pub fn decode_tokens(&self, tokens: &[i64]) -> Result<String, MoonshineError>;
    pub fn generate(&mut self, samples: &[f32], max_tokens_per_second: f32, max_tokens_override: Option<usize>) -> Result<Vec<i64>, MoonshineError>;
    pub fn decoder_reset(&self, state: &mut StreamingState);
    pub fn decode_step(&mut self, state: &mut StreamingState, token: i64) -> Result<Vec<f32>, MoonshineError>;
}
```

From streaming_engine.rs (current high-level API used by benchmark):
```rust
pub struct MoonshineStreamingEngine {
    model: Option<StreamingModel>,  // private field
}
// transcribe_samples() calls model.generate() internally
```

From streaming_config.rs:
```rust
pub struct StreamingConfig {
    pub bos_id: i64,
    pub eos_id: i64,
    pub max_seq_len: usize,
    // ...
}
```

Current Whisper benchmark (lines 653-761) — NO VAD chunking:
```rust
for (model_path, model_label) in &found_whisper {
    // ... load model ...
    for (wav_path, clip_label) in &clip_paths {
        let audio = read_wav_to_f32(wav_path)?;
        // audio fed directly — no chunking!
        state.full(params, &audio);
    }
}
```

Current cfg gate for vad_chunk_audio (line 128):
```rust
#[cfg(any(feature = "bench_extra", feature = "parakeet"))]
fn vad_chunk_audio(samples: &[f32]) -> Vec<Vec<f32>> { ... }
```

Import gate (line 30-31):
```rust
#[cfg(any(feature = "bench_extra", feature = "parakeet"))]
use voice_activity_detector::VoiceActivityDetector;
```

Note: benchmark binary has `required-features = ["whisper", "parakeet"]` so parakeet
is always present when benchmark runs. Adding "whisper" to the cfg gate is for
correctness but not strictly required.
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add VAD chunking to Whisper benchmark section</name>
  <files>src-tauri/src/bin/benchmark.rs</files>
  <action>
Add VAD chunking to the Whisper model benchmark section (around lines 677-760) using the exact same pattern already used by Moonshine, SenseVoice, and Parakeet sections.

1. Update the cfg gates to include `whisper`:
   - Line 30-31: Change `#[cfg(any(feature = "bench_extra", feature = "parakeet"))]` to `#[cfg(any(feature = "bench_extra", feature = "parakeet", feature = "whisper"))]` for the `VoiceActivityDetector` import.
   - Line 128: Change `#[cfg(any(feature = "bench_extra", feature = "parakeet"))]` to `#[cfg(any(feature = "bench_extra", feature = "parakeet", feature = "whisper"))]` for the `vad_chunk_audio` function.

2. In the Whisper benchmark loop, after reading the audio (line 685 `let audio = ...`), insert the VAD chunking block BEFORE the iteration loop:
   ```rust
   let needs_chunking = audio.len() > 30 * 16000;
   let vad_start = Instant::now();
   let chunks: Vec<Vec<f32>> = if needs_chunking {
       vad_chunk_audio(&audio)
   } else {
       vec![audio]
   };
   let vad_ms = if needs_chunking { vad_start.elapsed().as_millis() as u64 } else { 0 };
   if needs_chunking {
       println!("  VAD chunking: {}ms -> {} segments", vad_ms, chunks.len());
   }
   ```

3. Modify the iteration loop to process chunks instead of raw audio. For each iteration:
   - Create `combined_text` accumulator and `had_error` flag
   - Loop over `chunks.iter().enumerate()`, for each segment:
     - Create fresh `FullParams` and `state` (move params setup inside the chunk loop)
     - Call `state.full(params, &seg)` on the chunk
     - Extract segment text and append to `combined_text`
   - On error, set `had_error = true` and break
   - After chunk loop, if `had_error` break outer loop
   - Use `combined_text` for timing and text output

4. After printing avg/min/max/WER, add the VAD overhead line if chunking was used:
   ```rust
   if needs_chunking {
       println!("  (total incl. VAD: avg={}ms)", avg + vad_ms);
   }
   ```

Note: Each Whisper chunk needs its own `FullParams` and `state` because `state.full()` consumes the params. Create them inside the chunk loop. The `WhisperContext` (ctx) is reused across chunks.
  </action>
  <verify>
    <automated>cd src-tauri && cargo check --bin benchmark --features whisper,parakeet 2>&1 | tail -5</automated>
  </verify>
  <done>Whisper benchmark section applies VAD chunking for clips >30s using the same vad_chunk_audio pattern as all other models. cfg gates updated to include "whisper" feature. Compiles without errors.</done>
</task>

<task type="auto">
  <name>Task 2: Switch streaming Moonshine benchmarks to incremental frame feeding</name>
  <files>src-tauri/src/bin/benchmark.rs</files>
  <action>
Replace the current streaming Moonshine benchmark approach (which uses `engine.transcribe_samples()` to pass the full buffer at once) with incremental frame feeding using the low-level `StreamingModel` API. This applies to all three streaming model sections: moonshine-streaming-tiny, moonshine-streaming-small, moonshine-streaming-medium.

**Problem with current approach:** `MoonshineStreamingEngine` wraps `StreamingModel` but its `model` field is private. We cannot access `process_audio_chunk` through the engine.

**Solution:** For each streaming model section, load the `StreamingModel` directly instead of using `MoonshineStreamingEngine`. The `StreamingModel::new()` takes the same path and parameters.

1. Add imports at the top of the file (inside the `#[cfg(feature = "bench_extra")]` import block):
   ```rust
   #[cfg(feature = "bench_extra")]
   use transcribe_rs::engines::moonshine::streaming_model::StreamingModel;
   #[cfg(feature = "bench_extra")]
   use transcribe_rs::engines::moonshine::model::MoonshineError;
   ```

2. Define a constant for the incremental chunk size (near the top constants):
   ```rust
   /// Simulated microphone frame size for streaming benchmarks: 320ms at 16kHz = 5120 samples.
   /// This approximates real-world audio capture cadence.
   #[cfg(feature = "bench_extra")]
   const STREAMING_FRAME_SAMPLES: usize = 5120;
   ```

3. For each of the three streaming model sections (tiny ~line 1103, small ~line 1201, medium ~line 1299), replace the current pattern with:

   a. **Model loading:** Replace `MoonshineStreamingEngine` with direct `StreamingModel`:
   ```rust
   let mut model = match StreamingModel::new(
       mpath.as_path(),
       0, // num_threads: let ORT decide
       bench_extra_providers.clone(),
   ) {
       Ok(m) => m,
       Err(e) => {
           eprintln!("  ERROR loading {}: {}", model_name, e);
           continue; // or appropriate control flow for the if-let block
       }
   };
   ```
   (Use a flag + `if error { ... }` pattern since we're inside an `if let` block, not a `for` loop — `continue` won't work. Instead, set a `loaded_ok` flag or use a nested block.)

   b. **Inference loop:** For each clip, for each iteration:
   - Create fresh state: `let mut state = model.create_state();`
   - Feed audio in `STREAMING_FRAME_SAMPLES`-sized chunks, tracking time-to-first-partial:
     ```rust
     let t = Instant::now();
     let mut first_partial_ms: Option<u64> = None;

     // Feed audio incrementally in 320ms frames
     for frame_start in (0..seg.len()).step_by(STREAMING_FRAME_SAMPLES) {
         let frame_end = (frame_start + STREAMING_FRAME_SAMPLES).min(seg.len());
         let frame = &seg[frame_start..frame_end];
         model.process_audio_chunk(&mut state, frame)?;
     }

     // Encode all accumulated features (is_final=true for offline-style)
     model.encode(&mut state, true)?;

     if state.memory_len == 0 {
         // No features — skip
         continue;
     }

     model.compute_cross_kv(&mut state)?;

     // Decode
     let duration_sec = seg.len() as f32 / 16000.0;
     let max_tokens = ((duration_sec * 6.5).ceil() as usize).min(model.config.max_seq_len);
     let mut tokens: Vec<i64> = Vec::new();
     let mut current_token = model.config.bos_id;

     for _step in 0..max_tokens {
         let next_token = model.decode_step_greedy(&mut state, current_token)?;
         if next_token == model.config.eos_id {
             break;
         }
         tokens.push(next_token);
         current_token = next_token;

         // Record time-to-first-token
         if first_partial_ms.is_none() {
             first_partial_ms = Some(t.elapsed().as_millis() as u64);
         }
     }

     let text = model.decode_tokens(&tokens)?;
     ```
   - Note: `decode_step_greedy` is likely private. Check if it exists — if not, use `decode_step` which returns logits, then take argmax. Read the actual method signatures to confirm.

   c. **Fallback approach if low-level decode is not accessible:** If `decode_step_greedy` is private or `decode_step` returns something incompatible, use a simpler approach: feed audio incrementally via `process_audio_chunk` (which is the true streaming part), then call `model.generate()` or reconstruct the encode+decode pipeline. The key metric becomes "total time with incremental frontend" vs "total time with batch frontend", plus measuring when `process_audio_chunk` first returns features > 0 as `first_partial_ms`.

   d. **Output:** Print time-to-first-partial alongside existing metrics:
   ```rust
   if i == 0 {
       if let Some(fp) = first_partial_ms {
           println!("  [run 1] {}ms (first-partial: {}ms) — \"{}\"", elapsed, fp, truncate(&text, 80));
       }
   }
   ```

4. **Keep VAD chunking** for streaming models — the existing VAD chunking pattern for >30s clips must remain. The incremental feeding applies to each VAD segment independently.

5. The `BenchResult` struct and results output remain unchanged (same fields: model, clip, avg_ms, min_ms, max_ms, wer, first_text).

**IMPORTANT:** Before implementing, read `streaming_model.rs` fully to confirm:
- Whether `decode_step_greedy` exists and is pub (if not, there's a `decode_step` that returns logits — take argmax)
- The exact field name for `state.memory_len` (verify in `streaming_state.rs`)
- Whether `StreamingModel::new()` signature matches the assumed 3 args

If the low-level API turns out to be too complex to wire (e.g., too many private methods), fall back to a simpler approach: keep `MoonshineStreamingEngine` but manually split audio into `STREAMING_FRAME_SAMPLES` chunks and call `engine.transcribe_samples()` on each chunk separately, measuring time-to-first-non-empty-result. This is less accurate but still better than the current single-buffer approach.
  </action>
  <verify>
    <automated>cd src-tauri && cargo check --bin benchmark --features whisper,parakeet,bench_extra 2>&1 | tail -5</automated>
  </verify>
  <done>All three streaming Moonshine benchmark sections (tiny/small/medium) feed audio incrementally in ~320ms frames via the low-level StreamingModel API instead of passing the full buffer to transcribe_samples. Time-to-first-partial is reported for each run. VAD chunking for >30s clips remains. Compiles without errors.</done>
</task>

</tasks>

<verification>
1. `cargo check --bin benchmark --features whisper,parakeet` succeeds (Whisper VAD fix)
2. `cargo check --bin benchmark --features whisper,parakeet,bench_extra` succeeds (streaming fix)
3. Run a quick smoke test: `cargo run --bin benchmark --features whisper,parakeet --release -- --model whisper-tiny` on a 60s clip and verify VAD chunking output appears
4. Run: `cargo run --bin benchmark --features whisper,parakeet,bench_extra --release -- --model moonshine-streaming-tiny` and verify incremental feeding output with first-partial timing appears
</verification>

<success_criteria>
- Whisper models apply VAD chunking for clips >30s, matching all other models
- Streaming Moonshine models process audio incrementally in small frames, not as a single buffer
- Streaming benchmark output includes time-to-first-partial metric
- All benchmark features compile: whisper, parakeet, bench_extra
- WER computation unchanged (same final text assembly)
</success_criteria>

<output>
After completion, create `.planning/quick/35-fix-benchmark-fairness-add-vad-chunking-/35-SUMMARY.md`
</output>
