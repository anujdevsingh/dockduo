import React from "react";
import ReactDOM from "react-dom/client";
import Overlay from "./windows/Overlay";
import "./styles/global.css";

const params = new URLSearchParams(window.location.search);
const windowType = params.get("window") ?? "overlay";

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement);
root.render(
  <React.StrictMode>
    {windowType === "overlay" ? <Overlay /> : <div>Unknown window: {windowType}</div>}
  </React.StrictMode>,
);
