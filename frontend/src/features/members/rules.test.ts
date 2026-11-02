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
    const ok = newMemberSchema.safeParse({ nickname: " Ana ", rank_id: "r", status: "trial", email: "", notes: "", fields: { mic: true } });
    expect(ok.success && ok.data.nickname).toBe("Ana");
    const bad = newMemberSchema.safeParse({ nickname: "a", rank_id: "", status: "banned", email: "x", notes: "", fields: {} });
    expect(bad.success).toBe(false);
    expect(bad.error?.issues.map((i) => i.path[0]).sort()).toEqual(["email", "nickname", "rank_id", "status"]);
  });

  it("edição aceita qualquer status", () => {
    expect(editMemberSchema.safeParse({ nickname: "Ana", rank_id: "r", status: "left", notes: "", fields: {} }).success).toBe(true);
  });
});

describe("histórico de campos customizados", () => {
  it("usa o nome do schema e formata o valor", () => {
    const schema = {
      type: "object" as const,
      properties: { mic: { type: "boolean" as const, title: "Microfone" } },
      required: [],
      "x-order": ["mic"],
    };
    const h = entry("member.updated", { "fields.mic": { from: null, to: true }, "fields.old": { from: "a", to: null }, rank: { from: "A", to: "B" } });
    expect(describeHistory(h, schema)).toBe("Ficha alterada — rank: A → B; Microfone: — → Sim; old: a → —");
  });
});
