import { zodResolver } from "@hookform/resolvers/zod";
import { useState } from "react";
import { useForm } from "react-hook-form";

import { t } from "@/app/i18n";
import { Button } from "@/components/ui/button";
import { Card, FormError, PageMessage } from "@/components/ui/card";
import { Dialog } from "@/components/ui/dialog";
import { Field, SelectField } from "@/components/ui/field";
import { PanelGate } from "@/features/guild-panel/panel-gate";
import { useRanks } from "@/features/members/use-members";
import { ApiError } from "@/lib/api/client";
import type { Rank } from "@/lib/api/types";
import { applyApiError } from "@/lib/forms";

import { canMove, canToggle, movedIds, PERMISSIONS, rankSchema, reassignTargets, type RankForm } from "./rules";
import { useCreateRank, useDeleteRank, useReorderRanks, useUpdateRank } from "./use-ranks";

const permLabel = (p: string) => t.ranks.perms[p] ?? p;

export function RanksPage() {
  return (
    <PanelGate>
      {(g) =>
        g.me.permissions.includes("ranks.manage") ? (
          <RanksLoader mine={g.me.permissions} />
        ) : (
          <Card className="mx-auto max-w-md text-center text-muted">{t.ranks.noPermission}</Card>
        )
      }
    </PanelGate>
  );
}

type Editing = { kind: "new" } | { kind: "edit"; rank: Rank } | { kind: "remove"; rank: Rank } | null;

function RanksLoader({ mine }: { mine: string[] }) {
  const ranks = useRanks();
  const reorder = useReorderRanks();
  const [editing, setEditing] = useState<Editing>(null);
  const [error, setError] = useState<string | null>(null);
  if (ranks.isPending) return <PageMessage>{t.common.loading}</PageMessage>;
  if (!ranks.data) return <PageMessage>{t.common.genericError}</PageMessage>;
  const list = ranks.data;
  const close = () => setEditing(null);

  const move = (index: number, delta: -1 | 1) => {
    setError(null);
    reorder.mutate(movedIds(list, index, delta), {
      onError: (e) => setError(e instanceof ApiError ? e.message : t.common.genericError),
    });
  };

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold">{t.ranks.title}</h1>
          <p className="max-w-2xl text-sm text-muted">{t.ranks.hint}</p>
        </div>
        <Button type="button" onClick={() => setEditing({ kind: "new" })}>+ {t.ranks.add}</Button>
      </div>
      <FormError message={error} />

      <ol className="flex flex-col gap-3">
        {list.map((rank, i) => (
          <li key={rank.id}>
            <Card className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
              <div className="flex min-w-0 flex-col gap-2">
                <div className="flex flex-wrap items-baseline gap-x-3 gap-y-1">
                  <span className="text-xs tabular-nums text-muted">#{i + 1}</span>
                  <h2 className="font-semibold">{rank.name}</h2>
                  <span className="text-sm text-muted">{t.ranks.members(rank.member_count)}</span>
                </div>
                <div className="flex flex-wrap gap-1.5">
                  {rank.permissions.length === 0 && <span className="text-xs text-muted">{t.ranks.noPermissions}</span>}
                  {rank.permissions.map((p) => (
                    <span key={p} className="rounded-md bg-surface-2 px-2 py-0.5 text-xs">{permLabel(p)}</span>
                  ))}
                </div>
              </div>
              {rank.can_manage && (
                <div className="flex shrink-0 gap-1">
                  <Button type="button" variant="ghost" className="h-9" aria-label={t.ranks.moveUp}
                    disabled={!canMove(list, i, -1) || reorder.isPending} onClick={() => move(i, -1)}>↑</Button>
                  <Button type="button" variant="ghost" className="h-9" aria-label={t.ranks.moveDown}
                    disabled={!canMove(list, i, 1) || reorder.isPending} onClick={() => move(i, 1)}>↓</Button>
                  <Button type="button" variant="ghost" className="h-9" onClick={() => setEditing({ kind: "edit", rank })}>
                    {t.ranks.edit}
                  </Button>
                  <Button type="button" variant="ghost" className="h-9 text-danger hover:text-danger"
                    disabled={list.length === 1} onClick={() => setEditing({ kind: "remove", rank })}>
                    {t.ranks.remove}
                  </Button>
                </div>
              )}
            </Card>
          </li>
        ))}
      </ol>

      {editing?.kind === "new" && <RankDialog mine={mine} onClose={close} />}
      {editing?.kind === "edit" && <RankDialog mine={mine} rank={editing.rank} onClose={close} />}
      {editing?.kind === "remove" && <RemoveRankDialog ranks={list} rank={editing.rank} onClose={close} />}
    </div>
  );
}

