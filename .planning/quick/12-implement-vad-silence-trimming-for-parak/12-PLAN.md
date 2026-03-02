---
phase: quick-12
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/src/vad.rs
  - src-tauri/src/pipeline.rs
autonomous: true
requirements: [QUICK-12]

must_haves:
  truths:
    - "Audio sent to transcription engine has leading silence trimmed"
    - "Audio sent to transcription engine has trailing silence trimmed"
    - "Padding of 1 chunk (512 samples) preserved around speech boundaries"
    - "Full buffer returned when no speech chunks detected (fail-open)"
    - "Trim applies to both Whisper and Parakeet engines"
    - "Trim ratio logged for debugging"
  artifacts:
    - path: "src-tauri/src/vad.rs"
      provides: "vad_trim_silence() function"
      contains: "pub fn vad_trim_silence"
    - path: "src-tauri/src/pipeline.rs"
      provides: "Trim integration between speech gate and engine dispatch"
      contains: "vad::vad_trim_silence"
  key_links:
    - from: "src-tauri/src/pipeline.rs"
      to: "src-tauri/src/vad.rs"
      via: "vad::vad_trim_silence(&samples) call"
      pattern: "vad::vad_trim_silence"
    - from: "src-tauri/src/vad.rs"
      to: "voice_activity_detector::VoiceActivityDetector"
      via: "Fresh VAD instance for trim pass"
      pattern: "VoiceActivityDetector::builder"
---

<objective>
Add VAD-based silence trimming to the audio pipeline so leading and trailing silence is removed before sending audio to the transcription engine.

Purpose: Parakeet (and Whisper) receive cleaner audio without silence padding, improving transcription accuracy — especially for short utterances where silence may dominate the buffer.
Output: `vad_trim_silence()` in vad.rs, integrated into pipeline.rs between speech gate and engine dispatch.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@src-tauri/src/vad.rs
@src-tauri/src/pipeline.rs
@src-tauri/src/transcribe_parakeet.rs

<interfaces>
<!-- From src-tauri/src/vad.rs — existing patterns to follow -->

```rust
// Constants already defined (reuse these):
const SPEECH_PROBABILITY_THRESHOLD: f32 = 0.5;
const CHUNK_SIZE: usize = 512;

// Existing function with identical VAD init pattern:
pub fn vad_gate_check(samples: &[f32]) -> bool { ... }
```

<!-- From src-tauri/src/pipeline.rs — insertion point -->

