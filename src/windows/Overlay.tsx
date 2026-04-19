import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import Character from "../components/Character";
import { appStore } from "../store/appStore";
import type { AiStatus, CharacterId } from "../lib/sprites";

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

    const unlistenTb = listen<TaskbarInfo>("taskbar-changed", (event) => {
      setTb(event.payload);
    });

    // Real AI status events from the Rust side (claude process lifecycle).
    const unlistenAi = listen<{ character: CharacterId; status: AiStatus }>(
      "ai-status-changed",
      (event) => {
        appStore.setAiStatus(event.payload.character, event.payload.status);
      },
    );

    return () => {
      unlistenTb.then((un) => un());
      unlistenAi.then((un) => un());
    };
  }, []);

  // Overlay occupies the full overlay window; the characters walk along
  // its bottom edge. We compute their track in CSS pixels.
  const W = typeof window !== "undefined" ? window.innerWidth : 1920;
  const H = typeof window !== "undefined" ? window.innerHeight : 200;

  const openTerminal = async (character: CharacterId) => {
    try {
      await invoke<number>("spawn_claude", { character });
      // Rust emits the "busy" status itself on successful spawn.
    } catch (err) {
      // If claude is already running for this character, that's fine.
      // Any other error gets logged so we can see it during dev.
      console.warn(`spawn_claude(${character}) failed:`, err);
    }
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
