import { useEffect, useState } from "react";

import { t } from "@/app/i18n";
import { Button } from "@/components/ui/button";
import { Card, PageMessage } from "@/components/ui/card";
import { PanelGate } from "@/features/guild-panel/panel-gate";
import type { Guild } from "@/lib/api/types";
import { cn, formatDate } from "@/lib/utils";

import { AddMemberDialog, MemberDialog } from "./member-dialogs";
import { StatusBadge } from "./status-badge";
import { ALL_STATUSES, membersQuery } from "./rules";
import { useMemberPage, useRanks } from "./use-members";

const PER_PAGE = 25;
const control = "h-10 rounded-lg border border-border bg-surface-2 px-3 text-sm outline-none transition focus:border-primary";

export function MembersPage() {
  return <PanelGate>{(g) => <Members guild={g} />}</PanelGate>;
}

/** Valor que só muda depois de `ms` sem alterações (busca enquanto digita). */
function useDebounced<T>(value: T, ms: number) {
  const [v, setV] = useState(value);
  useEffect(() => {
    const id = setTimeout(() => setV(value), ms);
    return () => clearTimeout(id);
  }, [value, ms]);
  return v;
}

function Members({ guild }: { guild: Guild }) {
  const canManage = guild.me.permissions.includes("members.manage");
  const [search, setSearch] = useState("");
  const [status, setStatus] = useState("");
  const [rankId, setRankId] = useState("");
  const [page, setPage] = useState(1);
  const [openId, setOpenId] = useState<string | null>(null);
  const [adding, setAdding] = useState(false);
  const q = useDebounced(search, 300);

  // Filtro novo volta para a primeira página.
  useEffect(() => setPage(1), [q, status, rankId]);

  const ranks = useRanks();
  const members = useMemberPage(membersQuery({ q, status, rank_id: rankId, page, per_page: PER_PAGE }));
  const data = members.data;
  const pages = data ? Math.max(1, Math.ceil(data.total / data.per_page)) : 1;
  const from = data && data.total ? (data.page - 1) * data.per_page + 1 : 0;
  const to = data ? from + data.items.length - (data.items.length ? 1 : 0) : 0;

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <h1 className="text-2xl font-semibold">
          {t.panel.members} {data && <span className="text-muted">({data.total})</span>}
        </h1>
        {canManage && <Button onClick={() => setAdding(true)}>{t.members.add}</Button>}
      </div>

      <div className="flex flex-wrap gap-3">
        <input type="search" value={search} onChange={(e) => setSearch(e.target.value)} placeholder={t.members.search}
          aria-label={t.members.search} className={cn(control, "min-w-56 flex-1")} />
        <select value={status} onChange={(e) => setStatus(e.target.value)} aria-label={t.panel.status} className={control}>
          <option value="">{t.members.allStatuses}</option>
          {ALL_STATUSES.map((s) => <option key={s} value={s}>{t.memberStatus[s]}</option>)}
        </select>
        <select value={rankId} onChange={(e) => setRankId(e.target.value)} aria-label={t.panel.rank} className={control}>
          <option value="">{t.members.allRanks}</option>
          {ranks.data?.map((r) => <option key={r.id} value={r.id}>{r.name}</option>)}
        </select>
      </div>

      {members.isPending ? (
        <PageMessage>{t.common.loading}</PageMessage>
      ) : !data ? (
        <PageMessage>{t.common.genericError}</PageMessage>
      ) : (
        <>
          <Card className={cn("overflow-x-auto p-0 transition-opacity", members.isPlaceholderData && "opacity-60")}>
            <table className="w-full text-left text-sm">
              <thead className="border-b border-border text-muted">
                <tr>
                  <th className="px-4 py-3 font-medium">{t.members.nickname}</th>
                  <th className="px-4 py-3 font-medium">{t.panel.rank}</th>
                  <th className="px-4 py-3 font-medium">{t.panel.status}</th>
                  <th className="px-4 py-3 font-medium">{t.panel.joinedAt}</th>
                </tr>
              </thead>
              <tbody>
                {data.items.map((m) => (
                  <tr key={m.id} onClick={() => setOpenId(m.id)}
                    className="cursor-pointer border-b border-border last:border-0 hover:bg-surface-2">
                    <td className="px-4 py-3">
                      <button type="button" className="text-left font-medium hover:underline"
                        onClick={(e) => { e.stopPropagation(); setOpenId(m.id); }}>
                        {m.nickname}
                      </button>
                      {!m.has_account && <span className="ml-2 text-xs text-muted">({t.panel.noAccount})</span>}
                    </td>
                    <td className="px-4 py-3">{m.rank_name}</td>
                    <td className="px-4 py-3"><StatusBadge status={m.status} /></td>
                    <td className="px-4 py-3 text-muted">{formatDate(m.joined_at)}</td>
                  </tr>
                ))}
                {data.items.length === 0 && (
                  <tr><td colSpan={4} className="px-4 py-12 text-center text-muted">{t.members.empty}</td></tr>
                )}
              </tbody>
            </table>
          </Card>

          {data.total > 0 && (
            <div className="flex items-center justify-between gap-4 text-sm text-muted">
              <span>{t.members.range(from, to, data.total)}</span>
              <div className="flex gap-2">
                <Button variant="outline" disabled={page <= 1} onClick={() => setPage((p) => p - 1)}>{t.members.prev}</Button>
                <Button variant="outline" disabled={page >= pages} onClick={() => setPage((p) => p + 1)}>{t.members.next}</Button>
              </div>
            </div>
          )}
        </>
      )}

      {openId && <MemberDialog id={openId} ranks={ranks.data ?? []} onClose={() => setOpenId(null)} />}
      {adding && <AddMemberDialog ranks={ranks.data ?? []} onClose={() => setAdding(false)} onAdded={(id) => { setAdding(false); setOpenId(id); }} />}
    </div>
  );
}

