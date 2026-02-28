---
phase: quick
plan: 1
type: execute
wave: 1
depends_on: []
files_modified:
  - src/pill.css
  - src/components/FrequencyBars.tsx
  - src/components/ProcessingDots.tsx
  - src/components/CheckmarkIcon.tsx
  - src/Pill.tsx
  - src-tauri/tauri.conf.json
autonomous: false
requirements: [QUICK-01]

must_haves:
  truths:
    - "Pill window is 280x56 pixels with monochrome near-black glass appearance"
    - "Recording state shows 24 white mirrored bars growing symmetrically from center"
    - "Processing state shows shimmer sweep across pill with sequential-pulse white dots"
    - "Success state shows white checkmark that draws itself"
    - "Entrance animation scales from 0.85 with upward float, exit sinks down and fades"
    - "No indigo or purple colors appear anywhere in the UI"
  artifacts:
    - path: "src/pill.css"
      provides: "Monochrome glass styling, entrance float-up, exit sink-down, shimmer sweep, pulse dots keyframes"
      contains: "pill-glass"
    - path: "src/components/FrequencyBars.tsx"
      provides: "24 mirrored white bars in 36px container with opacity scaling"
      contains: "BAR_COUNT = 24"
    - path: "src/components/ProcessingDots.tsx"
      provides: "Sequential pulse animation white dots"
      contains: "pill-dot-pulse"
    - path: "src/components/CheckmarkIcon.tsx"
      provides: "White checkmark, larger size"
      contains: "stroke=\"white\""
    - path: "src/Pill.tsx"
      provides: "280x56 dimensions, updated timer durations for new animations"
      contains: "w-[280px] h-[56px]"
    - path: "src-tauri/tauri.conf.json"
      provides: "280x56 pill window dimensions"
      contains: "\"width\": 280"
  key_links:
    - from: "src/pill.css"
      to: "src/Pill.tsx"
      via: "CSS class names pill-glass, pill-entering, pill-exiting, pill-processing"
      pattern: "pill-entering|pill-exiting|pill-processing"
    - from: "src/pill.css"
      to: "src/components/ProcessingDots.tsx"
      via: "CSS class pill-dot-pulse"
      pattern: "pill-dot-pulse"
    - from: "src/Pill.tsx"
      to: "src-tauri/tauri.conf.json"
      via: "Pill dimensions must match window dimensions"
      pattern: "280.*56"
---

<objective>
Rework the pill overlay UI from indigo-purple AI aesthetic to monochrome luxury — white on near-black glass. Bigger pill (280x56), bigger mirrored waveform (24 bars, symmetric center growth), refined animations (float-up entrance, sink-down exit), shimmer sweep processing state with sequential-pulse dots, white checkmark.

Purpose: Current pill looks like generic AI slop. Monochrome luxury aesthetic is timeless and premium.
Output: Complete visual restyle of all pill components with no functional changes to event/state logic.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src/Pill.tsx
@src/pill.css
@src/components/FrequencyBars.tsx
@src/components/ProcessingDots.tsx
@src/components/CheckmarkIcon.tsx
@src-tauri/tauri.conf.json