```rust
// After speech gate check (line ~127), before engine dispatch (line ~137):
// `samples` is Vec<f32>, owned, ready to be trimmed.
// The trimmed result replaces `samples` via `let samples = vad::vad_trim_silence(&samples);`
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add vad_trim_silence() function to vad.rs</name>
  <files>src-tauri/src/vad.rs</files>
  <action>
Add a new public function `vad_trim_silence` to `src-tauri/src/vad.rs`, placed between `vad_gate_check()` (line 77) and the streaming VAD worker section (line 79 comment).

Implementation:

```rust
/// Trims leading and trailing silence from an audio buffer using Silero VAD.
///
/// Creates a fresh VoiceActivityDetector, processes the buffer in CHUNK_SIZE (512)
/// sample chunks, finds the first and last chunks with speech probability >= 0.5,
/// then returns a slice with 1-chunk padding on each side.
///
/// Falls back to returning the full buffer if:
/// - VAD initialization fails (fail-open)
/// - No speech chunks detected (entire buffer below threshold)
///
/// Cost: ~2-5ms CPU for a typical dictation buffer (1-10 seconds of audio).
pub fn vad_trim_silence(samples: &[f32]) -> Vec<f32> {
    let mut vad = match VoiceActivityDetector::builder()
        .sample_rate(16000u32)
        .chunk_size(CHUNK_SIZE)
        .build()
    {
        Ok(v) => v,
        Err(e) => {
            log::error!("VAD trim: failed to initialize VoiceActivityDetector: {}", e);
            return samples.to_vec();
        }
    };

    let total_chunks = samples.len() / CHUNK_SIZE;
    let mut first_speech: Option<usize> = None;
    let mut last_speech: Option<usize> = None;

    for (i, chunk) in samples.chunks(CHUNK_SIZE).enumerate() {
        if chunk.len() < CHUNK_SIZE {
            break; // partial final chunk — skip
        }
        let prob = vad.predict(chunk.to_vec());
        if prob >= SPEECH_PROBABILITY_THRESHOLD {
            if first_speech.is_none() {
                first_speech = Some(i);
            }
            last_speech = Some(i);
        }
    }

    let (first, last) = match (first_speech, last_speech) {
        (Some(f), Some(l)) => (f, l),
        _ => {
            log::info!("VAD trim: no speech detected in {} chunks, returning full buffer", total_chunks);
            return samples.to_vec();
        }
    };

    // Apply 1-chunk padding on each side (clamped to buffer bounds)
    let start_chunk = if first > 0 { first - 1 } else { 0 };
    let end_chunk = if last + 1 < total_chunks { last + 1 } else { last };

    let start_sample = start_chunk * CHUNK_SIZE;
    let end_sample = std::cmp::min((end_chunk + 1) * CHUNK_SIZE, samples.len());

    log::info!(
        "VAD trim: speech chunks {}-{} of {} (padded: {}-{}), trimmed {:.1}% ({} -> {} samples)",
        first,
        last,
        total_chunks,
        start_chunk,
        end_chunk,
        (1.0 - (end_sample - start_sample) as f64 / samples.len() as f64) * 100.0,
        samples.len(),
        end_sample - start_sample
    );

    samples[start_sample..end_sample].to_vec()
}
```

Key design points:
- Fresh VoiceActivityDetector per call (no stale LSTM state) — same pattern as `vad_gate_check`
- Uses existing `CHUNK_SIZE` (512) and `SPEECH_PROBABILITY_THRESHOLD` (0.5) constants
- 1-chunk padding prevents clipping speech onset/offset
- Fail-open: returns full buffer on VAD init failure or no-speech detection
- Returns `Vec<f32>` (owned) since pipeline.rs needs to own the trimmed buffer
- Logs trim percentage for debugging
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && cargo check -p voice-to-text --features "whisper,parakeet" 2>&1 | tail -5</automated>
  </verify>
  <done>vad_trim_silence() compiles, takes &[f32] and returns Vec&lt;f32&gt;, uses fresh VAD instance, finds first/last speech chunks, applies 1-chunk padding, falls back to full buffer on no-speech</done>
</task>

<task type="auto">
  <name>Task 2: Integrate vad_trim_silence into pipeline.rs</name>
  <files>src-tauri/src/pipeline.rs</files>
  <action>
In `src-tauri/src/pipeline.rs`, insert the VAD trim call between the speech gate log (line 129-133) and the active engine read (line 137). The `samples` binding must be shadowed with the trimmed result.

After line 134 (`let _ = sample_count;`), add:

```rust
    // VAD silence trim: remove leading/trailing silence before engine dispatch.
    // Applies to both Whisper and Parakeet — engine-agnostic improvement.
    // Cost: ~2-5ms (one Silero VAD pass over the buffer).
    let samples = vad::vad_trim_silence(&samples);
```

This shadows the existing `samples: Vec<f32>` with the trimmed version. The rest of the pipeline (engine dispatch, text processing, injection) operates on the trimmed buffer without any other changes.

The `vad::` path is already imported at the top of pipeline.rs (line 4: `use crate::vad;`), so no new imports needed.

Important: This single line is the entire integration. Do NOT modify the speech gate logic, engine dispatch, or any other pipeline step. The trim is purely a buffer preprocessing step.
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && cargo check -p voice-to-text --features "whisper,parakeet" 2>&1 | tail -5</automated>
  </verify>
  <done>Pipeline calls vad_trim_silence() after speech gate and before engine dispatch. The trimmed samples buffer is used by both Whisper and Parakeet engines. cargo check passes with no errors.</done>
</task>

</tasks>

<verification>
1. `cargo check -p voice-to-text --features "whisper,parakeet"` passes — no compilation errors
2. `cargo build -p voice-to-text --features "whisper,parakeet" --release` succeeds — full release build
3. In vad.rs: `vad_trim_silence` is `pub`, takes `&[f32]`, returns `Vec<f32>`
4. In pipeline.rs: `vad::vad_trim_silence(&samples)` appears between speech gate and engine dispatch
5. Log output includes "VAD trim:" with chunk range and trim percentage
</verification>

<success_criteria>
- vad_trim_silence() function exists in vad.rs with correct signature and behavior
- pipeline.rs calls vad_trim_silence() exactly once, after speech gate, before engine dispatch
- Both Whisper and Parakeet code paths receive the trimmed buffer (single trim point, not per-engine)
- Full release build compiles without errors or warnings
</success_criteria>

<output>
After completion, create `.planning/quick/12-implement-vad-silence-trimming-for-parak/12-SUMMARY.md`
</output>
