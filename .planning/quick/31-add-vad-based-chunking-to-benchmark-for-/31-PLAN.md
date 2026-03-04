---
phase: quick-31
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/src/bin/benchmark.rs
autonomous: true
requirements: [VAD-CHUNK-BENCH]

must_haves:
  truths:
    - "Moonshine/SenseVoice benchmarks on 30s+ clips use VAD-based chunking"
    - "Short clips (<= 30s) bypass chunking and go directly to the model"
    - "VAD chunking time is included in the measured latency"
    - "Chunked transcription text is concatenated and WER is computed on the full result"
  artifacts:
    - path: "src-tauri/src/bin/benchmark.rs"
      provides: "vad_chunk_audio function and updated bench_extra loops"
      contains: "fn vad_chunk_audio"
  key_links:
    - from: "vad_chunk_audio"
      to: "VoiceActivityDetector"
      via: "voice_activity_detector crate predict()"
      pattern: "vad\\.predict"
    - from: "bench_extra moonshine/sensevoice loops"
      to: "vad_chunk_audio"
      via: "conditional call when audio.len() > 30s threshold"
      pattern: "vad_chunk_audio"
---

<objective>
Add a VAD-based audio chunking function to benchmark.rs and wire it into the Moonshine tiny, Moonshine base, and SenseVoice benchmark loops so that clips longer than 30 seconds are split at silence boundaries before transcription.

Purpose: Moonshine (trained on 4-30s segments) and SenseVoice (30s inference limit) produce garbage on long audio. The official approach is VAD segmentation before inference. Without this, benchmark WER on 60s clips is meaningless.

Output: Updated benchmark.rs with `vad_chunk_audio` function and modified bench_extra loops.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src-tauri/src/bin/benchmark.rs (full file — the only file modified)
@src-tauri/src/vad.rs (reference for VoiceActivityDetector API patterns — do NOT modify)
@src-tauri/Cargo.toml (voice_activity_detector = "0.2.1" already in deps — do NOT modify)
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add vad_chunk_audio function and wire into bench_extra loops</name>
  <files>src-tauri/src/bin/benchmark.rs</files>
  <action>
**1. Add import** (inside the `#[cfg(feature = "bench_extra")]` block, near lines 19-27):
```rust
#[cfg(feature = "bench_extra")]
use voice_activity_detector::VoiceActivityDetector;
```

**2. Add `vad_chunk_audio` function** (place it after `read_wav_to_f32`, around line 109, inside a `#[cfg(feature = "bench_extra")]` gate):

