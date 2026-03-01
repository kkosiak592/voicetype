---
phase: 05-vad-toggle-mode
verified: 2026-02-28T00:00:00Z
status: human_needed
score: 8/8 must-haves verified
re_verification: false
human_verification:
  - test: "Hold-to-talk mode works end-to-end with real microphone"
    expected: "Hold hotkey, speak a sentence, release — text injected at cursor. No regression from pre-phase behavior."
    why_human: "Requires real microphone, running app, and focus target to observe clipboard injection"
  - test: "Toggle mode auto-stop on silence"
    expected: "Tap hotkey, speak, pause ~3.0s — recording auto-stops, pill switches to processing, text appears"
    why_human: "Requires real-time VAD running on live microphone input; cannot verify programmatically"
  - test: "Toggle mode second tap = instant stop"
    expected: "Tap hotkey, begin speaking, tap hotkey again — recording stops immediately, transcribes without waiting for silence"
    why_human: "Requires hardware interaction and real-time state observation"
  - test: "VAD silence gate rejects non-speech in toggle mode"
    expected: "Tap hotkey, say nothing, wait ~3.0s — auto-stops but no text is injected (pill shows error flash)"
    why_human: "Requires running VAD worker reading live audio buffer"
  - test: "Recording mode persists across app restarts"
    expected: "Set mode to Toggle in settings, quit app, relaunch — settings panel still shows Toggle selected"
    why_human: "Requires app lifecycle testing; settings.json write/read path involves filesystem"
  - test: "No double-transcription on second tap"
    expected: "Second tap cancels the VAD worker; transcription runs exactly once"
    why_human: "Race condition prevention requires real-time observation of parallel async tasks"
  - test: "60-second safety cap"
    expected: "Start toggle mode, speak continuously for 60s — recording auto-stops and transcribes"
    why_human: "Requires sustained real-time recording to observe MAX_RECORDING_FRAMES behavior"
---

# Phase 05: VAD + Toggle Mode Verification Report

**Phase Goal:** Integrate Silero VAD for speech detection gating; add toggle recording mode alongside existing hold-to-talk
**Verified:** 2026-02-28
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

Plan 01 (REC-03) must-haves:

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | If user activates dictation and says nothing, whisper does not run and no text is injected | VERIFIED | `vad_gate_check()` returns false when speech_frames < MIN_SPEECH_FRAMES (9); `run_pipeline()` emits pill-result error and returns early at line 71-79 of pipeline.rs |
| 2 | Short non-speech sounds (coughs, clicks, breaths under 300ms) are discarded without running whisper | VERIFIED | MIN_SPEECH_FRAMES=9 @ 32ms/chunk = 288ms threshold; post-hoc gate iterates all 512-sample chunks and counts only speech-classified frames |
| 3 | Normal speech recordings continue to transcribe correctly (VAD gate does not reject valid speech) | VERIFIED (automated) / HUMAN for end-to-end | Gate returns true when speech_frames >= 9; code path to whisper inference is unchanged for passing recordings. End-to-end requires human |

Plan 02 (REC-02, REC-04) must-haves:

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 4 | User can tap the hotkey to start recording, speak, pause ~3.0s, and recording auto-stops and transcribes | VERIFIED (code) / HUMAN (runtime) | spawn_vad_worker() called on toggle IDLE->RECORDING; SILENCE_FRAMES_THRESHOLD=94 (3.0s); trigger_auto_stop() calls run_pipeline() via crate path |
| 5 | User can tap the hotkey a second time to stop recording early (instant hard stop, straight to transcription) | VERIFIED (code) / HUMAN (runtime) | Second tap CAS RECORDING->PROCESSING in setup() handler (lib.rs:618); VAD worker cancelled before pipeline spawn |
| 6 | User can switch between hold-to-talk and toggle mode in settings, and the selection persists across restarts | VERIFIED (code) / HUMAN (runtime) | RecordingModeToggle.tsx invokes set_recording_mode; lib.rs persists to settings.json; read_saved_mode() restores on startup |
| 7 | Hold-to-talk mode continues to work exactly as before (no regression) | VERIFIED (code) / HUMAN (runtime) | Mode::HoldToTalk branch in Pressed/Released handlers is unchanged from pre-phase logic; toggle mode paths are separate branches |
| 8 | OS key repeat events during hold do not cause double-stop in toggle mode | VERIFIED | CAS transition(RECORDING, PROCESSING) is atomic — only one concurrent key-repeat event can succeed; others see state != RECORDING and are no-ops |

