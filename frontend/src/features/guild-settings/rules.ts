// Editor de campos de membro: linhas do formulário ↔ schema (espelho de `domain::fields`).
import type { FieldDef, FieldType, ObjectSchema } from "@/lib/api/types";
import { orderedFields } from "@/lib/fields";

export const FIELD_TYPES: readonly FieldType[] = ["string", "integer", "number", "boolean"];
export const MAX_FIELDS = 30;
export const KEY_RE = /^[a-z][a-z0-9_]{0,31}$/;

/** Um campo como está no editor (números ainda como texto). */
export interface FieldRow {
  uid: string;
  key: string;
  /** Já existe no servidor: a chave não muda (os valores salvos dependem dela). */
  saved: boolean;
  title: string;
  type: FieldType;
  required: boolean;
  options: string;
  minimum: string;
  maximum: string;
  minLength: string;
  maxLength: string;
}

let seq = 0;
const uid = () => `f${++seq}`;

const str = (n: number | undefined) => (n === undefined ? "" : String(n));

export function rowsFromSchema(schema: ObjectSchema): FieldRow[] {
  return orderedFields(schema).map(([key, def]) => ({
    uid: uid(),
    key,
    saved: true,
    title: def.title,
    type: def.type,
    required: schema.required.includes(key),
    options: (def.enum ?? []).join("\n"),
    minimum: str(def.minimum),
    maximum: str(def.maximum),
    minLength: str(def.minLength),
    maxLength: str(def.maxLength),
  }));
}

export function newRow(): FieldRow {
  return {
    uid: uid(), key: "", saved: false, title: "", type: "string", required: false,
    options: "", minimum: "", maximum: "", minLength: "", maxLength: "",
  };
}

/** Chave a partir do nome: sem acento, minúsculas e `_`, sem repetir as existentes. */
export function keyFromTitle(title: string, taken: readonly string[]): string {
  let base = title
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "")
    .slice(0, 28);
  if (!base) base = "campo";
  if (!/^[a-z]/.test(base)) base = `c_${base}`.slice(0, 28);
  let key = base;
  for (let i = 2; taken.includes(key); i++) key = `${base}_${i}`;
  return key;
}

const num = (s: string) => (s.trim() === "" ? undefined : Number(s.trim().replace(",", ".")));

/** Erro de cada linha (por `uid`) que dá para pegar antes de enviar. */
export function rowErrors(rows: readonly FieldRow[]): Record<string, string> {
  const errors: Record<string, string> = {};
  rows.forEach((r) => {
    const numbers = [r.minimum, r.maximum, r.minLength, r.maxLength].map(num);
    if (!r.title.trim()) errors[r.uid] = "informe o nome do campo";
    else if (!KEY_RE.test(r.key)) errors[r.uid] = "chave inválida: letras minúsculas, números e _, começando por letra";
    else if (rows.some((o) => o !== r && o.key === r.key)) errors[r.uid] = "chave repetida";
    else if (numbers.some((n) => n !== undefined && !Number.isFinite(n))) errors[r.uid] = "limites devem ser números";
  });
  return errors;
}

export function schemaFromRows(rows: readonly FieldRow[]): ObjectSchema {
  const properties: Record<string, FieldDef> = {};
  for (const r of rows) {
    const def: FieldDef = { type: r.type, title: r.title.trim() };
    const options = r.options.split("\n").map((o) => o.trim()).filter(Boolean);
    const numeric = r.type === "integer" || r.type === "number";
    if (options.length && (numeric || r.type === "string")) {
      def.enum = numeric ? options.map((o) => Number(o.replace(",", "."))) : options;
    } else if (numeric) {
      def.minimum = num(r.minimum);
      def.maximum = num(r.maximum);
    } else if (r.type === "string") {
      def.minLength = num(r.minLength);
      def.maxLength = num(r.maxLength);
    }
    properties[r.key] = JSON.parse(JSON.stringify(def)) as FieldDef; // tira os `undefined`
  }
  return {
    type: "object",
    properties,
    required: rows.filter((r) => r.required).map((r) => r.key),
    "x-order": rows.map((r) => r.key),
  };
}
