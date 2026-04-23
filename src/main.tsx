import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import BrowserOnlyPlaceholder from "./windows/BrowserOnlyPlaceholder";
import Overlay from "./windows/Overlay";
import Onboarding from "./windows/Onboarding";
import AgentChat from "./windows/AgentChat";
import "./styles/global.css";

/** True only inside the WebView2 process (see `@tauri-apps/api` webview.js). */
function isTauriWebview(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

// Route by Tauri window label so all webviews share the same bundle.
// Important: if this page opens in a normal browser (localhost:1420), do NOT
// render Overlay — that duplicates Bruce/Jazz while tauri dev is running.
function resolveLabel(): string | null {
  if (!isTauriWebview()) {
    return null;
  }
  try {
    return getCurrentWebviewWindow().label;
  } catch {
    return null;
  }
}

const label = resolveLabel();

// Bubble windows are NOT transparent (unlike the overlay). If we leave body
// transparent, WebView2 paints its default white backdrop until React mounts.
// Force the surface colour immediately so the window opens as a dark bubble.
if (label && label.startsWith("bubble_")) {
  const bg = "#FAF9F5";
  document.documentElement.style.background = bg;
  document.body.style.background = bg;
  document.body.style.backgroundColor = bg;
}

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement);
root.render(
  <React.StrictMode>
    {label === null ? (
      <BrowserOnlyPlaceholder />
    ) : label === "onboarding" ? (
      <Onboarding />
    ) : label.startsWith("bubble_") ? (
      <AgentChat />
    ) : (
      <Overlay />
    )}
  </React.StrictMode>,
);
