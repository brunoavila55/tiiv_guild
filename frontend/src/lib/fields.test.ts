import { describe, expect, it } from "vitest";

import type { ObjectSchema } from "./api/types";
import { formatFieldValue, fromFormValues, orderedFields, toFormValues } from "./fields";

const schema: ObjectSchema = {
  type: "object",
  properties: {
    discord: { type: "string", title: "Discord" },
    level: { type: "integer", title: "Nível", minimum: 1 },
    mic: { type: "boolean", title: "Microfone" },
    role: { type: "string", title: "Função", enum: ["Tank", "Healer"] },
  },
  required: ["level"],
  "x-order": ["role", "level", "ghost"],
};

describe("orderedFields", () => {
  it("segue x-order e põe o resto por chave", () => {
    expect(orderedFields(schema).map(([k]) => k)).toEqual(["role", "level", "discord", "mic"]);
  });
});

describe("valores do formulário", () => {
  it("converte de e para a API", () => {
    const form = toFormValues(schema, { level: 10, mic: true });
    expect(form).toEqual({ role: "", level: "10", discord: "", mic: true });
    expect(fromFormValues(schema, { ...form, discord: "  x ", level: "12,0" })).toEqual({
      role: null, level: 12, discord: "x", mic: true,
    });
    expect(fromFormValues(schema, { level: "abc" }).level).toBe("abc");
  });

  it("formata para exibição", () => {
    expect(formatFieldValue(schema.properties.mic, false)).toBe("Não");
    expect(formatFieldValue(schema.properties.level, null)).toBe("—");
    expect(formatFieldValue(schema.properties.level, 1500)).toBe("1.500");
  });
});
