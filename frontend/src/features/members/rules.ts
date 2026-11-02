// Regras de membros do lado do cliente (espelho de `domain::members`); o backend revalida tudo.
import { z } from "zod";

import { t } from "@/app/i18n";
import type { HistoryEntry, MemberStatus } from "@/lib/api/types";

/** Status que dá para escolher ao cadastrar (sair/banir é remoção). */
export const NEW_MEMBER_STATUSES = ["active", "trial", "inactive"] as const;
export const ALL_STATUSES: readonly MemberStatus[] = ["active", "trial", "inactive", "left", "banned"];

const nickname = z.string().trim().min(2, "deve ter entre 2 e 32 caracteres").max(32, "deve ter entre 2 e 32 caracteres");
const notes = z.string().trim().max(2000, "máximo de 2000 caracteres");

export const newMemberSchema = z.object({
  nickname,
  rank_id: z.string().min(1, "escolha um rank"),
  status: z.enum(NEW_MEMBER_STATUSES),
  email: z.string().trim().refine((v) => v === "" || /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(v), "e-mail inválido"),
  notes,
});
export type NewMemberForm = z.infer<typeof newMemberSchema>;

export const editMemberSchema = z.object({
  nickname,
  rank_id: z.string().min(1, "escolha um rank"),
  status: z.enum(ALL_STATUSES as [MemberStatus, ...MemberStatus[]]),
  notes,
});
export type EditMemberForm = z.infer<typeof editMemberSchema>;

/** Filtros da listagem como query string (só o que está preenchido). */
export function membersQuery(f: { q: string; status: string; rank_id: string; page: number; per_page: number }) {
  const p = new URLSearchParams();
  if (f.q.trim()) p.set("q", f.q.trim());
  if (f.status) p.set("status", f.status);
  if (f.rank_id) p.set("rank_id", f.rank_id);
  if (f.page > 1) p.set("page", String(f.page));
  p.set("per_page", String(f.per_page));
  return `?${p.toString()}`;
}

function label(key: string, value: unknown): string {
  if (key === "status" && typeof value === "string" && value in t.memberStatus) {
    return t.memberStatus[value as MemberStatus];
  }
  return String(value);
}

/** Uma linha legível para um registro do histórico. */
export function describeHistory(h: HistoryEntry): string {
  const title = t.members.actions[h.action] ?? h.action;
  if (h.action !== "member.updated") return title;
  // Ordem fixa: o jsonb não preserva a ordem das chaves.
  const order = Object.keys(t.members.changed);
  const rank = (k: string) => (order.includes(k) ? order.indexOf(k) : order.length);
  const entries = Object.entries(h.payload).sort(([a], [b]) => rank(a) - rank(b));
  const parts = entries.map(([key, change]) => {
    const name = t.members.changed[key] ?? key;
    if (change && typeof change === "object" && "from" in change && "to" in change) {
      return `${name}: ${label(key, change.from)} → ${label(key, change.to)}`;
    }
    return name;
  });
  return parts.length ? `${title} — ${parts.join("; ")}` : title;
}
