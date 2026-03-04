import { useEffect, useState, useRef } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { invoke } from "@tauri-apps/api/core";
import { FrequencyBars } from "./components/FrequencyBars";
import { ProcessingDots } from "./components/ProcessingDots";

const appWindow = getCurrentWebviewWindow();

type PillDisplayState = "hidden" | "recording" | "processing" | "error";
type AnimState = "hidden" | "visible" | "exiting";
type DragState = "idle" | "ready" | "dragging";

export function Pill() {
  const [displayState, setDisplayState] = useState<PillDisplayState>("hidden");
  const [animState, setAnimState] = useState<AnimState>("hidden");
  const [level, setLevel] = useState(0);
  const [dragState, setDragState] = useState<DragState>("idle");

  // Timer for exit animation sequencing
  const exitTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Long-press timer
  const longPressTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Throttle ref for invoke calls during drag
  const lastInvokeRef = useRef<number>(0);

  // Deferred hide: if a hide event fires mid-drag, queue it until drag ends
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

    // pill-hide: exit animation then hide window (deferred if mid-drag)
    appWindow.listen("pill-hide", () => {
      setDragState((prev) => {
        if (prev === "ready" || prev === "dragging") {
          pendingHideRef.current = true;
          return prev;
        }
        clearAllTimers();
        setAnimState("exiting");
        exitTimerRef.current = setTimeout(() => {
          appWindow.hide();
          setAnimState("hidden");
          setDisplayState("hidden");
          exitTimerRef.current = null;
        }, 200);
        return "idle";
      });
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

    // pill-result: trigger exit animation on result (deferred if mid-drag)
    appWindow.listen<string>("pill-result", () => {
      setDragState((prev) => {
        if (prev === "ready" || prev === "dragging") {
          pendingHideRef.current = true;
          return prev;
        }
        setAnimState("exiting");
        exitTimerRef.current = setTimeout(() => {
          appWindow.hide();
          setAnimState("hidden");
          setDisplayState("hidden");
          exitTimerRef.current = null;
        }, 200);
        return "idle";
      });
    }).then((u) => unlisteners.push(u));

    return () => {
      unlisteners.forEach((u) => u());
      clearAllTimers();
    };
  }, []);

  // ---- Drag handlers ----

  function handlePointerDown(e: React.PointerEvent<HTMLDivElement>) {
    // Only primary button (left click / touch)
    if (e.button !== 0 && e.pointerType === "mouse") return;

    longPressTimerRef.current = setTimeout(() => {
      longPressTimerRef.current = null;
      setDragState("ready");
      e.currentTarget.setPointerCapture(e.pointerId);
    }, 600);
  }

  function handlePointerMove(e: React.PointerEvent<HTMLDivElement>) {
    setDragState((prev) => {
      if (prev === "idle") return prev;

      if (prev === "ready") {
        // First move after long-press — transition to dragging
        // (state update is async; we handle move in dragging branch too)
        const now = Date.now();
        if (now - lastInvokeRef.current >= 16) {
          lastInvokeRef.current = now;
          const x = Math.round(e.screenX) - 89; // 178/2
          const y = Math.round(e.screenY) - 23; // 46/2
          invoke("set_pill_position", { x, y }).catch(() => {});
        }
        return "dragging";
      }

      if (prev === "dragging") {
        const now = Date.now();
        if (now - lastInvokeRef.current >= 16) {
          lastInvokeRef.current = now;
          const x = Math.round(e.screenX) - 89;
          const y = Math.round(e.screenY) - 23;
          invoke("set_pill_position", { x, y }).catch(() => {});
        }
        return "dragging";
      }

      return prev;
    });
  }

  function handlePointerUp(e: React.PointerEvent<HTMLDivElement>) {
    // Clear long-press timer if it hasn't fired yet
    if (longPressTimerRef.current) {
      clearTimeout(longPressTimerRef.current);
      longPressTimerRef.current = null;
    }

    setDragState((prev) => {
      if (prev === "dragging" || prev === "ready") {
        try {
          e.currentTarget.releasePointerCapture(e.pointerId);
        } catch {
          // ignore if capture was not active
        }
        // Flush deferred hide if pill-hide/pill-result fired mid-drag
        if (pendingHideRef.current) {
          pendingHideRef.current = false;
          setAnimState("exiting");
          exitTimerRef.current = setTimeout(() => {
            appWindow.hide();
            setAnimState("hidden");
            setDisplayState("hidden");
            exitTimerRef.current = null;
          }, 200);
        }
        return "idle";
      }
      return prev;
    });
  }

  function handlePointerCancel(e: React.PointerEvent<HTMLDivElement>) {
    handlePointerUp(e);
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
        ${dragState === "ready" ? "pill-drag-ready" : ""}
        ${dragState === "dragging" ? "pill-dragging" : ""}
      `}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      onPointerCancel={handlePointerCancel}
      onDoubleClick={handleDoubleClick}
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