```rust
/// Split audio into VAD-based chunks for models with short context windows.
/// Only called for clips > 30s. Returns Vec of audio segments split at silence boundaries.
///
/// Algorithm:
/// 1. Run Silero VAD over entire audio in 512-sample chunks
/// 2. Track speech/silence state — silence starts when prob < 0.5
/// 3. When silence exceeds SILENCE_GAP_SAMPLES (300ms = 4800 samples at 16kHz),
///    end current segment and start a new one
/// 4. Cap segments at MAX_SEGMENT_SAMPLES (30s = 480000 samples)
/// 5. Discard segments shorter than MIN_SEGMENT_SAMPLES (0.5s = 8000 samples)
#[cfg(feature = "bench_extra")]
fn vad_chunk_audio(samples: &[f32]) -> Vec<Vec<f32>> {
    const CHUNK_SIZE: usize = 512;
    const SPEECH_THRESHOLD: f32 = 0.5;
    const SILENCE_GAP_CHUNKS: usize = 94; // 300ms / 32ms-per-chunk ~= 9.4, but use ~3s like vad.rs? NO — use 300ms for splitting: 300ms / 32ms = ~9 chunks
    // Correction: 300ms at 16kHz = 4800 samples. At 512 samples/chunk = 9.375 chunks. Round to 10.
    const SILENCE_SPLIT_CHUNKS: usize = 10; // ~320ms of silence triggers a split
    const MAX_SEGMENT_SAMPLES: usize = 30 * 16000; // 30 seconds
    const MIN_SEGMENT_SAMPLES: usize = 8000; // 0.5 seconds

    let mut vad = match VoiceActivityDetector::builder()
        .sample_rate(16000u32)
        .chunk_size(CHUNK_SIZE)
        .build()
    {
        Ok(v) => v,
        Err(e) => {
            eprintln!("  VAD chunking failed to init: {} — returning single segment", e);
            return vec![samples.to_vec()];
        }
    };

    // Classify each chunk as speech or silence
    let num_chunks = samples.len() / CHUNK_SIZE;
    let mut is_speech: Vec<bool> = Vec::with_capacity(num_chunks);
    for i in 0..num_chunks {
        let start = i * CHUNK_SIZE;
        let chunk = &samples[start..start + CHUNK_SIZE];
        let prob = vad.predict(chunk.to_vec());
        is_speech.push(prob >= SPEECH_THRESHOLD);
    }

    // Find split points: runs of >= SILENCE_SPLIT_CHUNKS consecutive silence chunks
    let mut segments: Vec<Vec<f32>> = Vec::new();
    let mut seg_start_chunk: usize = 0;
    let mut silence_run: usize = 0;

    for (i, &speech) in is_speech.iter().enumerate() {
        if !speech {
            silence_run += 1;
        } else {
            silence_run = 0;
        }

        let seg_len_samples = (i + 1 - seg_start_chunk) * CHUNK_SIZE;

        // Split if: silence gap reached OR segment exceeds max duration
        let should_split = (silence_run >= SILENCE_SPLIT_CHUNKS && seg_len_samples > MIN_SEGMENT_SAMPLES)
            || seg_len_samples >= MAX_SEGMENT_SAMPLES;

        if should_split && i + 1 < num_chunks {
            // End segment at the start of the silence run (keep speech, trim trailing silence)
            let split_chunk = if silence_run >= SILENCE_SPLIT_CHUNKS {
                i + 1 - silence_run // start of silence run
            } else {
                i + 1 // max length reached — split here
            };

            let start_sample = seg_start_chunk * CHUNK_SIZE;
            let end_sample = std::cmp::min(split_chunk * CHUNK_SIZE, samples.len());

            if end_sample > start_sample && (end_sample - start_sample) >= MIN_SEGMENT_SAMPLES {
                segments.push(samples[start_sample..end_sample].to_vec());
            }

            seg_start_chunk = i + 1; // next chunk starts new segment
            silence_run = 0;
        }
    }

    // Final segment
    let start_sample = seg_start_chunk * CHUNK_SIZE;
    if start_sample < samples.len() {
        let remaining = &samples[start_sample..];
        if remaining.len() >= MIN_SEGMENT_SAMPLES {
            segments.push(remaining.to_vec());
        }
    }

    // Fallback: if chunking produced nothing, return the whole audio
    if segments.is_empty() {
        segments.push(samples.to_vec());
    }

    println!("  VAD chunking: {} segments from {:.1}s audio",
        segments.len(),
        samples.len() as f32 / 16000.0);
    for (i, seg) in segments.iter().enumerate() {
        println!("    segment {}: {:.1}s ({} samples)", i, seg.len() as f32 / 16000.0, seg.len());
    }

    segments
}
```

**3. Modify all three bench_extra loops** (moonshine-tiny ~line 658, moonshine-base ~line 729, sensevoice-small ~line 800).

In each loop, replace the inner iteration body. The current pattern is:
```rust
for (wav_path, clip_label) in &clip_paths {
    let audio = match read_wav_to_f32(wav_path) { ... };
    // ... direct transcribe_samples(audio.clone(), None) ...
}
```

Change to: after loading audio, check if the clip is >30s. If so, chunk it with VAD and transcribe each segment, concatenating text. Time the entire VAD+transcription pipeline. If <=30s, transcribe directly as before.

