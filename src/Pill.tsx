import { useEffect, useState, useCallback } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { PhysicalPosition } from "@tauri-apps/api/dpi";
import { load } from "@tauri-apps/plugin-store";

const appWindow = getCurrentWebviewWindow();

export function Pill() {
  const [visible, setVisible] = useState(false);

  // Position to bottom center on first load if no saved position
  useEffect(() => {
    async function initPosition() {
      try {
        const store = await load("settings.json");
        const saved = await store.get<{ x: number; y: number }>("pill-position");
        if (!saved) {
          // Default: bottom center, 60px from bottom edge
          const screenW = window.screen.width;
          const screenH = window.screen.height;
          const pillW = 120;
          const pillH = 40;
          const x = Math.round((screenW - pillW) / 2);
          const y = screenH - pillH - 60;
          await appWindow.setPosition(new PhysicalPosition(x, y));
        }
      } catch (e) {
        console.warn("Failed to init pill position:", e);
      }
    }
    initPosition();
  }, []);

  // Listen for show/hide events from backend
  useEffect(() => {
    const unlisteners: Array<() => void> = [];

    appWindow.listen("pill-show", () => {
      setVisible(true);
      appWindow.show();
    }).then((u) => unlisteners.push(u));

    appWindow.listen("pill-hide", () => {
      setVisible(false);
      // Delay actual window hide to allow fade-out transition
      setTimeout(() => appWindow.hide(), 300);
    }).then((u) => unlisteners.push(u));

    return () => unlisteners.forEach((u) => u());
  }, []);

  // Drag handling: startDragging on mousedown, save position on mouseup
  const handleMouseDown = useCallback(async (e: React.MouseEvent) => {
    e.preventDefault();
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
  }, []);

  return (
    <div
      onMouseDown={handleMouseDown}
      onMouseUp={handleMouseUp}
      className={`
        w-[120px] h-[40px] rounded-full
        bg-black/75 backdrop-blur-sm
        flex items-center justify-center
        cursor-grab active:cursor-grabbing
        select-none
        transition-opacity duration-300 ease-in-out
        ${visible ? "opacity-100" : "opacity-0 pointer-events-none"}
      `}
    >
      {/* Placeholder — Plan 04-02 adds FrequencyBars + state display here */}
      <span className="text-white/40 text-xs">VoiceType</span>
    </div>
  );
}
