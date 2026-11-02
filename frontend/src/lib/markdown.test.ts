import { describe, expect, it } from "vitest";

import { parseInline, parseMarkdown, safeHref } from "./markdown";

describe("parseMarkdown", () => {
  it("separa títulos, parágrafos, listas, citações, código e linhas", () => {
    const blocks = parseMarkdown("# Sobre\nSomos amigos.\nJogamos à noite.\n\n- um\n- dois\n\n1. primeiro\n\n> lema\n\n---\n\n```\n<b>x</b>\n```");
    expect(blocks.map((b) => b.type)).toEqual(["heading", "paragraph", "list", "list", "quote", "hr", "code"]);
    expect(blocks[1]).toEqual({
      type: "paragraph",
      children: [{ type: "text", text: "Somos amigos." }, { type: "break" }, { type: "text", text: "Jogamos à noite." }],
    });
    expect(blocks[2]).toMatchObject({ ordered: false, items: [[{ text: "um" }], [{ text: "dois" }]] });
    expect(blocks[3]).toMatchObject({ ordered: true });
    expect(blocks[6]).toEqual({ type: "code", text: "<b>x</b>" });
  });

  it("limita o nível dos títulos", () => {
    expect(parseMarkdown("###### fundo")[0]).toMatchObject({ type: "heading", level: 3 });
  });

  it("trata HTML como texto", () => {
    expect(parseMarkdown("<img src=x onerror=alert(1)>")).toEqual([
      { type: "paragraph", children: [{ type: "text", text: "<img src=x onerror=alert(1)>" }] },
    ]);
  });
});

describe("parseInline", () => {
  it("reconhece negrito, itálico, código e links", () => {
    expect(parseInline("**forte** e *leve* e `x` e [site](https://acme.gg)")).toEqual([
      { type: "strong", children: [{ type: "text", text: "forte" }] },
      { type: "text", text: " e " },
      { type: "em", children: [{ type: "text", text: "leve" }] },
      { type: "text", text: " e " },
      { type: "code", text: "x" },
      { type: "text", text: " e " },
      { type: "link", href: "https://acme.gg", children: [{ type: "text", text: "site" }] },
    ]);
  });

  it("não quebra palavras com sublinhado", () => {
    expect(parseInline("snake_case_name")).toEqual([{ type: "text", text: "snake_case_name" }]);
  });

  it("descarta links perigosos mantendo o texto", () => {
    expect(parseInline("[clique](javascript:alert(1))")).toEqual([{ type: "text", text: "clique" }, { type: "text", text: ")" }]);
    expect(parseInline("[x](data:text/html,oi)")).toEqual([{ type: "text", text: "x" }]);
  });
});

describe("safeHref", () => {
  it("aceita só http(s) absoluto", () => {
    expect(safeHref("https://discord.gg/acme")).toBe(true);
    expect(safeHref("http://acme.gg")).toBe(true);
    expect(safeHref("javascript:alert(1)")).toBe(false);
    expect(safeHref("/painel")).toBe(false);
    expect(safeHref("mailto:a@b.c")).toBe(false);
  });
});
