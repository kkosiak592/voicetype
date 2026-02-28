import { useEffect, useRef } from "react";

interface FrequencyBarsProps {
  level: number; // 0.0 - 1.0 normalized RMS from backend
}

const BAR_COUNT = 12;

// Per-bar sinusoidal frequencies (Hz) — spread across 1.2–3.0 Hz range
const BAR_FREQS = [1.2, 1.5, 1.8, 2.1, 2.4, 2.7, 3.0, 2.7, 2.4, 2.1, 1.8, 1.5];

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

    // Build bar elements once
    const bars: HTMLDivElement[] = [];
    for (let i = 0; i < BAR_COUNT; i++) {
      const bar = document.createElement("div");
      bar.style.width = "3px";
      bar.style.borderRadius = "9999px";
      bar.style.background = "linear-gradient(to top, #6366f1, #c084fc)";
      bar.style.transition = "height 40ms ease-out";
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
        // Composite height as fraction of container (0–1), then to px (container is 22px)
        const fraction = Math.max(0.05, activeHeight + idleWave);
        const heightPx = Math.round(fraction * 22);
        bars[i].style.height = `${heightPx}px`;
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
      className="flex items-end gap-[2px]"
      style={{ height: "22px" }}
    />
  );
}
