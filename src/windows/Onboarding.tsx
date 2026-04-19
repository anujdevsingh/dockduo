import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { AgentInfo } from "../store/appStore";

/**
 * First-run onboarding window.
 *
 * - Lists which coding-agent CLIs are already installed on the machine.
 * - Shows install hints for the ones that aren't.
 * - "Get started" persists `onboarded = true` and closes the window,
 *   handing control back to the overlay.
 */
export default function Onboarding() {
  const [agents, setAgents] = useState<AgentInfo[] | null>(null);

  useEffect(() => {
    invoke<AgentInfo[]>("list_agents")
      .then(setAgents)
      .catch(() => setAgents([]));
  }, []);

  const finish = async () => {
    try {
      await invoke("mark_onboarded");
    } catch (e) {
      console.warn("mark_onboarded failed", e);
    }
    try {
      await getCurrentWebviewWindow().close();
    } catch (e) {
      console.warn("close window failed", e);
    }
  };

  const have = (kind: "claude" | "codex" | "gemini") =>
    agents?.some((a) => a.kind === kind) ?? false;

  return (
    <div
      style={{
        minHeight: "100vh",
        padding: "32px 40px",
        background:
          "linear-gradient(180deg, #141923 0%, #1c2331 60%, #0f1218 100%)",
        color: "#E8ECF4",
        fontFamily: "-apple-system, Segoe UI, system-ui, sans-serif",
        display: "flex",
        flexDirection: "column",
        gap: 20,
      }}
    >
      <div>
        <h1 style={{ margin: 0, fontSize: 28, fontWeight: 700 }}>
          Welcome to DockDuo
        </h1>
        <p style={{ margin: "6px 0 0", color: "#A0AAC0", fontSize: 14 }}>
          Two tiny companions that live above your taskbar and open coding
          agents in a terminal with one click.
        </p>
      </div>

      <div>
        <h2 style={{ fontSize: 14, color: "#A0AAC0", fontWeight: 600, margin: "0 0 10px", letterSpacing: 0.4, textTransform: "uppercase" }}>
          Coding agents detected
        </h2>
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          <AgentRow
            name="Claude Code"
            installed={have("claude")}
            hint="npm install -g @anthropic-ai/claude-code"
            loading={agents === null}
          />
          <AgentRow
            name="OpenAI Codex CLI"
            installed={have("codex")}
            hint="npm install -g @openai/codex"
            loading={agents === null}
          />
          <AgentRow
            name="Google Gemini CLI"
            installed={have("gemini")}
            hint="npm install -g @google/generative-ai-cli"
            loading={agents === null}
          />
        </div>
        {agents !== null && agents.length === 0 && (
          <div
            style={{
              marginTop: 12,
              padding: "10px 14px",
              background: "rgba(217,83,79,0.12)",
              border: "1px solid rgba(217,83,79,0.45)",
              borderRadius: 10,
              fontSize: 13,
              color: "#FFC4C4",
            }}
          >
            No coding agent was found on your PATH. Install at least one —
            Claude Code is the default. DockDuo will still launch, but
            clicking a character will show a red error bubble until an agent
            is installed.
          </div>
        )}
      </div>

      <div>
        <h2 style={{ fontSize: 14, color: "#A0AAC0", fontWeight: 600, margin: "0 0 10px", letterSpacing: 0.4, textTransform: "uppercase" }}>
          Tips
        </h2>
        <ul style={{ margin: 0, paddingLeft: 20, color: "#CBD3E0", fontSize: 13, lineHeight: 1.6 }}>
          <li>Click a character to spawn its agent in a new terminal.</li>
          <li>If multiple agents are installed, a picker appears — choose one.</li>
          <li>Press <kbd style={kbdStyle}>Ctrl</kbd>+<kbd style={kbdStyle}>Shift</kbd>+<kbd style={kbdStyle}>L</kbd> to hide/show.</li>
          <li>Right-click the tray icon for themes and settings.</li>
        </ul>
      </div>

      <div style={{ marginTop: "auto", display: "flex", justifyContent: "flex-end" }}>
        <button
          onClick={finish}
          style={{
            padding: "10px 22px",
            fontSize: 14,
            fontWeight: 600,
            background: "#4F8EF7",
            color: "#FFFFFF",
            border: "none",
            borderRadius: 8,
            cursor: "pointer",
            boxShadow: "0 4px 14px rgba(79,142,247,0.35)",
          }}
        >
          Get started
        </button>
      </div>
    </div>
  );
}

const kbdStyle: React.CSSProperties = {
  display: "inline-block",
  padding: "1px 6px",
  background: "#2a3142",
  border: "1px solid #3d4558",
  borderRadius: 4,
  fontFamily: "ui-monospace, Menlo, Consolas, monospace",
  fontSize: 11,
  margin: "0 2px",
};

interface AgentRowProps {
  name: string;
  installed: boolean;
  hint: string;
  loading: boolean;
}

function AgentRow({ name, installed, hint, loading }: AgentRowProps) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 12,
        padding: "10px 14px",
        background: "rgba(255,255,255,0.04)",
        border: "1px solid rgba(255,255,255,0.08)",
        borderRadius: 10,
      }}
    >
      <span
        style={{
          width: 20,
          height: 20,
          borderRadius: 999,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          background: loading
            ? "#3a4050"
            : installed
              ? "#2F9E5A"
              : "#7E828C",
          color: "#FFFFFF",
          fontSize: 12,
          fontWeight: 700,
        }}
      >
        {loading ? "…" : installed ? "✓" : "—"}
      </span>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ fontSize: 14, fontWeight: 600 }}>{name}</div>
        {!installed && !loading && (
          <div
            style={{
              fontSize: 12,
              color: "#8F97A8",
              fontFamily: "ui-monospace, Menlo, Consolas, monospace",
              marginTop: 2,
            }}
          >
            {hint}
          </div>
        )}
      </div>
    </div>
  );
}
