import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
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

export default function Overlay() {
  const [tb, setTb] = useState<TaskbarInfo | null>(null);

  useEffect(() => {
    // Load persisted theme on mount so the very first frame uses the
    // user's chosen palette rather than the default Midnight.
    invoke<{ theme: ThemeId }>("get_config")
      .then((cfg) => applyTheme(cfg.theme))
      .catch((e) => console.warn("get_config failed", e));

    // Tray menu emits this when the user picks a different theme.
    const unlistenTheme = listen<ThemeId>("theme-changed", (event) => {
      applyTheme(event.payload);
    });

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
      unlistenTheme.then((un) => un());
    };
  }, []);

  // Overlay occupies the full overlay window; the characters walk along
  // its bottom edge. We compute their track in CSS pixels.
  const W = typeof window !== "undefined" ? window.innerWidth : 1920;
  const H = typeof window !== "undefined" ? window.innerHeight : 200;

  /**
   * Spawn a specific agent for this character. Handles the benign
   * "already running" dedupe silently; everything else shows a red
   * error bubble.
   */
  const spawnWithAgent = async (character: CharacterId, agent: AgentInfo) => {
    try {
      await invoke<number>("spawn_agent", {
        character,
        agentPath: agent.path,
      });
    } catch (err) {
      const msg = typeof err === "string" ? err : String(err);
      if (msg.toLowerCase().includes("already running")) {
        console.info(`spawn_agent(${character}) dedupe:`, msg);
        return;
      }
      console.warn(`spawn_agent(${character}) failed:`, msg);
      appStore.setError(character, msg, 5000);
    }
  };

  /**
   * Click handler for a character sprite.
   *
   *  - 0 agents installed → red error bubble asking the user to install one
   *  - 1 agent installed  → spawn it immediately
   *  - 2+ agents          → show a picker bubble; user picks by clicking
   */
  const onCharacterClick = async (character: CharacterId) => {
    // If a picker is already showing for this character, a click on the
    // sprite itself dismisses it. (Clicks on pills are handled inside
    // Character.tsx via `onPickAgent`.)
    if (appStore.get().pickers[character]) {
      appStore.clearPicker(character);
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
      // Auto-spawn — priority order is already baked into list_agents.
      await spawnWithAgent(character, agents[0]);
      return;
    }

    // Multiple agents present — let the user choose.
    appStore.setPicker(character, agents, 10000);
  };

  /** Called by Character.tsx when the user clicks a picker pill. */
  const onPickAgent = (character: CharacterId, agent: AgentInfo) => {
    appStore.clearPicker(character);
    void spawnWithAgent(character, agent);
  };

  return (
    <div style={{ position: "fixed", inset: 0, pointerEvents: "none" }}>
      <Character
        character="bruce"
        initialFraction={0.2}
        trackLeft={0}
        trackRight={W}
        trackBottom={H}
        onClick={() => void onCharacterClick("bruce")}
        onPickAgent={(a) => onPickAgent("bruce", a)}
      />
      <Character
        character="jazz"
        initialFraction={0.75}
        trackLeft={0}
        trackRight={W}
        trackBottom={H}
        onClick={() => void onCharacterClick("jazz")}
        onPickAgent={(a) => onPickAgent("jazz", a)}
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
