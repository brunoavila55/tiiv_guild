import type { UseFormRegisterReturn } from "react-hook-form";

import { t } from "@/app/i18n";
import { Field, SelectField } from "@/components/ui/field";
import type { FieldDef, MemberDetail, ObjectSchema } from "@/lib/api/types";
import { formatFieldValue, orderedFields } from "@/lib/fields";

/** Entradas dos campos customizados da guilda, montadas a partir do schema. */
export function CustomFieldInputs({ schema, register, error }: {
  schema: ObjectSchema;
  register: (key: string) => UseFormRegisterReturn;
  error: (key: string) => string | undefined;
}) {
  const fields = orderedFields(schema);
  if (!fields.length) return null;
  return (
    <fieldset className="flex flex-col gap-4">
      <legend className="mb-3 text-sm font-semibold">{t.fields.section}</legend>
      <div className="grid gap-4 sm:grid-cols-2">
        {fields.map(([key, def]) => (
          <CustomFieldInput key={key} id={`field-${key}`} def={def} required={schema.required.includes(key)}
            registration={register(key)} error={error(key)} />
        ))}
      </div>
    </fieldset>
  );
}

function hint(def: FieldDef): string | undefined {
  const { minimum: min, maximum: max } = def;
  if (min !== undefined && max !== undefined) return `${min} a ${max}`;
  if (min !== undefined) return `mínimo ${min}`;
  if (max !== undefined) return `máximo ${max}`;
  return undefined;
}

function CustomFieldInput({ id, def, required, registration, error }: {
  id: string;
  def: FieldDef;
  required: boolean;
  registration: UseFormRegisterReturn;
  error?: string;
}) {
  const label = required ? `${def.title} *` : def.title;
  if (def.type === "boolean") {
    return (
      <div className="flex flex-col gap-1.5 sm:pt-7">
        <label htmlFor={id} className="flex items-center gap-2 text-sm">
          <input id={id} type="checkbox" className="size-4 accent-[var(--color-primary)]" {...registration} />
          {label}
        </label>
        {error && <p className="text-xs text-danger">{error}</p>}
      </div>
    );
  }
  if (def.enum) {
    return (
      <SelectField id={id} label={label} {...registration} error={error}>
        <option value="">{t.fields.choose}</option>
        {def.enum.map((o) => <option key={String(o)} value={String(o)}>{formatFieldValue(def, o)}</option>)}
      </SelectField>
    );
  }
  const numeric = def.type !== "string";
  return (
    <Field id={id} label={label} {...registration} error={error} autoComplete="off"
      inputMode={def.type === "integer" ? "numeric" : numeric ? "decimal" : undefined}
      maxLength={numeric ? undefined : (def.maxLength ?? 500)} hint={numeric ? hint(def) : undefined} />
  );
}

/** Valores dos campos customizados na ficha (só leitura). */
export function CustomFieldValues({ schema, m }: { schema: ObjectSchema; m: MemberDetail }) {
  return (
    <>
      {orderedFields(schema).map(([key, def]) => (
        <div key={key} className="contents">
          <dt className="text-muted">{def.title}</dt>
          <dd className="break-words">{formatFieldValue(def, m.fields[key])}</dd>
        </div>
      ))}
    </>
  );
}
