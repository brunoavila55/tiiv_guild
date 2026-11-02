import { Link, Navigate } from "@tanstack/react-router";

import { guildUrl } from "@/app/host";
import { t } from "@/app/i18n";
import { ButtonLink } from "@/components/ui/button";
import { Card, PageMessage } from "@/components/ui/card";
import { useMe } from "@/features/auth/use-me";
import { ApiError } from "@/lib/api/client";

export function MyGuildsPage() {
  const me = useMe();
  if (me.error instanceof ApiError && me.error.status === 401) return <Navigate to="/entrar" />;
  if (me.isPending) return <PageMessage>{t.common.loading}</PageMessage>;
  if (!me.data) return <PageMessage>{t.common.genericError}</PageMessage>;

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">{t.guilds.mine}</h1>
        <Link
          to="/nova-guilda"
          className="inline-flex h-10 items-center rounded-lg bg-primary px-4 text-sm font-medium text-white hover:brightness-110"
        >
          {t.guilds.create}
        </Link>
      </div>
      {me.data.guilds.length === 0 ? (
        <Card className="text-center text-muted">{t.guilds.none}</Card>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2">
          {me.data.guilds.map((g) => (
            <Card key={g.slug} className="flex items-center justify-between gap-4">
              <div className="min-w-0">
                <p className="truncate font-semibold">
                  {g.name} {g.tag && <span className="text-muted">[{g.tag}]</span>}
                </p>
                <p className="truncate text-sm text-muted">
                  {g.is_owner ? t.guilds.owner : g.rank_name} · {g.slug}
                </p>
              </div>
              <ButtonLink variant="outline" href={guildUrl(g.slug, "/painel")}>
                {t.guilds.open}
              </ButtonLink>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
