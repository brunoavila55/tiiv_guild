import { Link, Outlet } from "@tanstack/react-router";
import { useEffect, type CSSProperties } from "react";

import { appUrl, rootUrl } from "@/app/host";
import { t } from "@/app/i18n";
import { ButtonLink } from "@/components/ui/button";
import { PageMessage } from "@/components/ui/card";
import { ApiError } from "@/lib/api/client";

import { useTenant } from "./use-tenant";

/** Layout de `{slug}.`: aplica as cores da guilda e o cabeçalho com a identidade dela. */
export function TenantLayout() {
  const tenant = useTenant();

  useEffect(() => {
    if (tenant.data) document.title = `${tenant.data.name} · ${t.brand}`;
  }, [tenant.data]);

  if (tenant.isPending) return <PageMessage>{t.common.loading}</PageMessage>;
  if (tenant.error instanceof ApiError && tenant.error.status === 404) {
    return (
      <PageMessage>
        <p className="mb-4">{t.common.notFound}</p>
        <ButtonLink variant="outline" href={rootUrl()}>
          {t.brand}
        </ButtonLink>
      </PageMessage>
    );
  }
  if (!tenant.data) return <PageMessage>{t.common.genericError}</PageMessage>;

  const g = tenant.data;
  const style = {
    ...(g.primary_color && { "--guild-primary": g.primary_color }),
    ...(g.secondary_color && { "--guild-secondary": g.secondary_color }),
  } as CSSProperties;

  return (
    <div style={style} className="min-h-screen">
      <header className="border-b border-border bg-surface">
        <div className="mx-auto flex h-16 max-w-5xl items-center justify-between gap-4 px-4">
          <Link to="/" className="flex min-w-0 items-center gap-3">
            {g.logo_url ? (
              <img src={g.logo_url} alt="" className="size-9 rounded-lg object-cover" />
            ) : (
              <span className="grid size-9 place-items-center rounded-lg bg-primary text-sm font-bold text-white">
                {(g.tag ?? g.name).slice(0, 2).toUpperCase()}
              </span>
            )}
            <span className="truncate font-semibold">{g.name}</span>
          </Link>
          <nav className="flex items-center gap-1 text-sm">
            <Link to="/painel" className="rounded-lg px-3 py-2 text-muted hover:text-fg" activeOptions={{ exact: true }} activeProps={{ className: "text-fg" }}>
              {t.panel.overview}
            </Link>
            <Link to="/painel/membros" className="rounded-lg px-3 py-2 text-muted hover:text-fg" activeProps={{ className: "text-fg" }}>
              {t.panel.members}
            </Link>
            <a href={appUrl("/")} className="rounded-lg px-3 py-2 text-muted hover:text-fg">
              {t.guilds.mine}
            </a>
          </nav>
        </div>
      </header>
      <main className="mx-auto max-w-5xl px-4 py-8">
        <Outlet />
      </main>
    </div>
  );
}