Key constraints:
- No backdrop-filter blur (WebView2 bug #4945 — already omitted, keep it that way)
- Must stay pill-shaped (border-radius: 9999px)
- RAF for waveform bars (already in place, keep pattern)
- All animations via CSS keyframes (no JS-driven transitions except RAF bars)
- animState/displayState dual-state machine in Pill.tsx — do NOT change the state logic, only dimensions/classes/timers
</context>

<tasks>

<task type="auto">
  <name>Task 1: Monochrome CSS foundation and animation keyframes</name>
  <files>src/pill.css</files>
  <action>
Complete rewrite of pill.css. Keep the `@import "tailwindcss"` and transparent html/body/root rules. Replace everything else:

**pill-glass** — monochrome luxury:
- `background: rgba(10, 10, 10, 0.88)` (near-black, not blue-tinted)
- `border: 1px solid rgba(255, 255, 255, 0.08)` (subtle white, not indigo)
- `box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6), inset 0 1px 0 rgba(255, 255, 255, 0.06)` (no indigo glow)
- Keep `border-radius: 9999px`

**pill-enter keyframe** — scale 0.85→1 with upward float:
```
from { opacity: 0; transform: scale(0.85) translateY(8px); }
to   { opacity: 1; transform: scale(1) translateY(0); }
```
- Duration: 260ms, easing: `cubic-bezier(0.22, 1, 0.36, 1)` (smooth deceleration)
- `.pill-entering` class applies this

**pill-exit keyframe** — sink-down fade:
```
from { opacity: 1; transform: scale(1) translateY(0); }
to   { opacity: 0; transform: scale(0.92) translateY(6px); }
```
- Duration: 200ms, easing: `ease-in`
- `.pill-exiting` class applies this

**pill-processing shimmer sweep** — replace indigo glow pulse:
- Use a `::before` pseudo-element on `.pill-processing`
- The pseudo-element: `content: ''`, absolute positioned, full size, border-radius inherit
- Background: `linear-gradient(90deg, transparent 0%, rgba(255, 255, 255, 0.06) 50%, transparent 100%)`
- `background-size: 200% 100%`
- Animation: `shimmer-sweep 2s ease-in-out infinite` — moves gradient from left to right
- Keyframe: `from { background-position: 200% 0; } to { background-position: -200% 0; }`
- `.pill-processing` must have `position: relative; overflow: hidden` for the pseudo-element
- Remove the old `indigo-glow-pulse` keyframe and `.pill-processing` border/animation rules entirely

**pill-dot-pulse** — replace dot-bounce:
- Remove `dot-bounce` keyframe and `.pill-dot-bounce` class
- New `@keyframes dot-pulse`: `0%, 100% { opacity: 0.3; transform: scale(0.85); } 50% { opacity: 1; transform: scale(1); }` — gentle scale+opacity pulse, NOT vertical bounce
- Duration: 1000ms, ease-in-out, infinite
- `.pill-dot-pulse` class

**checkmark draw** — keep the draw animation but update stroke-dasharray/offset to match new larger size (path length ~30 for a 24x24 viewBox):
- `stroke-dasharray: 30; stroke-dashoffset: 30;`
- Same 280ms ease-out 40ms delay

**content-fade-in/out** — keep as-is (120ms)
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && npx vite build 2>&1 | tail -5</automated>
  </verify>
  <done>pill.css contains monochrome-only styling (zero indigo/purple references), float-up entrance, sink-down exit, shimmer sweep processing, pulse dots keyframe, larger checkmark draw values. Build succeeds.</done>
</task>

<task type="auto">
  <name>Task 2: Rewrite FrequencyBars, ProcessingDots, CheckmarkIcon to monochrome</name>
  <files>src/components/FrequencyBars.tsx, src/components/ProcessingDots.tsx, src/components/CheckmarkIcon.tsx</files>
  <action>
**FrequencyBars.tsx** — 24 mirrored white bars:
- Change `BAR_COUNT = 24`
- Regenerate `BAR_FREQS`: 24 values spread across 1.0–3.5 Hz range (mirror pattern: ascending first 12, descending last 12). Use: `[1.0, 1.2, 1.4, 1.6, 1.8, 2.0, 2.3, 2.6, 2.9, 3.2, 3.4, 3.5, 3.5, 3.4, 3.2, 2.9, 2.6, 2.3, 2.0, 1.8, 1.6, 1.4, 1.2, 1.0]`
- Regenerate `BAR_PHASES` from new count
- Regenerate `BELL` from new count
- Bar DOM construction changes:
  - `bar.style.width = "4px"` (was 3px)
  - `bar.style.background = "white"` (was indigo-purple gradient)
  - Add opacity scaling: after computing `fraction`, set `bar.style.opacity = String(0.4 + fraction * 0.6)` — shorter bars are more transparent, taller bars are opaque. This creates the "mirrored" visual depth effect.
  - Remove `bar.style.transition = "height 40ms ease-out"` — RAF updates at 60fps, transition creates lag
- Container height: `style={{ height: "36px" }}` (was 22px)
- Container className: `"flex items-center gap-[2px]"` (was `items-end` — change to `items-center` for symmetric mirrored growth from center)
- Height calculation: `const heightPx = Math.round(fraction * 36)` (was * 22)

**ProcessingDots.tsx** — sequential pulse, white:
- Change `bg-indigo-400` to `bg-white`
- Change `pill-dot-bounce` class to `pill-dot-pulse`
- Keep 3 dots, keep `gap-[5px]`
- Change dot size: `w-[6px] h-[6px]` (was 5px)
- Change animation delay to `${i * 200}ms` (was 120ms) — slower sequential pulse reads as more deliberate

**CheckmarkIcon.tsx** — white, larger:
- SVG dimensions: `width="24" height="24" viewBox="0 0 24 24"` (was 20x20)
- Polyline points scaled: `points="4,12 9,17 20,6"` (proportionally scaled from 20→24 viewBox)
- `stroke="white"` (was `#818cf8` indigo)
- `strokeWidth="2.5"` — keep the same, works at 24px
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && npx vite build 2>&1 | tail -5</automated>
  </verify>
  <done>FrequencyBars renders 24 white bars with items-center alignment in 36px container with opacity scaling. ProcessingDots uses pill-dot-pulse class with white dots. CheckmarkIcon is 24x24 white. No indigo/purple colors in any component. Build succeeds.</done>
</task>

<task type="auto">
  <name>Task 3: Update Pill.tsx dimensions and tauri.conf.json window size</name>
  <files>src/Pill.tsx, src-tauri/tauri.conf.json</files>
  <action>
**Pill.tsx** changes — dimensions and timer adjustments only, NO state machine logic changes:

1. Dimension classes: Change `w-[160px] h-[48px]` to `w-[280px] h-[56px]`

2. Initial position calculation: Update the position math in `initPosition()`:
   - `const x = Math.round((screenW - 280) / 2)` (was 160)
   - `const y = screenH - 56 - 60` (was 48)

3. Timer durations to match new CSS animations:
   - Entrance timer: `220` → `260` (matching new pill-enter duration)
   - Exit timer (all occurrences): `180` → `200` (matching new pill-exit duration)

4. Remove the indigo-glow processing comment on line 181 — update to "Processing state: shimmer sweep + pulse dots"

5. Everything else stays exactly the same — event listeners, animState/displayState logic, drag handling, clearAllTimers, all refs.

**tauri.conf.json** — pill window dimensions:
- Change pill window `"width": 160` → `"width": 280`
- Change pill window `"height": 48` → `"height": 56`
- All other config stays identical
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && npx vite build 2>&1 | tail -5</automated>
  </verify>
  <done>Pill.tsx uses 280x56 dimensions with timer durations matching CSS keyframes (260ms enter, 200ms exit). tauri.conf.json pill window is 280x56. Build succeeds with no errors.</done>
</task>

</tasks>

<verification>
1. `npx vite build` completes with no errors
2. No occurrences of `indigo`, `#6366f1`, `#818cf8`, `#c084fc`, `bg-indigo` in any modified file
3. pill.css contains no `backdrop-filter` property
4. FrequencyBars uses `items-center` (not `items-end`)
5. FrequencyBars BAR_COUNT is 24
6. Pill.tsx has `w-[280px] h-[56px]`
7. tauri.conf.json pill window is 280x56
</verification>

<success_criteria>
- Build passes cleanly
- All pill UI files use monochrome white-on-dark aesthetic — zero indigo/purple references
- Pill window is 280x56 in both React and Tauri config
- FrequencyBars: 24 bars, 4px wide, mirrored (items-center), white with opacity scaling, 36px container
- ProcessingDots: pulse animation (not bounce), white
- CheckmarkIcon: 24x24, white stroke
- Entrance: scale(0.85)+translateY(8px) → scale(1)+translateY(0)
- Exit: scale(1) → scale(0.92)+translateY(6px), fade out
- Processing: shimmer sweep pseudo-element, not glow pulse
</success_criteria>

<output>
After completion, create `.planning/quick/1-premium-pill-ui-rework-monochrome-luxury/1-SUMMARY.md`
</output>
