import { useEffect, useState, useCallback, useRef } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { PhysicalPosition } from "@tauri-apps/api/dpi";
import { load } from "@tauri-apps/plugin-store";
import { FrequencyBars } from "./components/FrequencyBars";
import { ProcessingDots } from "./components/ProcessingDots";

const appWindow = getCurrentWebviewWindow();

type PillDisplayState = "hidden" | "recording" | "processing" | "error";
type AnimState = "hidden" | "entering" | "visible" | "exiting";

export function Pill() {
  const [displayState, setDisplayState] = useState<PillDisplayState>("hidden");
  const [animState, setAnimState] = useState<AnimState>("hidden");
  const [level, setLevel] = useState(0);

  // Timers for animation sequencing
  const enterTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const exitTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Clears all pending animation timers
  function clearAllTimers() {
    if (enterTimerRef.current) {
      clearTimeout(enterTimerRef.current);
      enterTimerRef.current = null;
    }
    if (exitTimerRef.current) {
      clearTimeout(exitTimerRef.current);
      exitTimerRef.current = null;
    }
  }

  // Position to bottom center on first load if no saved position
  useEffect(() => {
    async function initPosition() {
      try {
        const store = await load("settings.json");
        const saved = await store.get<{ x: number; y: number }>("pill-position");
        if (!saved) {
          const screenW = window.screen.width;
          const screenH = window.screen.height;
          const x = Math.round((screenW - 178) / 2);
          const y = screenH - 46 - 60;
          await appWindow.setPosition(new PhysicalPosition(x, y));
        }
      } catch (e) {
        console.warn("Failed to init pill position:", e);
      }
    }
    initPosition();
  }, []);

  // Event listeners for all pill events from backend
  useEffect(() => {
    const unlisteners: Array<() => void> = [];

    // pill-show: make visible with entrance animation
    appWindow.listen("pill-show", () => {
      clearAllTimers();
      appWindow.show();
      setAnimState("entering");
      // After entrance animation completes (260ms), transition to visible
      enterTimerRef.current = setTimeout(() => {
        setAnimState("visible");
        enterTimerRef.current = null;
      }, 260);
    }).then((u) => unlisteners.push(u));

    // pill-hide: exit animation then hide window
    appWindow.listen("pill-hide", () => {
      clearAllTimers();
      setAnimState("exiting");
      // After exit animation completes (200ms), hide window and reset state
      exitTimerRef.current = setTimeout(() => {
        appWindow.hide();
        setAnimState("hidden");
        setDisplayState("hidden");
        exitTimerRef.current = null;
      }, 200);
    }).then((u) => unlisteners.push(u));

    // pill-state: update display state
    // Backend sends "recording" | "processing" | "idle" — "idle" is handled by pill-hide
    appWindow.listen<string>("pill-state", (e) => {
      if (e.payload === "idle") {
        // idle is handled by pill-hide — don't set directly to avoid race with success/error flash
        return;
      }
      setDisplayState(e.payload as PillDisplayState);
    }).then((u) => unlisteners.push(u));

    // pill-level: update RMS level for frequency bars
    appWindow.listen<number>("pill-level", (e) => {
      setLevel(e.payload);
    }).then((u) => unlisteners.push(u));

    // pill-result: trigger exit animation on result
    appWindow.listen<string>("pill-result", () => {
      setAnimState("exiting");
      exitTimerRef.current = setTimeout(() => {
        appWindow.hide();
        setAnimState("hidden");
        setDisplayState("hidden");
        exitTimerRef.current = null;
      }, 200);
    }).then((u) => unlisteners.push(u));

    return () => {
      unlisteners.forEach((u) => u());
      clearAllTimers();
    };
  }, []);

  // Drag handling: temporarily enable focusable for drag, restore after
  const handleMouseDown = useCallback(async (e: React.MouseEvent) => {
    e.preventDefault();
    await appWindow.setFocusable(true);
    await appWindow.startDragging();
  }, []);

  const handleMouseUp = useCallback(async () => {
    try {
      const pos = await appWindow.outerPosition();
      const store = await load("settings.json");
      await store.set("pill-position", { x: pos.x, y: pos.y });
      await store.save();
    } catch (e) {
      console.warn("Failed to save pill position:", e);
    }
    await appWindow.setFocusable(false);
  }, []);

  return (
    <div
      onMouseDown={handleMouseDown}
      onMouseUp={handleMouseUp}
      className={`
        pill-glass
        w-[170px] h-[38px] rounded-full
        flex items-center justify-center
        cursor-grab active:cursor-grabbing
        select-none
        ${animState === "entering" ? "pill-entering" : ""}
        ${animState === "exiting" ? "pill-exiting" : ""}
        ${animState === "hidden" ? "opacity-0 pointer-events-none" : ""}
        ${displayState === "processing" ? "pill-processing" : ""}
        ${displayState === "recording" ? "pill-rainbow-border" : ""}
      `}
    >
      {/* Recording state: frequency bars only — no red dot */}
      {displayState === "recording" && (
        <div className="flex items-center justify-center px-3 pill-content-fade-in">
          <FrequencyBars level={level} />
        </div>
      )}

      {/* Processing state: shimmer sweep + pulse dots */}
      {displayState === "processing" && (
        <div className="pill-content-fade-in">
          <ProcessingDots />
        </div>
      )}

      {/* Error state: render nothing — pill is already exiting via scale-down */}
    </div>
  );
}
