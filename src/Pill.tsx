import { useEffect, useState, useRef } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { FrequencyBars } from "./components/FrequencyBars";
import { ProcessingDots } from "./components/ProcessingDots";

const appWindow = getCurrentWebviewWindow();

type PillDisplayState = "hidden" | "recording" | "processing" | "error";
type AnimState = "hidden" | "visible" | "exiting";

export function Pill() {
  const [displayState, setDisplayState] = useState<PillDisplayState>("hidden");
  const [animState, setAnimState] = useState<AnimState>("hidden");
  const [level, setLevel] = useState(0);

  // Timer for exit animation sequencing
  const exitTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  function clearAllTimers() {
    if (exitTimerRef.current) {
      clearTimeout(exitTimerRef.current);
      exitTimerRef.current = null;
    }
  }

  // Event listeners for all pill events from backend
  useEffect(() => {
    const unlisteners: Array<() => void> = [];

    // pill-show: make visible immediately (no entrance animation)
    appWindow.listen("pill-show", () => {
      clearAllTimers();
      appWindow.show();
      setAnimState("visible");
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

  return (
    <div
      className={`
        pill-glass
        w-[110px] h-[32px] rounded-full
        flex items-center justify-center
        select-none shadow-md overflow-hidden relative
        ${animState === "exiting" ? "pill-exiting" : ""}
        ${animState === "hidden" ? "opacity-0 pointer-events-none" : ""}
        ${displayState === "processing" ? "pill-processing" : ""}
        ${displayState === "recording" ? "pill-recording" : ""}
      `}
    >
      {/* Recording state: frequency bars only — no red dot */}
      {displayState === "recording" && (
        <div className="flex items-center justify-center px-2 w-full h-full pill-content-fade-in relative z-10">
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
