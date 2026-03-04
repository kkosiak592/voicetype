---
phase: quick-34
plan: 34
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/src/bin/benchmark.rs
autonomous: true
requirements: []
must_haves:
  truths:
    - "Parakeet 60s clips are VAD-chunked into segments before transcription"
    - "Parakeet 5s and 30s clips pass through without chunking (<=30s threshold)"
    - "VAD chunking time and segment count are printed for chunked clips"
    - "Chunked segment texts are concatenated with spaces, matching moonshine pattern"
  artifacts:
    - path: "src-tauri/src/bin/benchmark.rs"
      provides: "VAD chunking for parakeet benchmark section"
      contains: "vad_chunk_audio"
  key_links:
    - from: "vad_chunk_audio"
      to: "parakeet benchmark loop"
      via: "conditional chunking for audio > 30s"
      pattern: "needs_chunking.*vad_chunk_audio"
---

<objective>
Add VAD-based chunking to the parakeet-tdt-v2 benchmark section so clips >30s are split at silence boundaries before transcription.

Purpose: Parakeet's encoder has quadratic attention cost. Splitting 60s audio into ~20-25s segments should yield ~40-50% speedup on long clips. The pattern is already proven in 5 other model sections (moonshine-tiny, moonshine-base, sensevoice, moonshine-streaming-tiny/small/medium).

Output: Modified benchmark.rs where parakeet uses the same VAD chunking as other models.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src-tauri/src/bin/benchmark.rs
</context>

<tasks>

<task type="auto">
  <name>Task 1: Widen vad_chunk_audio cfg gate and add VAD chunking to parakeet section</name>
  <files>src-tauri/src/bin/benchmark.rs</files>
  <action>
Two changes in benchmark.rs:

**Change 1: Widen cfg gates so vad_chunk_audio is available when parakeet feature is enabled.**

The `use voice_activity_detector::VoiceActivityDetector;` import (line ~31) is currently gated by `#[cfg(feature = "bench_extra")]`. Change it to:
```rust
#[cfg(any(feature = "bench_extra", feature = "parakeet"))]
use voice_activity_detector::VoiceActivityDetector;
```

The `fn vad_chunk_audio` definition (line ~128-129) is currently gated by `#[cfg(feature = "bench_extra")]`. Change it to:
```rust
#[cfg(any(feature = "bench_extra", feature = "parakeet"))]
fn vad_chunk_audio(samples: &[f32]) -> Vec<Vec<f32>> {
```

This works because `voice_activity_detector` is a non-optional dependency in Cargo.toml (always compiled).

**Change 2: Add VAD chunking to the parakeet benchmark loop.**

In the parakeet section (inside `#[cfg(feature = "parakeet")] if parakeet_found ...`), the current per-clip loop (starting around line 799) reads the WAV and immediately iterates ITERATIONS times calling `parakeet.transcribe_samples(audio.clone(), ...)`.

Replace the inner loop body with the same chunking pattern used by moonshine/sensevoice. After reading the WAV and before the iteration loop, add:

```rust
let needs_chunking = audio.len() > 30 * 16000; // > 30 seconds
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

Then replace the iteration loop to iterate over chunks per run:

```rust
for i in 0..ITERATIONS {
    let t = Instant::now();
    let mut combined_text = String::new();
    let mut had_error = false;

    for (seg_idx, seg) in chunks.iter().enumerate() {
        match parakeet.transcribe_samples(
            seg.clone(),
            16000,
            1,
            Some(TimestampMode::Sentences),
        ) {
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
        first_text = combined_text.clone();
        println!(
            "  [run 1] {}ms — \"{}\"",
            elapsed,
            truncate(&first_text, 80)
        );
    } else {
        println!("  [run {}] {}ms", i + 1, elapsed);
    }
}
```

Note: The rest of the parakeet section (WER computation, results.push) stays the same. Only the audio reading -> iteration loop changes.
  </action>
  <verify>
    <automated>cd /c/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text/src-tauri && cargo check --bin benchmark --features whisper,parakeet --release 2>&1 | tail -5</automated>
  </verify>
  <done>
    - cargo check passes with no errors for --features whisper,parakeet (without bench_extra)
    - cargo check passes with --features whisper,parakeet,bench_extra
    - Parakeet benchmark section uses vad_chunk_audio for clips >30s
    - Clips <=30s pass through as single-element vec (no chunking overhead)
    - Combined text from chunks is space-joined, matching moonshine/sensevoice pattern
  </done>
</task>

</tasks>

<verification>
- `cargo check --bin benchmark --features whisper,parakeet --release` compiles cleanly
- `cargo check --bin benchmark --features whisper,parakeet,bench_extra --release` compiles cleanly (no duplicate symbol from widened cfg gate)
- The parakeet section's chunking code mirrors the moonshine pattern exactly (same variable names, same print format, same text concatenation logic)
</verification>

<success_criteria>
Parakeet benchmark section VAD-chunks audio >30s before transcription, using the same vad_chunk_audio function as moonshine/sensevoice. Both feature combinations compile cleanly.
</success_criteria>

<output>
After completion, create `.planning/quick/34-add-vad-based-chunking-to-parakeet-model/34-SUMMARY.md`
</output>