function RankDialog({ mine, rank, onClose }: { mine: string[]; rank?: Rank; onClose: () => void }) {
  const create = useCreateRank();
  const update = useUpdateRank(rank?.id ?? "");
  const save = rank ? update : create;
  const [formError, setFormError] = useState<string | null>(null);
  const form = useForm<RankForm>({
    resolver: zodResolver(rankSchema),
    defaultValues: { name: rank?.name ?? "", permissions: rank?.permissions ?? [] },
  });
  const errors = form.formState.errors;
  const checked = form.watch("permissions");

  const toggle = (p: string, on: boolean) =>
    form.setValue("permissions", on ? [...checked, p] : checked.filter((c) => c !== p), { shouldDirty: true });

  const submit = form.handleSubmit((data) =>
    save.mutate(data, {
      onSuccess: onClose,
      onError: (e) => setFormError(applyApiError(e, form.setError, ["name", "permissions"])),
    }),
  );

  return (
    <Dialog title={rank ? t.ranks.editTitle : t.ranks.addTitle} onClose={onClose}>
      <form onSubmit={submit} className="flex flex-col gap-4" noValidate>
        <FormError message={formError} />
        <Field label={t.ranks.name} autoFocus maxLength={32} {...form.register("name")} error={errors.name?.message} />
        <fieldset className="flex flex-col gap-2">
          <legend className="mb-1 text-sm font-medium">{t.ranks.permissions}</legend>
          {PERMISSIONS.map((p) => {
            const enabled = canToggle(mine, p);
            return (
              <label key={p} className={enabled ? "flex items-center gap-2 text-sm" : "flex items-center gap-2 text-sm text-muted"}>
                <input type="checkbox" className="size-4 accent-[var(--color-primary)]" disabled={!enabled}
                  checked={checked.includes(p)} onChange={(e) => toggle(p, e.target.checked)} />
                {permLabel(p)}
              </label>
            );
          })}
          {errors.permissions?.message && <p className="text-xs text-danger">{errors.permissions.message}</p>}
          {PERMISSIONS.some((p) => !canToggle(mine, p)) && <p className="text-xs text-muted">{t.ranks.lackingHint}</p>}
        </fieldset>
        <div className="flex justify-end gap-2">
          <Button type="button" variant="ghost" onClick={onClose}>{t.common.cancel}</Button>
          <Button type="submit" disabled={save.isPending}>{t.common.save}</Button>
        </div>
      </form>
    </Dialog>
  );
}

function RemoveRankDialog({ ranks, rank, onClose }: { ranks: Rank[]; rank: Rank; onClose: () => void }) {
  const remove = useDeleteRank(rank.id);
  const targets = reassignTargets(ranks, rank);
  const inUse = rank.member_count > 0;
  const [target, setTarget] = useState(targets.at(-1)?.id ?? "");
  const [error, setError] = useState<string | null>(null);

  const confirm = () =>
    remove.mutate(inUse ? target : null, {
      onSuccess: onClose,
      onError: (e) =>
        setError(e instanceof ApiError ? [e.message, ...Object.values(e.fields)].join(" — ") : t.common.genericError),
    });

  return (
    <Dialog title={t.ranks.removeTitle} onClose={onClose}>
      <div className="flex flex-col gap-4">
        <FormError message={error} />
        <p className="text-sm">{t.ranks.removeConfirm(rank.name)}</p>
        {inUse &&
          (targets.length ? (
            <>
              <p className="text-sm text-muted">{t.ranks.reassignHint(rank.name, rank.member_count)}</p>
              <SelectField label={t.ranks.reassignTo} value={target} onChange={(e) => setTarget(e.target.value)}>
                {targets.map((r) => <option key={r.id} value={r.id}>{r.name}</option>)}
              </SelectField>
            </>
          ) : (
            <p className="text-sm text-danger">{t.ranks.noTarget}</p>
          ))}
        <div className="flex justify-end gap-2">
          <Button type="button" variant="ghost" onClick={onClose}>{t.common.cancel}</Button>
          <Button type="button" className="bg-danger" disabled={remove.isPending || (inUse && !target)} onClick={confirm}>
            {t.ranks.remove}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}
