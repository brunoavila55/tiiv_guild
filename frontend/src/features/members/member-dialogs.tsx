import { zodResolver } from "@hookform/resolvers/zod";
import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";

import { t } from "@/app/i18n";
import { Button } from "@/components/ui/button";
import { FormError, PageMessage } from "@/components/ui/card";
import { Dialog } from "@/components/ui/dialog";
import { Field, SelectField, TextAreaField } from "@/components/ui/field";
import type { MemberDetail, Rank } from "@/lib/api/types";
import { applyApiError } from "@/lib/forms";
import { formatDate } from "@/lib/utils";

import { StatusBadge } from "./status-badge";
import {
  ALL_STATUSES, describeHistory, editMemberSchema, NEW_MEMBER_STATUSES, newMemberSchema,
  type EditMemberForm, type NewMemberForm,
} from "./rules";
import { useAddMember, useMember, useMemberHistory, useRemoveMember, useUpdateMember } from "./use-members";

export function AddMemberDialog({ ranks, onClose, onAdded }: { ranks: Rank[]; onClose: () => void; onAdded: (id: string) => void }) {
  const add = useAddMember();
  const [formError, setFormError] = useState<string | null>(null);
  // Padrão: o rank mais baixo (cadastro típico é de quem está chegando).
  const form = useForm<NewMemberForm>({
    resolver: zodResolver(newMemberSchema),
    defaultValues: { nickname: "", rank_id: ranks.at(-1)?.id ?? "", status: "active", email: "", notes: "" },
  });
  const errors = form.formState.errors;

  const submit = form.handleSubmit((data) =>
    add.mutate(data, {
      onSuccess: (m) => onAdded(m.id),
      onError: (e) => setFormError(applyApiError(e, form.setError, ["nickname", "rank_id", "status", "email", "notes"])),
    }),
  );

  return (
    <Dialog title={t.members.addTitle} onClose={onClose}>
      <form onSubmit={submit} className="flex flex-col gap-4" noValidate>
        <FormError message={formError} />
        <Field label={t.members.nickname} autoFocus {...form.register("nickname")} error={errors.nickname?.message} />
        <div className="grid gap-4 sm:grid-cols-2">
          <SelectField label={t.panel.rank} {...form.register("rank_id")} error={errors.rank_id?.message}>
            {ranks.map((r) => <option key={r.id} value={r.id}>{r.name}</option>)}
          </SelectField>
          <SelectField label={t.panel.status} {...form.register("status")} error={errors.status?.message}>
            {NEW_MEMBER_STATUSES.map((s) => <option key={s} value={s}>{t.memberStatus[s]}</option>)}
          </SelectField>
        </div>
        <Field label={t.members.email} type="email" autoComplete="off" {...form.register("email")}
          error={errors.email?.message} hint={t.members.emailHint} />
        <TextAreaField label={t.members.notes} rows={3} {...form.register("notes")} error={errors.notes?.message} />
        <div className="flex justify-end gap-2">
          <Button type="button" variant="ghost" onClick={onClose}>{t.common.cancel}</Button>
          <Button type="submit" disabled={add.isPending}>{t.members.add}</Button>
        </div>
      </form>
    </Dialog>
  );
}

export function MemberDialog({ id, ranks, onClose }: { id: string; ranks: Rank[]; onClose: () => void }) {
  const member = useMember(id);
  return (
    <Dialog title={member.data?.nickname ?? t.panel.members} onClose={onClose}>
      {member.isPending ? (
        <PageMessage>{t.common.loading}</PageMessage>
      ) : !member.data ? (
        <PageMessage>{t.common.genericError}</PageMessage>
      ) : (
        <div className="flex flex-col gap-6">
          <Summary m={member.data} />
          {member.data.can_edit ? (
            <EditForm key={member.data.id} m={member.data} ranks={ranks} />
          ) : (
            <p className="text-sm text-muted">{t.members.readOnly}</p>
          )}
          {member.data.can_remove && <RemoveButton m={member.data} />}
          <History id={id} />
        </div>
      )}
    </Dialog>
  );
}

function Summary({ m }: { m: MemberDetail }) {
  return (
    <dl className="grid grid-cols-2 gap-x-4 gap-y-2 text-sm">
      <dt className="text-muted">{t.panel.status}</dt>
      <dd><StatusBadge status={m.status} /> {m.is_owner && <span className="text-xs text-muted">({t.members.owner})</span>}</dd>
      <dt className="text-muted">{t.members.account}</dt>
      <dd>{m.account_name ?? t.panel.noAccount}</dd>
      <dt className="text-muted">{t.panel.joinedAt}</dt>
      <dd>{formatDate(m.joined_at)}</dd>
      {m.left_at && (
        <>
          <dt className="text-muted">{t.members.leftAt}</dt>
          <dd>{formatDate(m.left_at)}</dd>
        </>
      )}
    </dl>
  );
}

