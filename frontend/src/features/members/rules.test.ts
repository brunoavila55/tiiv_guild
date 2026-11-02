import { describe, expect, it } from "vitest";

import { describeHistory, editMemberSchema, membersQuery, newMemberSchema } from "./rules";

const entry = (action: string, payload: Record<string, unknown> = {}) => ({
  id: "1", action, payload, actor_nickname: null, at: "2026-01-01T00:00:00Z",
});

describe("membersQuery", () => {
  it("só envia filtros preenchidos", () => {
    expect(membersQuery({ q: "  ", status: "", rank_id: "", page: 1, per_page: 25 })).toBe("?per_page=25");
    expect(membersQuery({ q: " 100%_ ", status: "left", rank_id: "r1", page: 3, per_page: 10 })).toBe(
      "?q=100%25_&status=left&rank_id=r1&page=3&per_page=10",
    );
  });
});

describe("describeHistory", () => {
  it("descreve alterações com rótulos em pt-BR", () => {
    expect(
      describeHistory(entry("member.updated", { notes: true, status: { from: "active", to: "banned" }, rank: { from: "Membro", to: "Oficial" } })),
    ).toBe("Ficha alterada — rank: Membro → Oficial; status: Ativo → Banido; notas editadas");
    expect(describeHistory(entry("member.added"))).toBe("Entrou na guilda");
    expect(describeHistory(entry("modulo.qualquer"))).toBe("modulo.qualquer");
  });
});

describe("schemas", () => {
  it("valida o cadastro", () => {
    const ok = newMemberSchema.safeParse({ nickname: " Ana ", rank_id: "r", status: "trial", email: "", notes: "" });
    expect(ok.success && ok.data.nickname).toBe("Ana");
    const bad = newMemberSchema.safeParse({ nickname: "a", rank_id: "", status: "banned", email: "x", notes: "" });
    expect(bad.success).toBe(false);
    expect(bad.error?.issues.map((i) => i.path[0]).sort()).toEqual(["email", "nickname", "rank_id", "status"]);
  });

  it("edição aceita qualquer status", () => {
    expect(editMemberSchema.safeParse({ nickname: "Ana", rank_id: "r", status: "left", notes: "" }).success).toBe(true);
  });
});
