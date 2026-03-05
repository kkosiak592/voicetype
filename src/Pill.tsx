import { useEffect, useState, useRef, useCallback } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { invoke } from "@tauri-apps/api/core";
import { FrequencyBars } from "./components/FrequencyBars";
import { ProcessingDots } from "./components/ProcessingDots";

const appWindow = getCurrentWebviewWindow();

type PillDisplayState = "hidden" | "recording" | "processing" | "moving" | "error";
type AnimState = "hidden" | "visible" | "exiting";

export function Pill() {
  const [displayState, setDisplayState] = useState<PillDisplayState>("hidden");
  const [animState, setAnimState] = useState<AnimState>("hidden");
  const [level, setLevel] = useState(0);

  // Timer for exit animation sequencing
  const exitTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Long-press timer
  const longPressTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Throttle ref for invoke calls during move
  const lastInvokeRef = useRef<number>(0);

  // Deferred hide: if a hide event fires while in move mode, queue it
  const pendingHideRef = useRef(false);

  function clearAllTimers() {
    if (exitTimerRef.current) {
      clearTimeout(exitTimerRef.current);
      exitTimerRef.current = null;
    }
    if (longPressTimerRef.current) {
      clearTimeout(longPressTimerRef.current);
      longPressTimerRef.current = null;
    }
  }

  function doHide() {
    clearAllTimers();
    setAnimState("exiting");
    exitTimerRef.current = setTimeout(() => {
      appWindow.hide();
      setAnimState("hidden");
      setDisplayState("hidden");
      exitTimerRef.current = null;
    }, 200);
  }

  // Global mouse move handler for move mode — follows cursor everywhere
  const handleGlobalMouseMove = useCallback((e: MouseEvent) => {
    const now = Date.now();
    if (now - lastInvokeRef.current >= 16) {
      lastInvokeRef.current = now;
      const x = Math.round(e.screenX) - 89; // 178/2
      const y = Math.round(e.screenY) - 23; // 46/2
      invoke("set_pill_position", { x, y }).catch(() => {});
    }
  }, []);

  // Attach/detach global mouse listener when entering/leaving move mode
  useEffect(() => {
    if (displayState === "moving") {
      document.addEventListener("mousemove", handleGlobalMouseMove);
      return () => {
        document.removeEventListener("mousemove", handleGlobalMouseMove);
      };
    }
  }, [displayState, handleGlobalMouseMove]);

  // Event listeners for all pill events from backend
  useEffect(() => {
    const unlisteners: Array<() => void> = [];

    // pill-show: make visible immediately (no entrance animation)
    appWindow.listen("pill-show", () => {
      clearAllTimers();
      pendingHideRef.current = false;
      appWindow.show();
      setAnimState("visible");
    }).then((u) => unlisteners.push(u));

    // pill-hide: exit animation then hide window (deferred if in move mode)
    appWindow.listen("pill-hide", () => {
      setDisplayState((prev) => {
        if (prev === "moving") {
          pendingHideRef.current = true;
          return prev;
        }
        doHide();
        return prev;
      });
    }).then((u) => unlisteners.push(u));

    // pill-state: update display state
    // Backend sends "recording" | "processing" | "idle" — "idle" is handled by pill-hide
    appWindow.listen<string>("pill-state", (e) => {
      if (e.payload === "idle") return;
      setDisplayState((prev) => {
        // Don't overwrite move mode with recording/processing
        if (prev === "moving") return prev;
        return e.payload as PillDisplayState;
      });
    }).then((u) => unlisteners.push(u));

    // pill-level: update RMS level for frequency bars
    appWindow.listen<number>("pill-level", (e) => {
      setLevel(e.payload);
    }).then((u) => unlisteners.push(u));

    // pill-result: trigger exit animation on result (deferred if in move mode)
    appWindow.listen<string>("pill-result", () => {
      setDisplayState((prev) => {
        if (prev === "moving") {
          pendingHideRef.current = true;
          return prev;
        }
        doHide();
        return prev;
      });
    }).then((u) => unlisteners.push(u));

    // pill-exit-move: backend tells us to exit move mode (hotkey was pressed)
    appWindow.listen("pill-exit-move", () => {
      invoke("exit_pill_move_mode").catch(() => {});
      setDisplayState("hidden");
      // Flush pending hide or just hide
      pendingHideRef.current = false;
      doHide();
    }).then((u) => unlisteners.push(u));

    return () => {
      unlisteners.forEach((u) => u());
      clearAllTimers();
    };
  }, []);

  // ---- Long-press to enter move mode ----

  function handlePointerDown(e: React.PointerEvent<HTMLDivElement>) {
    if (e.button !== 0 && e.pointerType === "mouse") return;

    longPressTimerRef.current = setTimeout(() => {
      longPressTimerRef.current = null;
      // Enter move mode
      invoke("enter_pill_move_mode").catch(() => {});
      setDisplayState("moving");
    }, 600);
  }

  function handlePointerUp() {
    // Clear long-press timer if it hasn't fired yet
    if (longPressTimerRef.current) {
      clearTimeout(longPressTimerRef.current);
      longPressTimerRef.current = null;
    }
    // In move mode, pointer up does NOT exit — only hotkey exits
  }

  function handlePointerCancel() {
    handlePointerUp();
  }

  function handleDoubleClick() {
    invoke("reset_pill_position").catch(() => {});
  }

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
        ${displayState === "moving" ? "pill-moving" : ""}
      `}
      onPointerDown={handlePointerDown}
      onPointerUp={handlePointerUp}
      onPointerCancel={handlePointerCancel}
      onDoubleClick={handleDoubleClick}
    >
      {/* Recording state: frequency bars only */}
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

      {/* Move mode: move indicator */}
      {displayState === "moving" && (
        <div className="pill-content-fade-in flex items-center justify-center gap-1.5 relative z-10">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="rgba(255,255,255,0.8)" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
            <polyline points="5 9 2 12 5 15" />
            <polyline points="9 5 12 2 15 5" />
            <polyline points="15 19 12 22 9 19" />
            <polyline points="19 9 22 12 19 15" />
            <line x1="2" y1="12" x2="22" y2="12" />
            <line x1="12" y1="2" x2="12" y2="22" />
          </svg>
        </div>
      )}

      {/* Error state: render nothing — pill is already exiting via scale-down */}
    </div>
  );
}
