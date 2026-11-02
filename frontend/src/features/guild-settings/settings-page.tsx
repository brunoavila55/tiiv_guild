import { useState } from "react";

import { t } from "@/app/i18n";
import { Button } from "@/components/ui/button";
import { Card, FormError, PageMessage } from "@/components/ui/card";
import { Field, SelectField, TextAreaField } from "@/components/ui/field";
import { PanelGate } from "@/features/guild-panel/panel-gate";
import { ApiError } from "@/lib/api/client";
import type { ObjectSchema } from "@/lib/api/types";

import { FIELD_TYPES, keyFromTitle, MAX_FIELDS, newRow, rowErrors, rowsFromSchema, schemaFromRows, type FieldRow } from "./rules";
import { useMemberFields, useSaveMemberFields } from "./use-member-fields";

export function SettingsPage() {
  return (
    <PanelGate>
      {(g) =>
        g.me.permissions.includes("guild.settings.edit") ? (
          <SettingsLoader />
        ) : (
          <Card className="mx-auto max-w-md text-center text-muted">{t.settings.noPermission}</Card>
        )
      }
    </PanelGate>
  );
}

function SettingsLoader() {
  const schema = useMemberFields();
  if (schema.isPending) return <PageMessage>{t.common.loading}</PageMessage>;
  if (!schema.data) return <PageMessage>{t.common.genericError}</PageMessage>;
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-semibold">{t.settings.title}</h1>
      <MemberFieldsEditor saved={schema.data} />
    </div>
  );
}

