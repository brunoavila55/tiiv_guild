import { describe, expect, it } from "vitest";

import { classifyHost, safeNext, slugError, slugify } from "./host";

const BASE = "tiiv.com.br";

describe("classifyHost", () => {
  it("classifica os hosts da plataforma e das guildas", () => {
    expect(classifyHost("tiiv.com.br", BASE)).toEqual({ kind: "root" });
    expect(classifyHost("www.tiiv.com.br", BASE)).toEqual({ kind: "root" });
    expect(classifyHost("app.tiiv.com.br", BASE)).toEqual({ kind: "app" });
    expect(classifyHost("ADMIN.tiiv.com.br", BASE)).toEqual({ kind: "admin" });
    expect(classifyHost("os-lobos.tiiv.com.br", BASE)).toEqual({ kind: "tenant", slug: "os-lobos" });
    expect(classifyHost("api.tiiv.com.br", BASE)).toEqual({ kind: "unknown" });
    expect(classifyHost("a.b.tiiv.com.br", BASE)).toEqual({ kind: "unknown" });
    expect(classifyHost("eviltiiv.com.br", BASE)).toEqual({ kind: "unknown" });
  });
});

describe("slug", () => {
  it("valida", () => {
    expect(slugError("os-lobos")).toBeNull();
    for (const bad of ["ab", "-abc", "abc-", "a--b", "Abc", "a_b", "app", "www"]) {
      expect(slugError(bad), bad).not.toBeNull();
    }
  });

  it("sugere a partir do nome", () => {
    expect(slugify("Ordem da Fênix!")).toBe("ordem-da-fenix");
    expect(slugify("  --Lobos  ")).toBe("lobos");
  });
});

describe("safeNext", () => {
  it("só aceita o domínio base", () => {
    expect(safeNext("http://acme.guildos.localhost:5173/painel")).toBe(
      "http://acme.guildos.localhost:5173/painel",
    );
    expect(safeNext("https://evil.com/")).toBeNull();
    expect(safeNext("javascript:alert(1)")).toBeNull();
    expect(safeNext(undefined)).toBeNull();
  });
});
