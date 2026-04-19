import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import Overlay from "./windows/Overlay";
import Onboarding from "./windows/Onboarding";
import "./styles/global.css";

// Route by Tauri window label so both windows share the same bundle.
const label = (() => {
  try {
    return getCurrentWebviewWindow().label;
  } catch {
    return "overlay";
  }
})();

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement);
root.render(
  <React.StrictMode>
    {label === "onboarding" ? <Onboarding /> : <Overlay />}
  </React.StrictMode>,
);
