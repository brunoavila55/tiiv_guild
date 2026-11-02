import { t } from "@/app/i18n";
import { Card, PageMessage } from "@/components/ui/card";
import { PanelGate } from "@/features/guild-panel/panel-gate";
import { useMembers } from "@/features/guild-panel/use-tenant";
import { formatDate } from "@/lib/utils";

export function MembersPage() {
  return <PanelGate>{() => <MembersTable />}</PanelGate>;
}

function MembersTable() {
  const members = useMembers();
  if (members.isPending) return <PageMessage>{t.common.loading}</PageMessage>;
  if (!members.data) return <PageMessage>{t.common.genericError}</PageMessage>;

  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-semibold">
        {t.panel.members} <span className="text-muted">({members.data.length})</span>
      </h1>
      <Card className="overflow-x-auto p-0">
        <table className="w-full text-left text-sm">
          <thead className="border-b border-border text-muted">
            <tr>
              <th className="px-4 py-3 font-medium">Nick</th>
              <th className="px-4 py-3 font-medium">{t.panel.rank}</th>
              <th className="px-4 py-3 font-medium">{t.panel.status}</th>
              <th className="px-4 py-3 font-medium">{t.panel.joinedAt}</th>
            </tr>
          </thead>
          <tbody>
            {members.data.map((m) => (
              <tr key={m.id} className="border-b border-border last:border-0">
                <td className="px-4 py-3">
                  {m.nickname}
                  {!m.has_account && <span className="ml-2 text-xs text-muted">({t.panel.noAccount})</span>}
                </td>
                <td className="px-4 py-3">{m.rank_name}</td>
                <td className="px-4 py-3">{t.memberStatus[m.status]}</td>
                <td className="px-4 py-3 text-muted">{formatDate(m.joined_at)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </Card>
    </div>
  );
}
