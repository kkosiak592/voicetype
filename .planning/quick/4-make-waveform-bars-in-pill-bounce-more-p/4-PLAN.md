---
phase: quick
plan: 4
type: execute
wave: 1
depends_on: []
files_modified:
  - src/components/FrequencyBars.tsx
  - src-tauri/src/pill.rs
autonomous: false
requirements: []
must_haves:
  truths:
    - "Waveform bars visibly bounce during normal speech volume"
    - "Bars reach near-full container height during loud speech"
    - "Clear visual difference between silence and speech"
  artifacts:
    - path: "src/components/FrequencyBars.tsx"
      provides: "Amplified audio-reactive frequency bar animation"
    - path: "src-tauri/src/pill.rs"
      provides: "More aggressive RMS normalization for speech levels"
  key_links:
    - from: "src-tauri/src/pill.rs"
      to: "src/components/FrequencyBars.tsx"
      via: "pill-level event (0.0-1.0 float)"
      pattern: "emit_to.*pill-level"
---

<objective>
Make the waveform bars in the pill overlay bounce more prominently during voice input.

Purpose: Current audio reactivity is too subtle — bars barely move during normal speech, making the recording state feel static. Bars need to respond visibly and energetically to voice input.

Output: Modified FrequencyBars.tsx and pill.rs with amplified audio reactivity.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src/components/FrequencyBars.tsx
@src-tauri/src/pill.rs
@src/Pill.tsx
@src/pill.css

<interfaces>
From src-tauri/src/pill.rs:
- compute_rms() returns f32 clamped to 0.0-1.0, currently using 10x multiplier
- Typical raw speech RMS is 0.01-0.1, so after 10x multiply typical speech lands at 0.1-1.0
- Emitted to frontend as "pill-level" event at ~30fps

From src/components/FrequencyBars.tsx:
- Receives `level: number` prop (0.0-1.0)
- 24 bars, 30px max container height
- activeHeight = lv * BELL[i] * ((wave + 1) / 2) — linear relationship to level
- idleWave = 0.15 * BELL[i] * ... — constant floor always animating
- Final px = fraction * 30, minimum 0.06 fraction (about 2px)
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Amplify RMS normalization and apply non-linear curve in FrequencyBars</name>
  <files>src-tauri/src/pill.rs, src/components/FrequencyBars.tsx</files>
  <action>
Two changes to make bars bounce more prominently:

**Backend (pill.rs):**
- In compute_rms(), increase the multiplier from 10.0 to 15.0 — this pushes typical speech RMS (0.01-0.1 raw) into a higher normalized range (0.15-1.0 instead of 0.1-1.0). Still clamped to 1.0.
- Update the doc comment to reflect the new multiplier and typical speech range.

**Frontend (FrequencyBars.tsx):**
Apply these specific changes to the tick() function:

1. Apply a non-linear amplification curve to the level before using it for bar heights. Use a square root curve to boost quiet-to-mid levels while keeping loud levels near max:
   ```
   const amplified = Math.pow(lv, 0.55);
   ```
   This maps: 0.1 -> 0.28, 0.3 -> 0.51, 0.5 -> 0.68, 0.8 -> 0.88, 1.0 -> 1.0

2. Reduce the idle wave amplitude from 0.15 to 0.08 — this increases the contrast between idle and active states, making the jump when speech starts more noticeable.

3. Increase the height multiplier from 30 to 32 (the container is 30px, but bars can slightly overflow via border-radius for a more lively look).

4. Increase the wave amplitude contribution. Change the activeHeight formula from:
   ```
   const activeHeight = lv * BELL[i] * ((wave + 1) / 2);
   ```
   to:
   ```
   const activeHeight = amplified * BELL[i] * (0.3 + 0.7 * ((wave + 1) / 2));
   ```
   The `0.3 + 0.7 * wave` ensures bars never dip below 30% of their bell-curve height when active — they stay visibly tall during speech, with the sinusoidal wave adding bouncy variation on top rather than pulling bars all the way down to zero.

5. Lower the minimum fraction from 0.06 to 0.04 — makes silent state bars thinner, increasing perceived dynamic range.

6. Adjust opacity to `0.5 + fraction * 0.5` (from `0.4 + fraction * 0.6`) so bars stay more visible at mid heights.

Do NOT change: BAR_COUNT, BAR_FREQS, BAR_PHASES, BELL curve, bar colors, bar width, container height CSS, or the RAF loop structure.
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && npx vite build 2>&1 | tail -5</automated>
  </verify>
  <done>
    - compute_rms uses 15x multiplier instead of 10x
    - FrequencyBars applies sqrt-ish power curve (pow 0.55) to level
    - Idle wave reduced from 0.15 to 0.08
    - Active height formula uses 0.3 + 0.7 * wave to maintain minimum bar height during speech
    - Height multiplier increased to 32
    - Minimum fraction lowered to 0.04
    - Build succeeds without errors
  </done>
</task>

<task type="checkpoint:human-verify" gate="blocking">
  <name>Task 2: Verify waveform bar reactivity</name>
  <files>N/A</files>
  <action>Human visual verification of bar bounce prominence during recording.</action>
  <what-built>Amplified waveform bar reactivity — bars should bounce more visibly during speech, with greater contrast between silence and active recording</what-built>
  <how-to-verify>
    1. Run `npx tauri dev` (ensure vite build first if needed)
    2. Trigger recording via hotkey
    3. Speak at normal conversational volume
    4. Observe: bars should visibly bounce and reach near-full height during speech
    5. Pause speaking: bars should drop to a subtle idle wave (noticeably smaller than active)
    6. Speak loudly: center bars should nearly fill the 30px container
    7. Speak softly: bars should still show clear movement (not flat)
  </how-to-verify>
  <verify>Human visual confirmation</verify>
  <done>User confirms bars bounce prominently during speech</done>
  <resume-signal>Type "approved" if bars bounce prominently, or describe what needs adjustment (e.g., "still too subtle", "too aggressive", "idle too flat")</resume-signal>
</task>

</tasks>

<verification>
- `npx vite build` succeeds without errors
- Cargo builds without errors (pill.rs changes compile)
- Visual verification of bar reactivity during recording
</verification>

<success_criteria>
- Waveform bars visibly and energetically respond to normal speech
- Clear visual contrast between idle (silence) and active (speech) states
- Bars reach near-full container height during loud speech
- Quiet speech still produces visible bar movement
- No regression in animation smoothness or performance
</success_criteria>

<output>
After completion, create `.planning/quick/4-make-waveform-bars-in-pill-bounce-more-p/4-SUMMARY.md`
</output>
