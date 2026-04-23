import type { ReactNode } from "react";
import { Fragment, createElement } from "react";

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

/** Minimal markdown → HTML for tests and simple previews (headings, bold, inline code). */
export function renderChatMarkdownToHtml(markdown: string): string {
  const lines = markdown.split("\n");
  const parts: string[] = [];
  let inFence = false;
  let fenceBuf: string[] = [];

  const flushFence = () => {
    if (!fenceBuf.length) return;
    parts.push(`<pre><code>${escapeHtml(fenceBuf.join("\n"))}</code></pre>`);
    fenceBuf = [];
  };

  for (const line of lines) {
    if (line.trim().startsWith("```")) {
      if (inFence) {
        inFence = false;
        flushFence();
      } else {
        inFence = true;
      }
      continue;
    }
    if (inFence) {
      fenceBuf.push(line);
      continue;
    }
    const t = line.trim();
    if (t.startsWith("### ")) {
      parts.push(`<h3>${escapeHtml(t.slice(4))}</h3>`);
    } else if (t.startsWith("## ")) {
      parts.push(`<h2>${escapeHtml(t.slice(3))}</h2>`);
    } else if (t.startsWith("# ")) {
      parts.push(`<h1>${escapeHtml(t.slice(2))}</h1>`);
    } else if (t.startsWith("- ")) {
      parts.push(`<li>${inlineToHtml(t.slice(2))}</li>`);
    } else if (t.length === 0) {
      parts.push("<br/>");
    } else {
      parts.push(`<p>${inlineToHtml(line)}</p>`);
    }
  }
  if (inFence) flushFence();
  return parts.join("");
}

function inlineToHtml(s: string): string {
  let out = escapeHtml(s);
  out = out.replace(/`([^`]+)`/g, "<code>$1</code>");
  out = out.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
  return out;
}

/** React transcript rendering (subset aligned with `renderChatMarkdownToHtml`). */
export function renderChatMarkdown(text: string): ReactNode {
  const lines = text.split("\n");
  const nodes: ReactNode[] = [];
  let inFence = false;
  let fenceBuf: string[] = [];
  let key = 0;

  const pushFence = () => {
    if (!fenceBuf.length) return;
    nodes.push(
      createElement(
        "pre",
        { key: key++, style: { margin: "0.5em 0", overflow: "auto" } },
        createElement("code", null, fenceBuf.join("\n")),
      ),
    );
    fenceBuf = [];
  };

  for (const line of lines) {
    if (line.trim().startsWith("```")) {
      if (inFence) {
        inFence = false;
        pushFence();
      } else {
        inFence = true;
      }
      continue;
    }
    if (inFence) {
      fenceBuf.push(line);
      continue;
    }
    const t = line.trim();
    if (t.startsWith("### ")) {
      nodes.push(createElement("h3", { key: key++ }, t.slice(4)));
    } else if (t.startsWith("## ")) {
      nodes.push(createElement("h2", { key: key++ }, t.slice(3)));
    } else if (t.startsWith("# ")) {
      nodes.push(createElement("h1", { key: key++ }, t.slice(2)));
    } else if (t.startsWith("- ")) {
      nodes.push(
        createElement("div", { key: key++, style: { marginLeft: "1em" } }, "• ", inlineToReact(t.slice(2))),
      );
    } else if (t.length === 0) {
      nodes.push(createElement("br", { key: key++ }));
    } else {
      nodes.push(createElement("p", { key: key++, style: { margin: "0.35em 0" } }, inlineToReact(line)));
    }
  }
  if (inFence) pushFence();
  return createElement(Fragment, null, nodes);
}

function inlineToReact(s: string): ReactNode {
  const parts: ReactNode[] = [];
  let i = 0;
  let k = 0;
  while (i < s.length) {
    const tick = s.indexOf("`", i);
    const bold = s.indexOf("**", i);
    let next = -1;
    let mode: "code" | "bold" | null = null;
    if (tick >= 0 && (bold < 0 || tick <= bold)) {
      next = tick;
      mode = "code";
    } else if (bold >= 0) {
      next = bold;
      mode = "bold";
    }
    if (mode === null) {
      parts.push(s.slice(i));
      break;
    }
    if (next > i) parts.push(s.slice(i, next));
    if (mode === "code") {
      const end = s.indexOf("`", next + 1);
      if (end < 0) {
        parts.push(s.slice(next));
        break;
      }
      parts.push(createElement("code", { key: k++ }, s.slice(next + 1, end)));
      i = end + 1;
    } else {
      const end = s.indexOf("**", next + 2);
      if (end < 0) {
        parts.push(s.slice(next));
        break;
      }
      parts.push(createElement("strong", { key: k++ }, s.slice(next + 2, end)));
      i = end + 2;
    }
  }
  return createElement(Fragment, null, parts);
}