**Score:** 8/8 truths verified at code level. 7 require human verification for runtime correctness.

### Required Artifacts

#### Plan 01 Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src-tauri/src/vad.rs` | VadWorker struct, vad_gate_check(), VadWorkerHandle | VERIFIED | 292 lines; exports vad_gate_check (line 41), VadWorkerHandle (line 86), spawn_vad_worker (line 117); all substantive |
| `src-tauri/src/pipeline.rs` | run_pipeline() with VAD gate replacing 1600-sample minimum | VERIFIED | vad::vad_gate_check called at line 71; old `samples.len() < 1600` is absent; cancel_stale_vad_worker at line 57 |
| `src-tauri/Cargo.toml` | voice_activity_detector dependency | VERIFIED | `voice_activity_detector = "0.2.1"` at line 49 |

#### Plan 02 Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src-tauri/src/lib.rs` | RecordingMode managed state, mode-aware hotkey handlers, VadWorkerHandle managed state, set_recording_mode and get_recording_mode commands | VERIFIED | RecordingMode (line 34), VadWorkerState (line 55), read_saved_mode (line 81), set_recording_mode (line 104), get_recording_mode (line 126), both registered in invoke_handler (lines 483-484), managed in setup() (lines 542-545) |
| `src-tauri/src/pipeline.rs` | cancel_vad_worker() helper called from run_pipeline() | VERIFIED | cancel_stale_vad_worker() defined (line 193), called at line 57 of run_pipeline() |
| `src/components/RecordingModeToggle.tsx` | React toggle component for hold-to-talk vs toggle mode selection | VERIFIED | 73 lines; radio-card UI with two options, invoke('set_recording_mode'), store persistence; fully substantive |
| `src/App.tsx` | RecordingModeToggle wired into settings panel | VERIFIED | Imported at line 5, recordingMode state (line 11), loaded from store (line 19+33), rendered at line 78 |
| `src/lib/store.ts` | recordingMode field in AppSettings and DEFAULTS | VERIFIED | recordingMode: 'hold' \| 'toggle' in interface (line 7), DEFAULTS.recordingMode = 'hold' (line 14) |

### Key Link Verification

#### Plan 01 Key Links

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src-tauri/src/pipeline.rs` | `src-tauri/src/vad.rs` | vad::vad_gate_check() call | WIRED | `use crate::vad;` at pipeline.rs:4; `vad::vad_gate_check(&samples)` at pipeline.rs:71 |
| `src-tauri/src/vad.rs` | voice_activity_detector crate | VoiceActivityDetector::builder().sample_rate(16000).chunk_size(512).build() | WIRED | `use voice_activity_detector::VoiceActivityDetector` at vad.rs:5; builder call in both vad_gate_check (line 42) and spawn_vad_worker (line 127) |
| `src-tauri/src/vad.rs` | `src-tauri/src/pipeline.rs` | crate::pipeline::run_pipeline via inline crate path (no circular use-import) | WIRED | vad.rs has no `use crate::pipeline;` top-level import; references pipeline via `crate::pipeline::PipelineState` (line 233), `crate::pipeline::RECORDING/IDLE/PROCESSING` (lines 239, 264), `crate::pipeline::run_pipeline` (line 289) |

#### Plan 02 Key Links

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src-tauri/src/lib.rs` | `src-tauri/src/vad.rs` | vad::spawn_vad_worker() called on recording start in toggle mode | WIRED | lib.rs line 200/608: `let vad_handle = vad::spawn_vad_worker(app.clone(), audio.buffer.clone())` in both setup() and rebind_hotkey() toggle handlers |
| `src-tauri/src/lib.rs` | settings.json | read_saved_mode() at startup, set_recording_mode command for saves | WIRED | read_saved_mode reads `recording_mode` key (lib.rs:97); set_recording_mode writes it via serde_json merge (lib.rs:118); called in setup() at line 542 |
| `src/components/RecordingModeToggle.tsx` | `src-tauri/src/lib.rs` | invoke('set_recording_mode') Tauri command | WIRED | RecordingModeToggle.tsx:31: `await invoke('set_recording_mode', { mode })`; command registered in invoke_handler at lib.rs:483 |
| `src-tauri/src/lib.rs` | `src-tauri/src/pipeline.rs` | VadWorkerHandle cancel on second tap before run_pipeline() | WIRED | lib.rs:213-218 (setup handler) and lib.rs equivalent in rebind_hotkey handler: guard.take() + handle.cancel() before pipeline spawn |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| REC-02 | 05-02 | User can tap the hotkey to start recording and tap again to stop (toggle mode) | SATISFIED | Mode::Toggle handler in setup() and rebind_hotkey(): CAS IDLE->RECORDING on first tap, CAS RECORDING->PROCESSING on second tap; VAD worker spawned on first tap |
| REC-03 | 05-01 | In toggle mode, Silero VAD automatically detects silence and stops recording | SATISFIED | spawn_vad_worker() runs streaming VAD; SILENCE_FRAMES_THRESHOLD=94 (3.0s); trigger_auto_stop() called on silence threshold; also covers hold-to-talk via vad_gate_check post-hoc gate |
| REC-04 | 05-02 | User can switch between hold-to-talk and toggle mode in settings | SATISFIED | RecordingModeToggle.tsx rendered in App.tsx settings panel; invokes set_recording_mode; persists to settings.json; read_saved_mode() restores on startup |

