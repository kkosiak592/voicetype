import { useEffect, useRef } from "react";

interface FrequencyBarsProps {
  level: number; // 0.0 - 1.0 normalized RMS from backend
}

const BAR_COUNT = 24;

// Per-bar sinusoidal frequencies (Hz) — mirrored pattern: ascending first 12, descending last 12
const BAR_FREQS = [1.0, 1.2, 1.4, 1.6, 1.8, 2.0, 2.3, 2.6, 2.9, 3.2, 3.4, 3.5, 3.5, 3.4, 3.2, 2.9, 2.6, 2.3, 2.0, 1.8, 1.6, 1.4, 1.2, 1.0];

// Per-bar phase offsets — spread evenly across 2π so bars wave in sequence
const BAR_PHASES = BAR_FREQS.map((_, i) => (i / BAR_COUNT) * Math.PI * 2);

// Bell-curve amplitude envelope — center bars are taller
function bellCurve(i: number, count: number): number {
  const x = (i / (count - 1)) * 2 - 1; // -1 to 1
  return Math.exp(-x * x * 2.5); // gaussian, peaks at center
}

const BELL = Array.from({ length: BAR_COUNT }, (_, i) => bellCurve(i, BAR_COUNT));

// Evenly-spaced phase offset per bar so the idle wave travels at a constant speed left-to-right
const IDLE_PHASE_STEP = (Math.PI * 2) / BAR_COUNT;

export function FrequencyBars({ level }: FrequencyBarsProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const rafRef = useRef<number>(0);
  const startTimeRef = useRef<number | null>(null);
  // Keep a ref to level to avoid restarting the RAF loop on every level change
  const levelRef = useRef<number>(level);

  // Sync levelRef whenever the prop changes
  useEffect(() => {
    levelRef.current = level;
  }, [level]);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    // Build bar elements once — each bar gets a rainbow hue based on position
    const bars: HTMLDivElement[] = [];
    for (let i = 0; i < BAR_COUNT; i++) {
      const hue = Math.round((i / BAR_COUNT) * 300); // 0 (red) → 300 (magenta)
      const bar = document.createElement("div");
      bar.style.width = "3px";
      bar.style.borderRadius = "9999px";
      bar.style.background = `hsl(${hue}, 90%, 65%)`;
      bar.style.flexShrink = "0";
      container.appendChild(bar);
      bars.push(bar);
    }

    function tick(now: number) {
      if (startTimeRef.current === null) {
        startTimeRef.current = now;
      }
      const t = (now - startTimeRef.current) / 1000; // seconds
      const lv = levelRef.current;

      // Non-linear amplification: boosts quiet-to-mid levels, keeps loud levels near max
      // Maps: 0.1 -> 0.28, 0.3 -> 0.51, 0.5 -> 0.68, 0.8 -> 0.88, 1.0 -> 1.0
      const amplified = Math.pow(lv, 0.55);

      for (let i = 0; i < BAR_COUNT; i++) {
        // Sinusoidal wave contribution scaled by level
        const wave = Math.sin(2 * Math.PI * BAR_FREQS[i] * t + BAR_PHASES[i]);
        // Active height: 0.3 floor ensures bars stay visibly tall during speech,
        // 0.7 * wave adds bouncy sinusoidal variation on top
        const activeHeight = amplified * BELL[i] * (0.3 + 0.7 * ((wave + 1) / 2));
        // Idle traveling wave: uniform amplitude, linear phase sweep left-to-right
        // No BELL[i] factor — all bars participate equally; wave front moves rightward
        const idleWave = 0.12 * ((Math.sin(2 * Math.PI * 0.8 * t - i * IDLE_PHASE_STEP) + 1) / 2);
        // Composite height as fraction of container (0–1), then to px
        // Minimum lowered to 0.04 for thinner silent bars and greater dynamic range
        const fraction = Math.max(0.04, activeHeight + idleWave);
        // Container is 30px; use 32 multiplier so bars can slightly overflow for liveliness
        const heightPx = Math.round(fraction * 32);
        bars[i].style.height = `${heightPx}px`;
        // Opacity: bars stay more visible at mid heights
        bars[i].style.opacity = String(0.5 + fraction * 0.5);
      }

      rafRef.current = requestAnimationFrame(tick);
    }

    rafRef.current = requestAnimationFrame(tick);

    return () => {
      cancelAnimationFrame(rafRef.current);
      // Remove bar elements on unmount
      bars.forEach((bar) => container.removeChild(bar));
      startTimeRef.current = null;
    };
  }, []); // Empty deps — RAF loop never restarts; reads level via ref

  return (
    <div
      ref={containerRef}
      className="flex items-center gap-[1.5px]"
      style={{ height: "30px" }}
    />
  );
}
