---
phase: quick-37
plan: 37
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/src/bin/benchmark.rs
autonomous: true
requirements: ["QUICK-37"]

must_haves:
  truths:
    - "Running with --progressive produces per-chunk timing and post-release latency for each model/clip"
    - "Without --progressive, benchmark behavior is unchanged"
    - "Progressive results appear alongside batch results in the summary and pivot tables"
    - "WER from progressive mode matches batch mode (same chunks, same concatenation)"
  artifacts:
    - path: "src-tauri/src/bin/benchmark.rs"
      provides: "--progressive flag, progressive benchmark loop, combined output tables"
      contains: "--progressive"
  key_links:
    - from: "progressive flag parsing"
      to: "vad_chunk_audio"
      via: "always VAD-chunks regardless of clip length when progressive=true"
      pattern: "progressive.*vad_chunk"
---

<objective>
Add a `--progressive` flag to the benchmark binary that simulates VAD-driven progressive chunk transcription during recording. When enabled, each WAV is VAD-chunked, chunks are dispatched with simulated real-time availability (each chunk "becomes available" at the real-time offset of its end in the audio), and transcription happens sequentially as chunks arrive. The key new metric is "post-release latency" -- time from the last chunk becoming available to final transcription completion. This directly measures user-perceived delay in a progressive dictation pipeline vs the current batch approach.

Purpose: Quantify the latency advantage of progressive transcription to validate the architectural direction for sub-500ms end-of-speech-to-text delivery.
Output: Modified benchmark.rs with --progressive support and comparative output tables.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src-tauri/src/bin/benchmark.rs

Key existing patterns:
- CLI parsing: manual `std::env::args()` loop at line 540-557, checks `--model`/`-m` and `--cpu`
- VAD chunking: `vad_chunk_audio()` at line 137, same algorithm as `vad_chunk_for_moonshine` in vad.rs (320ms silence split, 30s cap, 0.5s min)
- BenchResult struct at line 516: { model, clip, avg_ms, min_ms, max_ms, wer, first_text }
- Each engine section: loads model, iterates `clip_paths`, runs ITERATIONS=3 times, pushes BenchResult
- Output: `print_summary()` at line 1846 (flat table + model summary + pivot tables), `write_markdown_report()` at line 2035

Engine transcription call sites:
- Whisper (line 798): loop over chunks, `state.full(params, seg)` per chunk, collect segment text
- Parakeet (line 928): loop over chunks, `parakeet.transcribe(&seg)` per chunk
- Moonshine tiny/base (line 1049, 1148): loop over chunks, `engine.transcribe(&seg)`
- Streaming moonshine (line 1253, 1431, 1599): loop over chunks, frame-by-frame feed
- SenseVoice (line 1761): loop over chunks, `engine.transcribe(&seg)`
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add --progressive flag and progressive benchmark mode</name>
  <files>src-tauri/src/bin/benchmark.rs</files>
  <action>
Add progressive mode to the benchmark binary. All changes in benchmark.rs:

1. **CLI flag parsing** (near line 543): Add `let mut progressive = false;` and parse `--progressive` alongside existing `--model` and `--cpu` flags. Print "Mode: progressive (simulated real-time dispatch)" in the header when enabled.

2. **New struct `ProgressiveResult`**: Define alongside `BenchResult`:
```rust
struct ProgressiveResult {
    model: String,
    clip: String,
    /// Total wall-clock time for all chunks (batch-equivalent)
    total_ms: u64,
    /// Time from last chunk "becoming available" to final transcription done
    post_release_ms: i64,  // can be negative if transcription finishes before last chunk arrives
    /// Number of VAD chunks
    num_chunks: usize,
    wer: f64,
    first_text: String,
}
```

3. **Add `let mut progressive_results: Vec<ProgressiveResult> = Vec::new();`** alongside the existing `results` vec.

