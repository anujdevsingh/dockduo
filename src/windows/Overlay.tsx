import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import Character from "../components/Character";
import { appStore } from "../store/appStore";

type TaskbarEdge = "bottom" | "top" | "left" | "right";

interface TaskbarInfo {
  edge: TaskbarEdge;
  rect: [number, number, number, number];
  auto_hide: boolean;
  dpi_scale: number;
  monitor_rect: [number, number, number, number];
}

export default function Overlay() {
  const [tb, setTb] = useState<TaskbarInfo | null>(null);

  useEffect(() => {
    invoke<TaskbarInfo>("get_taskbar_info")
      .then(setTb)
      .catch((e) => console.error("get_taskbar_info failed", e));

    const unlistenPromise = listen<TaskbarInfo>("taskbar-changed", (event) => {
      setTb(event.payload);
    });

    return () => {
      unlistenPromise.then((un) => un());
    };
  }, []);

  // Overlay occupies the full overlay window; the characters walk along
  // its bottom edge. We compute their track in CSS pixels.
  const W = typeof window !== "undefined" ? window.innerWidth : 1920;
  const H = typeof window !== "undefined" ? window.innerHeight : 200;

  const openTerminal = (character: "bruce" | "jazz") => {
    console.log(`open terminal for ${character} (wired in Phase 3)`);
    // Temporary feedback until Phase 3: flash busy -> completed
    appStore.setAiStatus(character, "busy");
    setTimeout(() => appStore.setAiStatus(character, "completed"), 1500);
    setTimeout(() => appStore.setAiStatus(character, "idle"), 5000);
  };

  return (
    <div style={{ position: "fixed", inset: 0, pointerEvents: "none" }}>
      <Character
        character="bruce"
        initialFraction={0.2}
        trackLeft={0}
        trackRight={W}
        trackBottom={H}
        onClick={() => openTerminal("bruce")}
      />
      <Character
        character="jazz"
        initialFraction={0.75}
        trackLeft={0}
        trackRight={W}
        trackBottom={H}
        onClick={() => openTerminal("jazz")}
      />

      {/* Debug badge removed — enable via VITE_DEBUG=1 if needed */}
      {import.meta.env.VITE_DEBUG === "1" && (
        <div
          style={{
            position: "absolute",
            top: 8,
            left: 12,
            fontSize: 11,
            color: "rgba(255,255,255,0.75)",
            background: "rgba(0,0,0,0.45)",
            padding: "3px 8px",
            borderRadius: 4,
            fontFamily: "monospace",
            pointerEvents: "none",
          }}
        >
          {tb
            ? `DockDuo · edge=${tb.edge} · dpi=${tb.dpi_scale.toFixed(2)} · auto_hide=${tb.auto_hide}`
            : "DockDuo · loading…"}
        </div>
      )}
    </div>
  );
}
