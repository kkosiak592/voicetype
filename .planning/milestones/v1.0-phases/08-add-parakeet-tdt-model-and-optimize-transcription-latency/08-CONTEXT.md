# Phase 8: Add Parakeet TDT Model and Optimize Transcription Latency - Context

**Gathered:** 2026-03-01
**Status:** Ready for planning

<domain>
## Phase Boundary

Add NVIDIA Parakeet TDT as a selectable transcription engine alongside Whisper for GPU users, targeting sub-500ms post-release latency. Apply pipeline micro-optimizations to reduce non-inference overhead. CPU users remain on Whisper only.

</domain>

<decisions>
## Implementation Decisions

### Engine coexistence
- Both Parakeet and Whisper are selectable for GPU users — not a replacement, a choice
- Whisper remains the default engine for GPU users (accuracy-first)
- CPU users continue to use Whisper small-en only (Parakeet requires CUDA)
- Engine selector lives in the settings dialog under the model sidebar section
- Hot-swap vs restart on engine switch: Claude's discretion

### Model management & first-run
- Parakeet appears as a third model card in FirstRun.tsx alongside Large v3 Turbo and Small English
- GPU users see three cards; CPU users see only the Small English card
- Which card is "Recommended" for GPU users: Claude's discretion (should align with default engine decision)
- Multiple models can coexist on disk or one-at-a-time: Claude's discretion
- Model hosting source (HuggingFace vs other): Claude's discretion

### Vocabulary biasing
- Corrections engine always applies regardless of active engine — profiles still work
- initial_prompt is skipped for Parakeet (not supported), active for Whisper
- ALL CAPS mode still applies for Parakeet
- How to handle the vocabulary biasing gap (warn user, corrections-only, etc.): Claude's discretion
- Whether to pre-populate Parakeet-specific corrections: Claude's discretion

### Micro-optimizations
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

</decisions>

<specifics>
## Specific Ideas

- User explicitly wants corrections applied to Parakeet output — profiles must remain engine-independent
- Engine selector should feel like part of the existing model settings, not a separate section
- Research artifact exists with full latency breakdown and parakeet-rs integration sketch: `artifacts/research/2026-03-01-sub-500ms-transcription-latency-technical.md`
- Todo exists with implementation intent: `.planning/todos/pending/2026-03-01-implement-sub-500ms-transcription-latency-improvements.md`

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `transcribe.rs`: Whisper inference wrapper — pattern to follow for Parakeet wrapper (load context, transcribe audio, return text)
- `download.rs`: Streaming model download with SHA256 + Channel events — extend `model_info()` with Parakeet entry
- `FirstRun.tsx`: Model card grid — add third card for Parakeet
- `pipeline.rs`: Pipeline orchestration — swap transcription call based on active engine
- `corrections.rs` + `profiles.rs`: Corrections and profile system — already engine-independent except initial_prompt

### Established Patterns
- WhisperStateMutex (Arc<Mutex<Option<WhisperContext>>>) in lib.rs — similar managed state needed for Parakeet transcriber
- Feature flags: `#[cfg(feature = "whisper")]` gates whisper-specific code — may need `#[cfg(feature = "parakeet")]` or runtime engine enum
- Model files stored in `%APPDATA%/VoiceType/models/` — Parakeet ONNX files go here too
- download_model command uses Channel<DownloadEvent> for streaming progress — reuse for Parakeet model

### Integration Points
- `lib.rs`: App setup — init Parakeet transcriber alongside or instead of Whisper based on settings
- `pipeline.rs:122-159`: Whisper inference block — needs engine dispatch (if parakeet { ... } else { whisper ... })
- `download.rs:51-65`: `model_info()` match — add Parakeet model entry
- `FirstRun.tsx:18-33`: MODELS array — add Parakeet model definition
- Settings UI ModelSelector — add engine toggle/dropdown

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 08-add-parakeet-tdt-model-and-optimize-transcription-latency*
*Context gathered: 2026-03-01*
