---
phase: quick-3
plan: 1
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/tauri.conf.json
  - src/Pill.tsx
  - src/pill.css
  - src/components/FrequencyBars.tsx
  - src/components/ProcessingDots.tsx
autonomous: false
requirements: []

must_haves:
  truths:
    - "Pill is visually smaller than the current 280x56 — compact, unobtrusive"
    - "Waveform bars display a vibrant color gradient instead of flat white"
    - "Processing dots animate with a lively bouncing/pulsing motion, not just opacity fade"
    - "Pill has an animated rainbow/gradient border during recording state"
  artifacts:
    - path: "src-tauri/tauri.conf.json"
      provides: "Reduced pill window dimensions"
      contains: "pill"
    - path: "src/Pill.tsx"
      provides: "Updated pill container with smaller dimensions and rainbow border class"
      min_lines: 100
    - path: "src/pill.css"
      provides: "Rainbow border keyframes, updated dot animations"
      contains: "@keyframes rainbow"
    - path: "src/components/FrequencyBars.tsx"
      provides: "Vibrant gradient-colored bars"
      min_lines: 50
    - path: "src/components/ProcessingDots.tsx"
      provides: "Enhanced animated thinking dots"
      min_lines: 10
  key_links:
    - from: "src/Pill.tsx"
      to: "src/pill.css"
      via: "CSS class pill-rainbow-border applied during recording"
      pattern: "pill-rainbow-border"
    - from: "src/Pill.tsx"
      to: "src-tauri/tauri.conf.json"
      via: "Window dimensions match CSS dimensions"
      pattern: "w-\\[.*\\] h-\\[.*\\]"
---

<objective>
Overhaul the pill overlay UI: shrink it to a more compact size, add vibrant gradient colors to the waveform bars, enhance the processing dots with a livelier bounce animation, and add an animated rainbow gradient border that activates during recording.

Purpose: The current monochrome pill is functional but visually flat. This overhaul makes it feel premium and alive — the kind of small UI element that makes you smile when it appears.
Output: Updated pill with smaller footprint, colorful waveform, bouncy dots, rainbow border.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@src/Pill.tsx
@src/pill.css
@src/components/FrequencyBars.tsx
@src/components/ProcessingDots.tsx
@src/components/CheckmarkIcon.tsx
@src-tauri/tauri.conf.json

<interfaces>
<!-- Current pill window config in tauri.conf.json -->
pill window: width=280, height=56, transparent=true, decorations=false, shadow=false

<!-- Pill.tsx container sizing (must match tauri.conf.json) -->
w-[280px] h-[56px] in Pill.tsx className
Position calc: (screenW - 280) / 2 in initPosition()

<!-- FrequencyBars: 24 bars, white color, 36px max height, 4px wide, 2px gap -->
BAR_COUNT = 24, bar.style.background = "white", container height: 36px

<!-- ProcessingDots: 3 white dots, 6px, pill-dot-pulse animation -->
3 dots, w-[6px] h-[6px], bg-white, staggered 200ms delay

<!-- CSS classes consumed by Pill.tsx -->
pill-glass, pill-entering, pill-exiting, pill-processing, pill-content-fade-in
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Shrink pill dimensions and add rainbow border CSS</name>
  <files>src-tauri/tauri.conf.json, src/Pill.tsx, src/pill.css</files>
  <action>
**1. Reduce pill window to 200x44px:**

In `src-tauri/tauri.conf.json`, change the pill window:
- `"width": 200` (was 280)
- `"height": 44` (was 56)

In `src/Pill.tsx`:
- Change container class from `w-[280px] h-[56px]` to `w-[200px] h-[44px]`
- Update `initPosition()` position calc: `(screenW - 200) / 2` and `screenH - 44 - 60`
- Add `pill-rainbow-border` class to the pill container when `displayState === "recording"` (alongside existing classes)

**2. Add animated rainbow border in pill.css:**