No orphaned requirements. REQUIREMENTS.md traceability table maps REC-02, REC-03, REC-04 all to Phase 5 — all three are claimed by plans 05-01 and 05-02.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| None found | — | — | — | — |

Scanned all 6 phase-modified files for: TODO/FIXME/HACK/PLACEHOLDER comments, empty return values (return null, return {}, return []), console.log-only handlers, no-op event handlers. Zero hits.

One notable non-issue: the comment at vad.rs:115 contains the string "use crate::pipeline;" only as documentation warning text, not as an actual import. The module top level has no such import. This is correct behavior per the plan.

### Human Verification Required

The following items require running the application with a real microphone and keyboard. All code paths are correctly wired — these verify runtime behavior only.

#### 1. Hold-to-talk mode (regression check)

**Test:** Build and run with `npx tauri dev --features whisper`. Hold the hotkey, speak a sentence, release. Observe pill behavior and cursor.
**Expected:** Pill shows recording state while held, switches to processing on release, text appears at cursor. No spurious transcriptions or stuck states.
**Why human:** Requires live audio capture, GPU/CPU whisper inference, clipboard injection — all I/O-bound and unverifiable statically.

#### 2. Toggle mode auto-stop on silence

**Test:** In settings, select "Toggle" mode. Tap hotkey, speak a sentence, stop speaking and wait.
**Expected:** Pill appears on tap, recording starts. After ~3 seconds of silence, pill switches to processing without user input, then text appears.
**Why human:** VAD worker polls a live audio buffer in an async task; silence detection timing requires real audio hardware.

#### 3. Toggle mode second tap

**Test:** In toggle mode, tap hotkey, speak briefly, tap hotkey again immediately (before silence timeout).
**Expected:** Recording stops immediately on second tap, transcription runs, text injected. No 3s silence wait.
**Why human:** Tests CAS race between second-tap handler and VAD worker; requires real-time observation.

#### 4. VAD gate rejects silence in toggle mode

**Test:** In toggle mode, tap hotkey, say nothing, wait ~3 seconds.
**Expected:** Auto-stop fires (VAD detects silence before ever_spoke=true), but VAD discards (insufficient speech), pill shows error flash, no text injected.
**Why human:** Validates the insufficient-speech discard path in trigger_auto_stop(); requires real audio input to verify the ever_spoke/speech_frames counters.

#### 5. Settings persistence across restarts

**Test:** Set mode to Toggle in settings panel. Quit the app via tray. Relaunch. Open settings.
**Expected:** Settings panel shows "Toggle" selected. App behavior uses toggle mode immediately without any additional user action.
**Why human:** Tests the read_saved_mode() + settings.json round-trip across a process boundary.

#### 6. No double-transcription (VAD worker cancellation)

**Test:** In toggle mode, tap to start, speak briefly, tap again quickly.
**Expected:** Exactly one transcription result appears. Logs should show "VAD worker: cancelled externally" (not a second pipeline trigger).
**Why human:** Validates the cancel channel oneshot behavior and CAS protection against concurrent pipeline triggers.

### Gaps Summary

No gaps found. All code-level checks pass.

The human_needed status reflects that 6 of the 7 observable truths have runtime-only components (live audio, real-time async tasks, app restarts) that cannot be verified statically. The implementation correctly wires all paths — human testing confirms they function correctly in the real environment.

The SUMMARY already documents that Task 3 of Plan 02 was a human-verify checkpoint and the user confirmed all 8 verification steps passed, including toggle mode working end-to-end and the 1.5s silence timeout being adjusted to 3.0s based on real-world feedback. The human verification items listed here are a formal record for traceability.

---

_Verified: 2026-02-28_
_Verifier: Claude (gsd-verifier)_
