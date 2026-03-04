import { useEffect, useRef } from "react";

interface FrequencyBarsProps {
  level: number; // 0.0 - 1.0 normalized RMS from backend
}

const BAR_COUNT = 12;

// Per-bar sinusoidal frequencies (Hz) — mirrored pattern: ascending first 6, descending last 6
const BAR_FREQS = [1.0, 1.4, 2.0, 2.8, 3.4, 3.5, 3.5, 3.4, 2.8, 2.0, 1.4, 1.0];

// Per-bar phase offsets — spread evenly across 2π so bars wave in sequence
const BAR_PHASES = BAR_FREQS.map((_, i) => (i / BAR_COUNT) * Math.PI * 2);

// Bell-curve amplitude envelope — center bars are taller
function bellCurve(i: number, count: number): number {
  const x = (i / (count - 1)) * 2 - 1; // -1 to 1
  return Math.exp(-x * x * 2.5); // gaussian, peaks at center
}

const BELL = Array.from({ length: BAR_COUNT }, (_, i) => bellCurve(i, BAR_COUNT));

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

    // Build bar elements once — each bar gets a subtle aesthetic gradient hue
    const bars: HTMLDivElement[] = [];
    for (let i = 0; i < BAR_COUNT; i++) {
      const hue = Math.round(200 + (i / BAR_COUNT) * 60); // 200 (blue) → 260 (purple)
      const bar = document.createElement("div");
      bar.style.width = "4px";
      bar.style.borderRadius = "9999px";
      bar.style.background = `hsl(${hue}, 85%, 65%)`; /* slightly softer colors */
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

      // Non-linear amplification: boosts quiet-to-mid levels significantly for more sensitivity
      const amplified = Math.min(1.0, Math.pow(lv * 2.0, 0.45));

      for (let i = 0; i < BAR_COUNT; i++) {
        // Sinusoidal wave contribution scaled by level
        const wave = Math.sin(2 * Math.PI * BAR_FREQS[i] * t + BAR_PHASES[i]);
        // Active height: lower floor (0.1) and higher variation (0.9) for more "bounce"
        const activeHeight = amplified * BELL[i] * (0.1 + 0.9 * ((wave + 1) / 2));
        // Idle wave
        const idleWave = 0.08 * BELL[i] * ((Math.sin(2 * Math.PI * 1.2 * t + BAR_PHASES[i]) + 1) / 2);

        const fraction = Math.max(0.1, activeHeight + idleWave);
        // Container is 20px
        const heightPx = Math.round(fraction * 20);
        bars[i].style.height = `${heightPx}px`;
        bars[i].style.opacity = String(0.6 + fraction * 0.4);
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
  }, []);

  return (
    <div
      ref={containerRef}
      className="flex items-center justify-center gap-[2px] w-full"
      style={{ height: "20px" }}
    />
  );
}
