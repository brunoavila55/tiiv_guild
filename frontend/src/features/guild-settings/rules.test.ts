import { describe, expect, it } from "vitest";

import type { ObjectSchema } from "@/lib/api/types";

import { keyFromTitle, newRow, rowErrors, rowsFromSchema, schemaFromRows } from "./rules";

const schema: ObjectSchema = {
  type: "object",
  properties: {
    role: { type: "string", title: "Função", enum: ["Tank", "Healer"] },
    level: { type: "integer", title: "Nível", minimum: 1, maximum: 100 },
    discord: { type: "string", title: "Discord", maxLength: 32 },
    mic: { type: "boolean", title: "Microfone" },
  },
  required: ["level"],
  "x-order": ["role", "level", "discord", "mic"],
};

describe("keyFromTitle", () => {
  it("gera chave válida e única", () => {
    expect(keyFromTitle("Fuso horário", [])).toBe("fuso_horario");
    expect(keyFromTitle("Fuso horário", ["fuso_horario"])).toBe("fuso_horario_2");
    expect(keyFromTitle("2º main", [])).toBe("c_2_main");
    expect(keyFromTitle("!!!", [])).toBe("campo");
  });
});

describe("schema ↔ linhas", () => {
  it("faz ida e volta", () => {
    const rows = rowsFromSchema(schema);
    expect(rows.every((r) => r.saved)).toBe(true);
    expect(rows[0]?.options).toBe("Tank\nHealer");
    expect(schemaFromRows(rows)).toEqual(schema);
  });

  it("só manda os limites do tipo e converte opções numéricas", () => {
    const row = { ...newRow(), key: "tier", title: " Tier ", type: "integer" as const, options: "1\n 2 \n\n", minimum: "5" };
    expect(schemaFromRows([row]).properties.tier).toEqual({ type: "integer", title: "Tier", enum: [1, 2] });
    const text = { ...newRow(), key: "obs", title: "Obs", minimum: "1", maxLength: "10" };
    expect(schemaFromRows([text]).properties.obs).toEqual({ type: "string", title: "Obs", maxLength: 10 });
  });

  it("aponta erros antes de enviar", () => {
    const a = { ...newRow(), key: "a", title: "A" };
    const errors = rowErrors([a, { ...a, uid: "x" }, { ...newRow(), key: "b", title: "" }, { ...newRow(), key: "c", title: "C", minimum: "x" }]);
    expect(Object.keys(errors)).toHaveLength(4);
  });
});
