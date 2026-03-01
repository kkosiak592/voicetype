import { useEffect, useRef } from "react";

interface WaveformCanvasProps {
  level: number;  // 0.0-1.0 RMS
  bins: number[]; // 16 FFT magnitude bins, 0.0-1.0 each
}

const W = 150;
const H = 30;

// Bell-curve amplitude envelope — center bins peak tallest, edges taper
function bellCurve(i: number, count: number): number {
  const x = (i / (count - 1)) * 2 - 1; // -1 to 1
  return Math.exp(-x * x * 2.5);        // gaussian, peaks at center
}

const BELL_16 = Array.from({ length: 16 }, (_, i) => bellCurve(i, 16));

// Draw smooth bezier curve through 16 control points using quadratic midpoints
function drawSmoothedCurve(
  ctx: CanvasRenderingContext2D,
  points: { x: number; y: number }[],
) {
  if (points.length < 2) return;
  ctx.beginPath();
  ctx.moveTo(points[0].x, points[0].y);
  for (let i = 0; i < points.length - 1; i++) {
    const mx = (points[i].x + points[i + 1].x) / 2;
    const my = (points[i].y + points[i + 1].y) / 2;
    ctx.quadraticCurveTo(points[i].x, points[i].y, mx, my);
  }
  ctx.lineTo(points[points.length - 1].x, points[points.length - 1].y);
  ctx.stroke();
}

// Draw a single waveform curve (top half) + its mirror (bottom half)
function drawCurveWithMirror(
  ctx: CanvasRenderingContext2D,
  gradient: CanvasGradient,
  lv: number,
  bins: number[],
  phaseOffset: number,
  lineWidth: number,
  alpha: number,
  shadowBlur: number,
  shadowColor: string,
) {
  const amplified = Math.pow(lv, 0.55);
  const centerY = H / 2;

  // Build top control points
  const topPoints: { x: number; y: number }[] = bins.map((bin, i) => {
    const x = (i / 15) * W;
    const amplitude = amplified * BELL_16[i] * bin * (H / 2 * 0.85);
    const wave = Math.sin(phaseOffset + i * 0.4);
    const y = centerY - amplitude * ((wave + 1) / 2 + 0.3);
    return { x, y };
  });

  // Build bottom mirror points (reflected, slightly lower opacity via separate draw)
  const bottomPoints: { x: number; y: number }[] = bins.map((bin, i) => {
    const x = (i / 15) * W;
    const amplitude = amplified * BELL_16[i] * bin * (H / 2 * 0.85);
    const wave = Math.sin(phaseOffset + i * 0.4);
    const y = centerY + amplitude * ((wave + 1) / 2 + 0.3);
    return { x, y };
  });

  ctx.strokeStyle = gradient;
  ctx.lineWidth = lineWidth;
  ctx.globalAlpha = alpha;
  ctx.shadowBlur = shadowBlur;
  ctx.shadowColor = shadowColor;

  // Top curve
  drawSmoothedCurve(ctx, topPoints);

  // Bottom mirror at 60% opacity relative to top
  ctx.globalAlpha = alpha * 0.6;
  drawSmoothedCurve(ctx, bottomPoints);
}

