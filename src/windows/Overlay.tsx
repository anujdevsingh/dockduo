import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import Character from "../components/Character";
import { appStore, type AgentInfo } from "../store/appStore";
import type { AiStatus, CharacterId } from "../lib/sprites";
import { applyTheme, type ThemeId } from "../lib/themes";

type TaskbarEdge = "bottom" | "top" | "left" | "right";

interface TaskbarInfo {
  edge: TaskbarEdge;
  rect: [number, number, number, number];
  auto_hide: boolean;
  dpi_scale: number;
  monitor_rect: [number, number, number, number];
}

type PausedMap = Record<CharacterId, boolean>;

export default function Overlay() {
  const [tb, setTb] = useState<TaskbarInfo | null>(null);
  const [paused, setPaused] = useState<PausedMap>({ bruce: false, jazz: false });

  const overlayWindowLabel = useMemo(() => {
    try {
      return getCurrentWebviewWindow().label;
    } catch {
      return "overlay";
    }
  }, []);

  useEffect(() => {
    invoke<{ theme: ThemeId }>("get_config")
      .then((cfg) => applyTheme(cfg.theme))
      .catch((e) => console.warn("get_config failed", e));

    const unlistenTheme = listen<ThemeId>("theme-changed", (event) => {
      applyTheme(event.payload);
    });

    invoke<TaskbarInfo>("get_taskbar_info")
      .then(setTb)
      .catch((e) => console.error("get_taskbar_info failed", e));

    const unlistenTb = listen<TaskbarInfo>("taskbar-changed", (event) => {
      setTb(event.payload);
    });

    const unlistenAi = listen<{ character: CharacterId; status: AiStatus }>(
      "ai-status-changed",
      (event) => {
        appStore.setAiStatus(event.payload.character, event.payload.status);
      },
    );

    const unlistenPause = listen<{ character: CharacterId }>(
      "sprite-walk-paused",
      (event) => {
        setPaused((p) => ({ ...p, [event.payload.character]: true }));
      },
    );
    const unlistenResume = listen<{ character: CharacterId }>(
      "sprite-walk-resumed",
      (event) => {
        setPaused((p) => ({ ...p, [event.payload.character]: false }));
      },
    );

    return () => {
      unlistenTb.then((un) => un());
      unlistenAi.then((un) => un());
      unlistenTheme.then((un) => un());
      unlistenPause.then((un) => un());
      unlistenResume.then((un) => un());
    };
  }, []);

  // Overlay occupies the full overlay window; the characters walk along
  // its bottom edge. We compute their track in CSS pixels.
  const W = typeof window !== "undefined" ? window.innerWidth : 1920;
  const H = typeof window !== "undefined" ? window.innerHeight : 200;

  /** Sprite center-x in CSS pixels within the overlay window. */
  const spriteCenterX = (character: CharacterId): number => {
    const b = appStore.get().bounds[character];
    if (!b) return W / 2;
    return b.x + b.w / 2;
  };

  /** Open (or close) the chat bubble for this character with the chosen agent. */
  const openBubble = async (character: CharacterId, agent: AgentInfo) => {
    try {
      await invoke("toggle_bubble", {
        character,
        kind: agent.kind,
        spriteCenterX: spriteCenterX(character),
      });
    } catch (err) {
      const msg = typeof err === "string" ? err : String(err);
      console.warn(`toggle_bubble(${character}) failed:`, msg);
      appStore.setError(
        character,
        `Chat window failed to open: ${msg}`,
        12_000,
      );
    }
  };

  /**
   * Click handler for a character sprite.
   *
   *  - 0 agents installed → red error bubble asking the user to install one
   *  - 1 agent installed  → open the bubble immediately (or close if open)
   *  - 2+ agents          → show a picker bubble; user picks by clicking
   */
  const onCharacterClick = async (character: CharacterId) => {
    if (appStore.get().pickers[character]) {
      appStore.clearPicker(character);
      return;
    }

    // Source of truth is the Rust side: does the bubble window exist?
    // (Using local `paused` state can get stuck if toggle_bubble failed.)
    let isOpen = false;
    try {
      isOpen = await invoke<boolean>("bubble_is_open", { character });
    } catch (err) {
      console.warn(`bubble_is_open(${character}) failed:`, err);
    }
    if (isOpen) {
      try {
        await invoke("close_bubble", { character });
      } catch (err) {
        console.warn(`close_bubble(${character}) failed:`, err);
      }
      return;
    }

    let agents: AgentInfo[] = [];
    try {
      agents = await invoke<AgentInfo[]>("list_agents");
    } catch (err) {
      appStore.setError(
        character,
        `could not detect coding agents: ${String(err)}`,
        5000,
      );
      return;
    }

    if (agents.length === 0) {
      appStore.setError(
        character,
        "No coding agent found. Install Claude Code, Codex, or Gemini CLI.",
        8000,
      );
      return;
    }

    if (agents.length === 1) {
      await openBubble(character, agents[0]);
      return;
    }

    appStore.setPicker(character, agents, 10000);
  };

  /** Called by Character.tsx when the user clicks a picker pill. */
  const onPickAgent = (character: CharacterId, agent: AgentInfo) => {
    appStore.clearPicker(character);
    void openBubble(character, agent);
  };

  return (
    <div style={{ position: "fixed", inset: 0, pointerEvents: "none" }}>
      <Character
        character="bruce"
        windowLabel={overlayWindowLabel}
        initialFraction={0.2}
        trackLeft={0}
        trackRight={W}
        trackBottom={H}
        paused={paused.bruce}
        onClick={() => void onCharacterClick("bruce")}
        onPickAgent={(a) => onPickAgent("bruce", a)}
      />
      <Character
        character="jazz"
        windowLabel={overlayWindowLabel}
        initialFraction={0.75}
        trackLeft={0}
        trackRight={W}
        trackBottom={H}
        paused={paused.jazz}
        onClick={() => void onCharacterClick("jazz")}
        onPickAgent={(a) => onPickAgent("jazz", a)}
      />

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
