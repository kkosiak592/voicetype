interface FrequencyBarsProps {
  level: number; // 0.0 - 1.0 normalized RMS from backend
  barCount?: number; // default 15
}

// Fixed multipliers simulate frequency band variation from a single RMS value.
// Center bars are taller (speech energy is mid-frequency), edges are shorter.
const BAND_MULTIPLIERS = [
  0.3, 0.5, 0.7, 0.85, 0.95, 1.0, 0.9, 1.0, 0.95, 0.85, 0.7, 0.6, 0.5, 0.4, 0.3,
];

export function FrequencyBars({ level, barCount = 15 }: FrequencyBarsProps) {
  return (
    <div className="flex items-end gap-[2px] h-6">
      {BAND_MULTIPLIERS.slice(0, barCount).map((mult, i) => {
        // Add slight random variation per render to avoid overly uniform look
        const jitter = 0.85 + Math.random() * 0.3; // 0.85 - 1.15
        const height = Math.max(2, level * mult * jitter * 100);
        return (
          <div
            key={i}
            className="w-[3px] rounded-full bg-indigo-400 transition-[height] duration-75 ease-out"
            style={{ height: `${height}%` }}
          />
        );
      })}
    </div>
  );
}