function drawWaveform(
  ctx: CanvasRenderingContext2D,
  t: number,
  lv: number,
  bins: number[],
) {
  ctx.clearRect(0, 0, W, H);

  // Horizontal cyan-to-purple-to-cyan gradient
  const gradient = ctx.createLinearGradient(0, 0, W, 0);
  gradient.addColorStop(0, "#8b5cf6");
  gradient.addColorStop(0.5, "#06b6d4");
  gradient.addColorStop(1, "#8b5cf6");

  // Idle state: gentle breathing sine wave when near silence
  if (lv < 0.02) {
    const idleAmp = H / 2 * 0.08;
    const centerY = H / 2;

    ctx.strokeStyle = gradient;
    ctx.lineWidth = 1.5;
    ctx.globalAlpha = 0.35;
    ctx.shadowBlur = 6;
    ctx.shadowColor = "#06b6d4";
    ctx.globalCompositeOperation = "source-over";

    ctx.beginPath();
    ctx.moveTo(0, centerY + Math.sin(t * 1.2) * idleAmp);
    for (let x = 1; x <= W; x++) {
      const y = centerY + Math.sin(t * 1.2 + (x / W) * Math.PI * 2) * idleAmp;
      ctx.lineTo(x, y);
    }
    ctx.stroke();

    // Reset state
    ctx.globalCompositeOperation = "source-over";
    ctx.globalAlpha = 1;
    ctx.shadowBlur = 0;
    return;
  }

  // Phase offsets for 3 layered curves creating mesh/ribbon effect
  const phase1 = 0;
  const phase2 = t * 0.7;
  const phase3 = t * 1.3;

  // --- Core lines (thin, opaque, no blend) ---
  ctx.globalCompositeOperation = "source-over";

  drawCurveWithMirror(ctx, gradient, lv, bins, phase1, 1.5, 0.9, 0, "#06b6d4");
  drawCurveWithMirror(ctx, gradient, lv, bins, phase2, 1.5, 0.7, 0, "#06b6d4");
  drawCurveWithMirror(ctx, gradient, lv, bins, phase3, 1.5, 0.5, 0, "#06b6d4");

  // --- Bloom passes with additive blending ---
  ctx.globalCompositeOperation = "lighter";

  // Bloom pass 1: tight bright halo
  drawCurveWithMirror(ctx, gradient, lv, bins, phase1, 2, 0.6, 8, "#06b6d4");
  drawCurveWithMirror(ctx, gradient, lv, bins, phase2, 2, 0.4, 8, "#06b6d4");
  drawCurveWithMirror(ctx, gradient, lv, bins, phase3, 2, 0.3, 8, "#06b6d4");

  // Bloom pass 2: wider glow
  drawCurveWithMirror(ctx, gradient, lv, bins, phase1, 3, 0.3, 16, "#8b5cf6");
  drawCurveWithMirror(ctx, gradient, lv, bins, phase2, 3, 0.2, 16, "#8b5cf6");

  // Bloom pass 3: outermost thick halo
  drawCurveWithMirror(ctx, gradient, lv, bins, phase1, 5, 0.15, 24, "#8b5cf6");

  // Reset composite state (Pitfall 3 — must reset after additive blending)
  ctx.globalCompositeOperation = "source-over";
  ctx.globalAlpha = 1;
  ctx.shadowBlur = 0;
}

export function WaveformCanvas({ level, bins }: WaveformCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const rafRef = useRef<number>(0);
  const levelRef = useRef<number>(level);
  const binsRef = useRef<number[]>(bins);

  // Sync refs when props change — avoids restarting the RAF loop
  useEffect(() => {
    levelRef.current = level;
  }, [level]);

  useEffect(() => {
    binsRef.current = bins;
  }, [bins]);

  // Mount once — set up HiDPI canvas and start RAF loop
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    // HiDPI scaling (Pitfall 2 — avoids blurry canvas on high-DPI screens)
    const dpr = window.devicePixelRatio || 1;
    canvas.width = W * dpr;
    canvas.height = H * dpr;
    ctx.scale(dpr, dpr);

    const startTime = performance.now();

    function tick(now: number) {
      const t = (now - startTime) / 1000; // seconds elapsed
      drawWaveform(ctx!, t, levelRef.current, binsRef.current);
      rafRef.current = requestAnimationFrame(tick);
    }

    rafRef.current = requestAnimationFrame(tick);

    return () => {
      cancelAnimationFrame(rafRef.current);
    };
  }, []); // Empty deps — RAF loop never restarts; reads props via refs

  return (
    <canvas
      ref={canvasRef}
      style={{ width: `${W}px`, height: `${H}px` }}
    />
  );
}
