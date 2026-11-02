import { useQuery } from "@tanstack/react-query";

import { guildUrl } from "@/app/host";
import { t } from "@/app/i18n";
import { ButtonLink } from "@/components/ui/button";
import { Card, PageMessage } from "@/components/ui/card";
import { loginUrlReturningHere } from "@/features/auth/use-me";
import { ApiError, get, retryUnlessClientError } from "@/lib/api/client";
import type { AdminGuild } from "@/lib/api/types";
import { formatDate } from "@/lib/utils";

/** `admin.BASE_DOMAIN` — painel da plataforma. */
export function AdminGuildsPage() {
  const guilds = useQuery({
    queryKey: ["admin", "guilds"],
    queryFn: () => get<AdminGuild[]>("/admin/guilds"),
    retry: retryUnlessClientError,
  });

  return (
    <div className="mx-auto min-h-screen max-w-5xl px-4">
      <header className="flex h-16 items-center border-b border-border">
        <span className="text-lg font-semibold">
          {t.brand} · {t.admin.title}
        </span>
      </header>
      <main className="py-8">
        {guilds.isPending ? (
          <PageMessage>{t.common.loading}</PageMessage>
        ) : guilds.error instanceof ApiError && guilds.error.status === 401 ? (
          <PageMessage>
            <ButtonLink href={loginUrlReturningHere()}>{t.auth.submitLogin}</ButtonLink>
          </PageMessage>
        ) : guilds.error instanceof ApiError && guilds.error.status === 403 ? (
          <PageMessage>{t.admin.forbidden}</PageMessage>
        ) : !guilds.data ? (
          <PageMessage>{t.common.genericError}</PageMessage>
        ) : (
          <div className="flex flex-col gap-6">
            <h1 className="text-2xl font-semibold">
              {t.admin.guilds} <span className="text-muted">({guilds.data.length})</span>
            </h1>
            <Card className="overflow-x-auto p-0">
              <table className="w-full text-left text-sm">
                <thead className="border-b border-border text-muted">
                  <tr>
                    <th className="px-4 py-3 font-medium">{t.guilds.name}</th>
                    <th className="px-4 py-3 font-medium">{t.admin.owner}</th>
                    <th className="px-4 py-3 font-medium">{t.admin.createdAt}</th>
                  </tr>
                </thead>
                <tbody>
                  {guilds.data.map((g) => (
                    <tr key={g.slug} className="border-b border-border last:border-0">
                      <td className="px-4 py-3">
                        <a href={guildUrl(g.slug)} className="hover:underline">
                          {g.name}
                        </a>{" "}
                        <span className="text-muted">{g.slug}</span>
                      </td>
                      <td className="px-4 py-3">{g.owner_email}</td>
                      <td className="px-4 py-3 text-muted">{formatDate(g.created_at)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </Card>
          </div>
        )}
      </main>
    </div>
  );
}
