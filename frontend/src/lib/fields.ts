// Campos customizados definidos pela guilda (espelho de `domain::fields`): ordem,
// conversão de/para formulário e exibição. O backend valida tudo.
import { t } from "@/app/i18n";

import type { FieldDef, FieldValue, ObjectSchema } from "./api/types";

export const EMPTY_SCHEMA: ObjectSchema = { type: "object", properties: {}, required: [], "x-order": [] };

/** Campos na ordem de exibição: a de `x-order`, depois os que faltarem, por chave. */
export function orderedFields(schema: ObjectSchema): [string, FieldDef][] {
  const order = schema["x-order"] ?? [];
  const listed = order.filter((k) => k in schema.properties);
  const rest = Object.keys(schema.properties).filter((k) => !order.includes(k)).sort();
  return [...listed, ...rest].map((k) => [k, schema.properties[k]!]);
}

/** Valores no formulário: checkbox para sim/não, texto para o resto. */
export type FieldFormValues = Record<string, string | boolean>;

export function toFormValues(schema: ObjectSchema, values: Record<string, FieldValue>): FieldFormValues {
  return Object.fromEntries(
    orderedFields(schema).map(([key, def]) => {
      const v = values[key];
      return [key, def.type === "boolean" ? v === true : v === undefined ? "" : String(v)];
    }),
  );
}

/** Valores para a API: vazio vira `null`; número inválido segue como texto para o backend apontar o erro. */
export function fromFormValues(schema: ObjectSchema, form: FieldFormValues): Record<string, FieldValue | null> {
  return Object.fromEntries(
    orderedFields(schema).map(([key, def]) => {
      const raw = form[key];
      if (def.type === "boolean") return [key, raw === true];
      const s = typeof raw === "string" ? raw.trim() : "";
      if (s === "") return [key, null];
      if (def.type === "string") return [key, s];
      const n = Number(s.replace(",", "."));
      return [key, Number.isFinite(n) ? n : s];
    }),
  );
}

export function formatFieldValue(def: FieldDef | undefined, value: unknown): string {
  if (value === null || value === undefined || value === "") return "—";
  if (typeof value === "boolean" || def?.type === "boolean") return value ? t.fields.yes : t.fields.no;
  if (typeof value === "number") return value.toLocaleString("pt-BR");
  return String(value);
}