4. **Progressive benchmark function**: Create a generic function that handles progressive dispatch for any engine. This avoids duplicating the simulation logic across all engine sections:

```rust
/// Simulate progressive VAD-driven chunk transcription.
///
/// For each chunk, computes its "available at" time = cumulative audio duration up to chunk end.
/// Transcribes chunks sequentially. If transcription of prior chunks finishes before the next
/// chunk is "available", we sleep the difference (simulating waiting for audio).
/// If transcription runs longer, the next chunk is already available (no wait).
///
/// Returns (total_transcribe_ms, post_release_ms, combined_text, num_chunks).
fn run_progressive<F>(
    samples: &[f32],
    mut transcribe_chunk: F,
) -> (u64, i64, String, usize)
where
    F: FnMut(&[f32]) -> Result<String, String>,
{
    let chunks = vad_chunk_audio(samples);  // always chunk, even short clips
    let num_chunks = chunks.len();

    // Compute each chunk's "available at" time relative to audio start.
    // We need to know where each chunk falls in the original audio timeline.
    // Since vad_chunk_audio splits sequentially and we know the sample rate is 16kHz,
    // track cumulative sample position to determine real-time offset.
    let mut chunk_available_at_ms: Vec<u64> = Vec::with_capacity(num_chunks);
    let mut cumulative_samples: usize = 0;
    for chunk in &chunks {
        cumulative_samples += chunk.len();
        // This chunk's audio ends at cumulative_samples / 16000 seconds
        let available_ms = (cumulative_samples as f64 / 16.0) as u64;
        chunk_available_at_ms.push(available_ms);
    }

    let wall_start = Instant::now();
    let mut combined_text = String::new();
    let mut total_transcribe_ms: u64 = 0;

    for (i, chunk) in chunks.iter().enumerate() {
        // Wait until this chunk "becomes available" (simulate real-time audio arrival)
        let available_at = chunk_available_at_ms[i];
        let elapsed_so_far = wall_start.elapsed().as_millis() as u64;
        if elapsed_so_far < available_at {
            std::thread::sleep(std::time::Duration::from_millis(available_at - elapsed_so_far));
        }

        let chunk_start = Instant::now();
        match transcribe_chunk(chunk) {
            Ok(text) => {
                let chunk_ms = chunk_start.elapsed().as_millis() as u64;
                total_transcribe_ms += chunk_ms;
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    if !combined_text.is_empty() {
                        combined_text.push(' ');
                    }
                    combined_text.push_str(trimmed);
                }
            }
            Err(e) => {
                eprintln!("  PROGRESSIVE ERROR chunk {}: {}", i, e);
            }
        }
    }

    let total_wall_ms = wall_start.elapsed().as_millis() as u64;
    // Last chunk was available at chunk_available_at_ms[last]
    // Transcription finished at total_wall_ms
    // Post-release latency = total_wall_ms - last_chunk_available_at
    let last_available = *chunk_available_at_ms.last().unwrap_or(&0);
    let post_release_ms = total_wall_ms as i64 - last_available as i64;

    (total_transcribe_ms, post_release_ms, combined_text, num_chunks)
}
```

5. **Wire progressive mode into each engine section**: After the existing batch benchmark loop for each engine (Whisper, Parakeet, Moonshine tiny/base, Streaming moonshine, SenseVoice), add a conditional block:

```rust
if progressive {
    println!("  Progressive mode:");
    // Run 1 iteration (progressive simulates real-time, multiple iterations waste time)
    let (total_ms, post_release_ms, text, num_chunks) = run_progressive(&audio, |chunk| {
        // Engine-specific transcription call for one chunk
        // ... (varies per engine)
    });
    let reference = reference_for_clip(clip_label);
    let (wer, _, _, _, _) = compute_wer(reference, &text);
    println!("    {} chunks, total={}ms, post-release={}ms, WER={:.1}%",
        num_chunks, total_ms, post_release_ms, wer);
    println!("    Text: \"{}\"", truncate(&text, 80));
    progressive_results.push(ProgressiveResult {
        model: model_label.to_string(),  // or appropriate label
        clip: clip_label.to_string(),
        total_ms,
        post_release_ms,
        num_chunks,
        wer,
        first_text: text,
    });
}
```

