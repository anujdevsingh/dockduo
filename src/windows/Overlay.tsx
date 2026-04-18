import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

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
      console.log("taskbar-changed", event.payload);
      setTb(event.payload);
    });

    return () => {
      unlistenPromise.then((un) => un());
    };
  }, []);

  // Phase 1 smoke test: render a colored stripe along the full overlay.
  // The overlay is transparent except for this stripe, proving the
  // window is positioned correctly above the taskbar.
  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        display: "flex",
        alignItems: "flex-end",
        justifyContent: "center",
        pointerEvents: "none",
      }}
    >
      <div
        style={{
          width: "100%",
          height: "12px",
          background: "linear-gradient(90deg,#E8845A,#58A6FF,#4CAF50)",
          opacity: 0.85,
        }}
      />
      <div
        style={{
          position: "absolute",
          top: 8,
          left: 12,
          fontSize: 12,
          color: "rgba(255,255,255,0.85)",
          background: "rgba(0,0,0,0.55)",
          padding: "4px 8px",
          borderRadius: 4,
          fontFamily: "monospace",
        }}
      >
        {tb
          ? `DockDuo · edge=${tb.edge} · dpi=${tb.dpi_scale.toFixed(2)} · auto_hide=${tb.auto_hide}`
          : "DockDuo · loading taskbar info…"}
      </div>
    </div>
  );
}