The modified inner loop body for each model (moonshine-tiny, moonshine-base, sensevoice-small):

```rust
for (wav_path, clip_label) in &clip_paths {
    println!("  Clip: {}", clip_label);
    let audio = match read_wav_to_f32(wav_path) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("  ERROR reading WAV: {}", e);
            continue;
        }
    };

    let needs_chunking = audio.len() > 30 * 16000; // > 30 seconds
    let chunks: Vec<Vec<f32>> = if needs_chunking {
        vad_chunk_audio(&audio)
    } else {
        vec![audio]
    };
    if needs_chunking {
        println!("  (VAD chunking: {} segments for {:.1}s clip)", chunks.len(), chunks.iter().map(|c| c.len()).sum::<usize>() as f32 / 16000.0);
    }

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
        model: "MODEL_NAME_HERE".to_string(), // Replace with actual model name for each loop
        clip: clip_label.to_string(),
        avg_ms: avg,
        min_ms: min,
        max_ms: max,
        wer,
        first_text,
    });
}
```

Use the correct model name string for each loop:
- moonshine-tiny loop: `"moonshine-tiny"`
- moonshine-base loop: `"moonshine-base"`
- sensevoice-small loop: `"sensevoice-small"`

**Important details:**
- The `vad_chunk_audio` call happens OUTSIDE the ITERATIONS loop so VAD runs once per clip, not once per iteration. The chunks are reused across iterations.
- The `Instant::now()` for latency measurement wraps the full segment transcription loop (all chunks), so latency reflects real-world usage (VAD chunking cost is one-time but all segment transcriptions are timed).
- Actually, VAD chunking cost should be measured separately and printed but NOT included in per-iteration latency (since in production VAD runs in real-time during recording, not at transcription time). Keep the current approach of timing only `transcribe_samples` calls.
- Wait — the research context says "Time the whole pipeline (VAD + all segment transcriptions)." So DO include VAD time. But VAD runs once before the iteration loop, and its output (chunks) is reused. So: measure VAD time separately, print it once. For per-iteration timing, only time the transcription of all chunks. Print total = VAD + avg_transcription at the end.

Revised approach: Add VAD timing:
```rust
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

Then after computing avg/min/max from iteration latencies, print:
```rust
if needs_chunking {
    println!("  (total incl. VAD: avg={}ms)", avg + vad_ms);
}
```

This way the results table latency = pure inference (comparable across models), but VAD overhead is visible.
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text/src-tauri && cargo check --bin benchmark --features whisper,parakeet,bench_extra 2>&1 | tail -5</automated>
  </verify>
  <done>
    - `vad_chunk_audio` function exists in benchmark.rs gated behind `#[cfg(feature = "bench_extra")]`
    - All three bench_extra loops (moonshine-tiny, moonshine-base, sensevoice-small) use chunking for clips > 30s
    - Clips <= 30s bypass chunking entirely (direct model call, identical to current behavior)
    - VAD time printed separately; per-iteration latency measures transcription of all segments
    - `cargo check --features whisper,parakeet,bench_extra` compiles without errors
  </done>
</task>

</tasks>

<verification>
- `cargo check --bin benchmark --features whisper,parakeet,bench_extra` succeeds
- Grep for `vad_chunk_audio` finds the function definition and 3 call sites
- Grep for `needs_chunking` confirms the >30s threshold guard in all 3 loops
- The `voice_activity_detector::VoiceActivityDetector` import exists under bench_extra cfg gate
</verification>

<success_criteria>
benchmark.rs compiles with bench_extra feature, contains VAD chunking for Moonshine and SenseVoice on clips >30s, and short clips remain unchanged.
</success_criteria>

<output>
After completion, create `.planning/quick/31-add-vad-based-chunking-to-benchmark-for-/31-SUMMARY.md`
</output>
