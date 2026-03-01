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

      for (let i = 0; i < BAR_COUNT; i++) {
        // Sinusoidal wave contribution scaled by level
        const wave = Math.sin(2 * Math.PI * BAR_FREQS[i] * t + BAR_PHASES[i]);
        // Active height: level-driven + bell envelope
        const activeHeight = lv * BELL[i] * ((wave + 1) / 2);
        // Idle wave: always running so bars are never fully flat
        const idleWave = 0.08 * BELL[i] * ((Math.sin(2 * Math.PI * 1.2 * t + BAR_PHASES[i]) + 1) / 2);
        // Composite height as fraction of container (0–1), then to px (container is 36px)
        const fraction = Math.max(0.05, activeHeight + idleWave);
        const heightPx = Math.round(fraction * 28);
        bars[i].style.height = `${heightPx}px`;
        // Opacity scaling: shorter bars are more transparent, taller bars are opaque
        bars[i].style.opacity = String(0.4 + fraction * 0.6);
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
      style={{ height: "28px" }}
    />
  );
}