function toForm(m: MemberDetail): EditMemberForm {
  return { nickname: m.nickname, rank_id: m.rank_id, status: m.status, notes: m.notes ?? "" };
}

function EditForm({ m, ranks }: { m: MemberDetail; ranks: Rank[] }) {
  const update = useUpdateMember(m.id);
  const [formError, setFormError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const form = useForm<EditMemberForm>({ resolver: zodResolver(editMemberSchema), defaultValues: toForm(m) });
  const errors = form.formState.errors;
  // Ficha mudou no servidor (ex.: removido) e não há edição em curso: mostra o valor novo.
  const { isDirty } = form.formState;
  useEffect(() => {
    if (!isDirty) form.reset(toForm(m));
  }, [m, isDirty, form]);
  // O dono nunca perde o acesso: só ativo ou em teste.
  const statuses = m.is_owner ? ALL_STATUSES.filter((s) => s === "active" || s === "trial") : ALL_STATUSES;

  const submit = form.handleSubmit((data) =>
    update.mutate(data, {
      onSuccess: (saved) => {
        form.reset(toForm(saved));
        setFormError(null);
        setNotice(t.members.saved);
      },
      onError: (e) => {
        setNotice(null);
        setFormError(applyApiError(e, form.setError, ["nickname", "rank_id", "status", "notes"]));
      },
    }),
  );

  return (
    <form onSubmit={submit} className="flex flex-col gap-4 border-t border-border pt-5" noValidate>
      <FormError message={formError} />
      {notice && !form.formState.isDirty && <p role="status" className="text-sm text-secondary">{notice}</p>}
      <Field label={t.members.nickname} {...form.register("nickname")} error={errors.nickname?.message} />
      <div className="grid gap-4 sm:grid-cols-2">
        <SelectField label={t.panel.rank} {...form.register("rank_id")} error={errors.rank_id?.message}>
          {ranks.map((r) => <option key={r.id} value={r.id}>{r.name}</option>)}
        </SelectField>
        <SelectField label={t.panel.status} {...form.register("status")} error={errors.status?.message}>
          {statuses.map((s) => <option key={s} value={s}>{t.memberStatus[s]}</option>)}
        </SelectField>
      </div>
      <TextAreaField label={t.members.notes} rows={3} {...form.register("notes")} error={errors.notes?.message} />
      <p className="-mt-2 text-xs text-muted">{t.members.notesHint}</p>
      <div className="flex justify-end">
        <Button type="submit" disabled={!form.formState.isDirty || update.isPending}>{t.common.save}</Button>
      </div>
    </form>
  );
}

function RemoveButton({ m }: { m: MemberDetail }) {
  const remove = useRemoveMember(m.id);
  const [error, setError] = useState<string | null>(null);
  const onClick = () => {
    if (!window.confirm(t.members.confirmRemove(m.nickname))) return;
    remove.mutate(undefined, { onError: (e) => setError(e instanceof Error ? e.message : t.common.genericError) });
  };
  return (
    <div className="flex flex-col gap-2 border-t border-border pt-5">
      <FormError message={error} />
      <div className="flex items-center justify-between gap-4">
        <p className="text-xs text-muted">{t.members.restoreHint}</p>
        <Button variant="outline" className="border-danger/50 text-danger hover:bg-danger/10" disabled={remove.isPending} onClick={onClick}>
          {t.members.remove}
        </Button>
      </div>
    </div>
  );
}

function History({ id }: { id: string }) {
  const history = useMemberHistory(id);
  return (
    <section className="flex flex-col gap-3 border-t border-border pt-5">
      <h3 className="text-sm font-semibold">{t.members.history}</h3>
      {history.isPending ? (
        <p className="text-sm text-muted">{t.common.loading}</p>
      ) : !history.data?.length ? (
        <p className="text-sm text-muted">{t.members.noHistory}</p>
      ) : (
        <ol className="flex max-h-60 flex-col gap-2 overflow-y-auto text-sm">
          {history.data.map((h) => (
            <li key={h.id} className="flex flex-col">
              <span>{describeHistory(h)}</span>
              <span className="text-xs text-muted">
                {formatDate(h.at)}
                {h.actor_nickname && ` · ${t.members.by(h.actor_nickname)}`}
              </span>
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}
