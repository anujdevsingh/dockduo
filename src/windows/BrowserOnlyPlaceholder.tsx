/**
 * Shown when this Vite bundle loads in a normal browser (e.g. user opened
 * http://localhost:1420) while `pnpm tauri dev` is also running. Without this,
 * the catch-all branch would render <Overlay /> and duplicate Bruce/Jazz on
 * screen on top of the real Tauri overlay.
 */
export default function BrowserOnlyPlaceholder() {
  return (
    <div
      style={{
        boxSizing: "border-box",
        minHeight: "100vh",
        padding: 32,
        margin: 0,
        background: "#121418",
        color: "#e8e8ec",
        fontFamily: "system-ui, sans-serif",
        fontSize: 14,
        lineHeight: 1.5,
      }}
    >
      <h1 style={{ fontSize: 18, fontWeight: 600, margin: "0 0 12px" }}>
        DockDuo dev server (browser)
      </h1>
      <p style={{ margin: "0 0 12px", maxWidth: 520 }}>
        This URL is only for the Tauri windows created by{" "}
        <code style={{ color: "#98c379" }}>pnpm tauri dev</code>. If you also
        opened this address in Chrome, Edge, or Cursor’s Simple Browser, close
        that tab — it loads the same overlay UI and you will see Bruce/Jazz
        doubled or tripled on your screen.
      </p>
      <p style={{ margin: 0, maxWidth: 520 }}>
        Use the small transparent DockDuo window above your taskbar; do not keep
        a browser tab on <code style={{ color: "#61afef" }}>localhost:1420</code>{" "}
        open while testing.
      </p>
    </div>
  );
}