Add a `@keyframes rainbow-rotate` that rotates a conic-gradient through 360 degrees:
```css
@keyframes rainbow-rotate {
  from { --border-angle: 0deg; }
  to { --border-angle: 360deg; }
}
```

Since CSS custom properties in @keyframes requires @property registration (supported in WebView2), add:
```css
@property --border-angle {
  syntax: "<angle>";
  initial-value: 0deg;
  inherits: false;
}
```

Add `.pill-rainbow-border` class that:
- Uses a `border: none` (remove the subtle white border from pill-glass)
- Uses a pseudo-element (::after) positioned with `inset: -2px`, `border-radius: inherit`, `z-index: -1`
- The pseudo-element background is a `conic-gradient(from var(--border-angle), #ff0000, #ff8800, #ffff00, #00ff00, #0088ff, #8800ff, #ff0000)`
- Apply `animation: rainbow-rotate 3s linear infinite`
- The pseudo-element is masked so only a 2px ring shows (use padding-box/border-box background-clip trick or simply size it 2px larger than the pill on all sides)

The pill-glass background will cover the center, so the conic gradient only peeks out as a 2px border.

**Important:** The pill-glass class currently has `border: 1px solid rgba(255, 255, 255, 0.08)`. When pill-rainbow-border is active, override this to `border: none` so the rainbow replaces it.

**3. Update pill-processing shimmer for smaller width:**
No changes needed — the shimmer uses percentage-based sizing.

**4. Reduce checkmark and dot spacing proportionally:**
In Pill.tsx, change the recording state padding from `px-4` to `px-3` for tighter fit in the smaller pill.
  </action>
  <verify>
    <automated>cd "C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text" && npx vite build 2>&1 | tail -5</automated>
  </verify>
  <done>
- Pill window is 200x44 in tauri.conf.json
- Pill container is w-[200px] h-[44px] in Pill.tsx
- Position calculation uses 200 and 44
- pill-rainbow-border class defined in pill.css with conic-gradient animation
- pill-rainbow-border applied in Pill.tsx when displayState === "recording"
- Vite build succeeds
  </done>
</task>

<task type="auto">
  <name>Task 2: Vibrant gradient waveform bars and bouncy processing dots</name>
  <files>src/components/FrequencyBars.tsx, src/components/ProcessingDots.tsx, src/pill.css</files>
  <action>
**1. Vibrant gradient FrequencyBars:**

In `FrequencyBars.tsx`, replace the flat white bar color with a per-bar HSL gradient. Each bar gets a hue based on its position:
- Map bar index 0..23 to hue range 0..300 (red through magenta, skipping the return-to-red)
- Set `bar.style.background` to `hsl(${hue}, 90%, 65%)` instead of `"white"`
- This creates a rainbow spectrum across the bars: red on left edges, through orange/yellow/green/cyan/blue/purple toward center then mirrored back

Alternative: use a single HSL mapping where center bars are warm (orange/yellow) and edge bars are cool (blue/purple) — a "heat" gradient. Implementation:
- `const hue = 30 + (BELL[i]) * 30` — center bars are ~60 (yellow), edge bars are ~30 (orange)
- Actually, for maximum vibrancy and visual pop, use the full rainbow: `const hue = (i / BAR_COUNT) * 300`

Go with the full rainbow mapping for maximum visual impact. The bars will look like an audio spectrum visualizer.

Also reduce bar width from `4px` to `3px` and gap from `2px` to `1.5px` (via flex gap) to fit 24 bars in the smaller 200px pill (24 bars * 3px + 23 gaps * 1.5px = 106.5px, fits comfortably in ~170px usable area after padding).

Reduce max height reference from `36` to `28` (proportional to smaller pill). Update the container `style={{ height: "28px" }}`.

Keep the opacity scaling (`0.4 + fraction * 0.6`) — it adds depth.

**2. Bouncy ProcessingDots:**

Replace the current 3-dot opacity pulse with a more dynamic bouncing animation. In `src/pill.css`:

Replace the `@keyframes dot-pulse` with a bounce animation:
```css
@keyframes dot-bounce {
  0%, 100% {
    transform: translateY(0) scale(1);
    opacity: 0.5;
  }
  50% {
    transform: translateY(-6px) scale(1.15);
    opacity: 1;
  }
}

.pill-dot-bounce {
  animation: dot-bounce 800ms ease-in-out infinite;
}
```

In `ProcessingDots.tsx`:
- Change class from `pill-dot-pulse` to `pill-dot-bounce`
- Change dot size from `w-[6px] h-[6px]` to `w-[5px] h-[5px]` (proportional to smaller pill)
- Change dot color from `bg-white` to a subtle gradient — use inline style: `background: linear-gradient(135deg, #a78bfa, #818cf8)` (soft purple gradient, complements rainbow theme)
- Keep the staggered `animationDelay` pattern but reduce to `150ms` spacing for snappier feel
- Reduce gap from `gap-[5px]` to `gap-[4px]`

Keep the old `.pill-dot-pulse` class in CSS (harmless) or remove it — either is fine.
  </action>
  <verify>
    <automated>cd "C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text" && npx vite build 2>&1 | tail -5</automated>
  </verify>
  <done>
- FrequencyBars renders 24 bars with HSL hue gradient (rainbow spectrum)
- Bar width reduced to 3px, container height to 28px for compact pill
- ProcessingDots bounce vertically with scale, not just fade
- Dots have purple gradient color instead of plain white
- Dot animation is snappier (800ms, 150ms stagger)
- Vite build succeeds
  </done>
</task>

<task type="checkpoint:human-verify" gate="blocking">
  <name>Task 3: Visual verification of pill UI overhaul</name>
  <files>n/a</files>
  <action>
User visually verifies all four changes: smaller pill, rainbow border, vibrant waveform, bouncy dots.
  </action>
  <verify>
    <automated>echo "Manual verification required"</automated>
  </verify>
  <done>User confirms pill looks correct across all states: recording (rainbow border + colorful bars), processing (bouncy purple dots), success (checkmark), error (silent dismiss).</done>
  <what-built>Complete pill UI overhaul: smaller 200x44 pill, vibrant rainbow waveform bars, bouncy purple processing dots, animated rainbow conic-gradient border during recording</what-built>
  <how-to-verify>
    1. Run `npx tauri dev` (ensure dist/ is built first with `npx vite build`)
    2. Trigger recording with your hotkey
    3. Verify the pill is noticeably smaller than before (200x44 vs 280x56)
    4. Verify the waveform bars show a rainbow color spectrum (red through purple across the bars)
    5. Verify the pill has a rotating rainbow border during recording
    6. Stop recording to trigger processing state
    7. Verify the processing dots bounce up and down (not just fade in/out) and appear purple-ish
    8. Let transcription complete, verify success checkmark still works
    9. Verify the pill still drags and repositions correctly
  </how-to-verify>
  <resume-signal>Type "approved" or describe what needs adjustment (colors, sizes, animation timing, etc.)</resume-signal>
</task>

</tasks>

<verification>
- `npx vite build` succeeds without errors
- tauri.conf.json pill window is 200x44
- Pill.tsx container matches 200x44 with rainbow-border class on recording
- FrequencyBars uses HSL hue per bar instead of white
- ProcessingDots uses bounce animation instead of pulse
- pill.css contains @keyframes rainbow-rotate and @keyframes dot-bounce
- No regressions in pill enter/exit/success/error animations
</verification>

<success_criteria>
- Pill is visually compact (200x44) — noticeably smaller than 280x56
- Waveform bars display a rainbow spectrum of colors
- Recording state shows an animated rainbow border rotating around the pill
- Processing dots bounce vertically with purple gradient color
- All existing pill states (recording, processing, success, error) still function correctly
- Pill drag-to-reposition still works
</success_criteria>

<output>
After completion, create `.planning/quick/3-pill-ui-overhaul-smaller-size-vibrant-wa/3-SUMMARY.md`
</output>