function MemberFieldsEditor({ saved }: { saved: ObjectSchema }) {
  const save = useSaveMemberFields();
  const [rows, setRows] = useState(() => rowsFromSchema(saved));
  const [dirty, setDirty] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [formError, setFormError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const change = (next: FieldRow[]) => {
    setRows(next);
    setDirty(true);
    setNotice(null);
  };
  const update = (uid: string, patch: Partial<FieldRow>) =>
    change(
      rows.map((r) => {
        if (r.uid !== uid) return r;
        const next = { ...r, ...patch };
        // Campo novo: a chave acompanha o nome até ser salvo.
        if (!r.saved && patch.title !== undefined) {
          next.key = keyFromTitle(patch.title, rows.filter((o) => o.uid !== uid).map((o) => o.key));
        }
        return next;
      }),
    );
  const move = (index: number, delta: number) => {
    const next = [...rows];
    const [row] = next.splice(index, 1);
    if (row) next.splice(index + delta, 0, row);
    change(next);
  };

  const discard = () => {
    setRows(rowsFromSchema(saved));
    setDirty(false);
    setErrors({});
    setFormError(null);
  };

  const submit = () => {
    const local = rowErrors(rows);
    setErrors(local);
    setNotice(null);
    if (Object.keys(local).length) return setFormError(null);
    save.mutate(schemaFromRows(rows), {
      onSuccess: (s) => {
        setRows(rowsFromSchema(s));
        setDirty(false);
        setFormError(null);
        setNotice(t.settings.saved);
      },
      onError: (e) => {
        if (!(e instanceof ApiError)) return setFormError(t.common.genericError);
        // `properties.<chave>` → linha; o resto vai para o topo.
        const byRow: Record<string, string> = {};
        const rest: string[] = [];
        for (const [path, message] of Object.entries(e.fields)) {
          const row = rows.find((r) => path === `properties.${r.key}`);
          if (row) byRow[row.uid] = message;
          else rest.push(message);
        }
        setErrors(byRow);
        setFormError(rest.length || !Object.keys(byRow).length ? [e.message, ...rest].join(" — ") : null);
      },
    });
  };

  return (
    <section className="flex flex-col gap-4">
      <div>
        <h2 className="text-lg font-semibold">{t.settings.memberFields}</h2>
        <p className="text-sm text-muted">{t.settings.memberFieldsHint}</p>
      </div>
      <FormError message={formError} />
      {notice && !dirty && (
        <p role="status" className="rounded-lg border border-secondary/40 bg-secondary/10 px-3 py-2 text-sm text-secondary">{notice}</p>
      )}

      {rows.length === 0 && <Card className="text-center text-sm text-muted">{t.settings.noFields}</Card>}
      {rows.map((row, i) => (
        <FieldRowEditor key={row.uid} row={row} error={errors[row.uid]} first={i === 0} last={i === rows.length - 1}
          onChange={(patch) => update(row.uid, patch)} onMove={(d) => move(i, d)}
          onRemove={() => change(rows.filter((r) => r.uid !== row.uid))} />
      ))}

      <div className="flex flex-wrap items-center justify-between gap-3">
        <Button type="button" variant="outline" disabled={rows.length >= MAX_FIELDS} onClick={() => change([...rows, newRow()])}>
          + {t.settings.addField}
        </Button>
        <div className="flex gap-3">
          <Button type="button" variant="ghost" onClick={discard} disabled={!dirty || save.isPending}>{t.settings.discard}</Button>
          <Button type="button" onClick={submit} disabled={!dirty || save.isPending}>{t.common.save}</Button>
        </div>
      </div>
      <p className="text-xs text-muted">{t.settings.removeHint}</p>
    </section>
  );
}

function FieldRowEditor({ row, error, first, last, onChange, onMove, onRemove }: {
  row: FieldRow;
  error?: string;
  first: boolean;
  last: boolean;
  onChange: (patch: Partial<FieldRow>) => void;
  onMove: (delta: number) => void;
  onRemove: () => void;
}) {
  const id = (name: string) => `${row.uid}-${name}`;
  const numeric = row.type === "integer" || row.type === "number";
  const hasOptions = row.options.trim() !== "";
  return (
    <Card className={error ? "flex flex-col gap-4 border-danger/60" : "flex flex-col gap-4"}>
      {error && <p className="text-sm text-danger">{error}</p>}
      <div className="grid gap-4 sm:grid-cols-[1fr_1fr_180px]">
        <Field id={id("title")} label={t.settings.fieldTitle} value={row.title} maxLength={40}
          onChange={(e) => onChange({ title: e.target.value })} />
        <Field id={id("key")} label={t.settings.fieldKey} value={row.key} maxLength={32} disabled={row.saved}
          className="font-mono" hint={t.settings.fieldKeyHint}
          onChange={(e) => onChange({ key: e.target.value })} />
        <SelectField id={id("type")} label={t.settings.fieldType} value={row.type}
          onChange={(e) => onChange({ type: e.target.value as FieldRow["type"] })}>
          {FIELD_TYPES.map((type) => <option key={type} value={type}>{t.settings.types[type]}</option>)}
        </SelectField>
      </div>

      {row.type !== "boolean" && (
        <div className="grid gap-4 sm:grid-cols-[1fr_1fr_1fr]">
          <div className="flex flex-col gap-1">
            <TextAreaField id={id("options")} label={t.settings.options} rows={3} value={row.options}
              onChange={(e) => onChange({ options: e.target.value })} />
            <p className="text-xs text-muted">{t.settings.optionsHint}</p>
          </div>
          {!hasOptions && numeric && (
            <>
              <Field id={id("min")} label={t.settings.minimum} inputMode="decimal" value={row.minimum}
                onChange={(e) => onChange({ minimum: e.target.value })} />
              <Field id={id("max")} label={t.settings.maximum} inputMode="decimal" value={row.maximum}
                onChange={(e) => onChange({ maximum: e.target.value })} />
            </>
          )}
          {!hasOptions && row.type === "string" && (
            <>
              <Field id={id("minlen")} label={t.settings.minLength} inputMode="numeric" value={row.minLength}
                onChange={(e) => onChange({ minLength: e.target.value })} />
              <Field id={id("maxlen")} label={t.settings.maxLength} inputMode="numeric" value={row.maxLength} placeholder="500"
                onChange={(e) => onChange({ maxLength: e.target.value })} />
            </>
          )}
        </div>
      )}

      <div className="flex flex-wrap items-center justify-between gap-3 border-t border-border pt-4">
        <label className="flex items-center gap-2 text-sm">
          <input type="checkbox" className="size-4 accent-[var(--color-primary)]" checked={row.required}
            onChange={(e) => onChange({ required: e.target.checked })} />
          {t.settings.required}
        </label>
        <div className="flex gap-1">
          <Button type="button" variant="ghost" className="h-9" disabled={first} onClick={() => onMove(-1)} aria-label={t.settings.moveUp}>↑</Button>
          <Button type="button" variant="ghost" className="h-9" disabled={last} onClick={() => onMove(1)} aria-label={t.settings.moveDown}>↓</Button>
          <Button type="button" variant="ghost" className="h-9 text-danger hover:text-danger" onClick={onRemove}>{t.settings.removeField}</Button>
        </div>
      </div>
    </Card>
  );
}
