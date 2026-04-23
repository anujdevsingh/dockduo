import { describe, expect, it } from "vitest";
import { renderChatMarkdownToHtml } from "./chatMarkdown";

describe("chatMarkdown", () => {
  it("renders heading", () => {
    expect(renderChatMarkdownToHtml("# Hi")).toContain("Hi");
    expect(renderChatMarkdownToHtml("# Hi")).toContain("<h1>");
  });

  it("renders bold", () => {
    expect(renderChatMarkdownToHtml("**x**")).toContain("<strong>x</strong>");
  });

  it("renders fenced block", () => {
    const md = "```\na\n```";
    expect(renderChatMarkdownToHtml(md)).toContain("<pre>");
    expect(renderChatMarkdownToHtml(md)).toContain("a");
  });
});
