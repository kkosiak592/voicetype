import { useEffect, useState, useCallback, useRef } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { PhysicalPosition } from "@tauri-apps/api/dpi";
import { load } from "@tauri-apps/plugin-store";
import { FrequencyBars } from "./components/FrequencyBars";

const appWindow = getCurrentWebviewWindow();

type PillDisplayState = "hidden" | "recording" | "processing" | "success" | "error";

export function Pill() {
  const [displayState, setDisplayState] = useState<PillDisplayState>("hidden");
  const [level, setLevel] = useState(0);
  const hideTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Position to bottom center on first load if no saved position
  useEffect(() => {
    async function initPosition() {
      try {
        const store = await load("settings.json");
        const saved = await store.get<{ x: number; y: number }>("pill-position");
        if (!saved) {
          const screenW = window.screen.width;
          const screenH = window.screen.height;
          const x = Math.round((screenW - 120) / 2);
          const y = screenH - 40 - 60;
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

    // pill-show: make visible (state set by pill-state event)
    appWindow.listen("pill-show", () => {
      if (hideTimerRef.current) {
        clearTimeout(hideTimerRef.current);
        hideTimerRef.current = null;
      }
      appWindow.show();
    }).then((u) => unlisteners.push(u));

    // pill-hide: fade out then hide window
    appWindow.listen("pill-hide", () => {
      setDisplayState("hidden");
      hideTimerRef.current = setTimeout(() => {
        appWindow.hide();
        hideTimerRef.current = null;
      }, 350); // slightly longer than 300ms fade transition
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

    // pill-result: success or error flash before hide
    appWindow.listen<string>("pill-result", (e) => {
      const result = e.payload as "success" | "error";
      setDisplayState(result);
      // Flash duration, then pill-hide from backend handles the rest
    }).then((u) => unlisteners.push(u));

    return () => {
      unlisteners.forEach((u) => u());
      if (hideTimerRef.current) clearTimeout(hideTimerRef.current);
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

  // Determine CSS classes based on state
  const isVisible = displayState !== "hidden";
  const isProcessing = displayState === "processing";
  const isSuccess = displayState === "success";
  const isError = displayState === "error";

  return (
    <div
      onMouseDown={handleMouseDown}
      onMouseUp={handleMouseUp}
      className={`
        w-[120px] h-[40px] rounded-full
        flex items-center justify-center
        cursor-grab active:cursor-grabbing
        select-none
        transition-opacity duration-300 ease-in-out
        ${isVisible ? "opacity-100" : "opacity-0 pointer-events-none"}
        ${isProcessing ? "pill-processing" : "bg-black/75"}
        ${isSuccess ? "pill-success" : ""}
        ${isError ? "pill-error" : ""}
      `}
    >
      {/* Recording state: frequency bars + red dot */}
      {displayState === "recording" && (
        <div className="flex items-center gap-2 px-3">
          {/* Red recording dot */}
          <div className="w-2 h-2 rounded-full bg-red-500 animate-pulse flex-shrink-0" />
          {/* Frequency bars */}
          <FrequencyBars level={level} />
        </div>
      )}

      {/* Processing state: text indicator (border animation from CSS) */}
      {displayState === "processing" && (
        <span className="text-white/60 text-[10px] font-medium tracking-wider uppercase">
          Processing
        </span>
      )}

      {/* Success state: text indicator (flash glow from CSS) */}
      {displayState === "success" && (
        <span className="text-green-400 text-sm font-bold">Done</span>
      )}

      {/* Error state: brief text (flash handles the visual) */}
      {displayState === "error" && (
        <span className="text-red-400 text-[10px] font-medium">No speech</span>
      )}
    </div>
  );
}