For each engine's closure:
- **Whisper**: Create state from ctx, set params, call `state.full(params, chunk)`, collect segment text
- **Parakeet**: Call `parakeet.transcribe(chunk)`
- **Moonshine tiny/base**: Call `engine.transcribe(chunk)`
- **Streaming moonshine**: Feed frames incrementally per chunk (same as current streaming bench), collect text
- **SenseVoice**: Call `engine.transcribe(chunk)`

6. **Output**: Pass `&progressive_results` to `print_summary` and `write_markdown_report`. Add new sections when progressive_results is non-empty:

In `print_summary()` -- add after existing tables:
```
PROGRESSIVE vs BATCH COMPARISON
Model                          | Clip | Batch (ms) | Prog Total | Post-Release | Chunks | dWER
---
{model} | {clip} | {batch_avg} | {prog_total} | {post_release}ms | {chunks} | {prog_wer - batch_wer}
```

Add a "POST-RELEASE LATENCY BY DURATION" pivot table matching the existing pivot format -- rows = models, columns = 5s/30s/60s/90s, values = avg post-release ms across clip variants.

In `write_markdown_report()` -- add equivalent markdown tables.

Update function signatures: `print_summary(results: &[BenchResult], progressive: &[ProgressiveResult])` and `write_markdown_report(results: &[BenchResult], progressive: &[ProgressiveResult])`. Update the call sites at the end of main().

IMPORTANT: The `run_progressive` function must use `std::thread::sleep` for the real-time simulation wait (not tokio -- this is a sync binary). The sleep simulates waiting for audio to arrive in real-time.

IMPORTANT: When `--progressive` is passed WITHOUT `--model` filter, run both batch and progressive for all models. Progressive runs after the batch iterations for each clip within each engine section. This keeps results paired for comparison.

IMPORTANT: VAD chunking in progressive mode should ALWAYS chunk (even 5s clips) since in real recording mode, VAD would always be active. Don't gate on `needs_chunking` / `> 30s` for progressive -- call `vad_chunk_audio()` unconditionally.
  </action>
  <verify>
    <automated>cd src-tauri && cargo check --bin benchmark --features whisper,parakeet,bench_extra --release 2>&1 | tail -5</automated>
  </verify>
  <done>
    - `--progressive` flag parses without error
    - Progressive benchmark runs for all engine types, producing ProgressiveResult entries
    - Post-release latency calculated correctly (wall time minus last chunk availability time)
    - Comparison table printed showing batch vs progressive side by side
    - Post-release latency pivot table by duration group
    - Without --progressive, output is identical to before
    - cargo check passes with all features
  </done>
</task>

</tasks>

<verification>
1. `cd src-tauri && cargo check --bin benchmark --features whisper,parakeet,bench_extra --release` -- compiles clean
2. `cd src-tauri && cargo run --bin benchmark --features whisper --release -- --model whisper-small --progressive` -- runs progressive mode on one model, shows comparison table
3. Without `--progressive`, existing output format unchanged
</verification>

<success_criteria>
- benchmark binary accepts --progressive flag
- Progressive mode VAD-chunks every clip, simulates real-time chunk arrival, transcribes sequentially
- Post-release latency (time from last chunk available to transcription done) is the key metric
- Comparison table pairs batch and progressive results for each model/clip
- Pivot table shows post-release latency by duration group
- No change to existing batch-only behavior when flag omitted
</success_criteria>

<output>
After completion, create `.planning/quick/37-add-progressive-flag-to-benchmark-binary/37-SUMMARY.md`
</output>
