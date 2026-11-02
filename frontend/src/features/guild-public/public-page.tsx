import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";

import { t } from "@/app/i18n";
import { Card, PageMessage } from "@/components/ui/card";
import { Markdown } from "@/components/ui/markdown";
import { useGuild } from "@/features/guild-panel/use-tenant";
import { ApiError, get, retryUnlessClientError } from "@/lib/api/client";
import { SOCIAL_KEYS, type PublicPage as PublicPageData } from "@/lib/api/types";
import { safeHref } from "@/lib/markdown";
import { formatDate } from "@/lib/utils";

const linkButton = "inline-flex h-10 items-center rounded-lg px-4 text-sm font-medium transition";

/** `{slug}.BASE_DOMAIN/`: branding + membros ativos. Desligada, a API responde 404. */
export function PublicPage() {
  const page = useQuery({ queryKey: ["public"], queryFn: () => get<PublicPageData>("/public"), retry: retryUnlessClientError });
  const guild = useGuild();
  const isMember = Boolean(guild.data);

  if (page.isPending) return <PageMessage>{t.common.loading}</PageMessage>;
  if (page.error instanceof ApiError && page.error.status === 404) {
    const canPublish = guild.data?.me.permissions.includes("guild.branding.edit") ?? false;
    return (
      <Card className="mx-auto mt-16 flex max-w-md flex-col items-center gap-4 text-center">
        <p className="text-muted">{t.panel.publicDisabled}</p>
        <Link to={canPublish ? "/painel/visual" : "/painel"} className={`${linkButton} bg-primary text-white hover:brightness-110`}>
          {canPublish ? t.panel.publishIt : isMember ? t.panel.goToPanel : t.panel.membersArea}
        </Link>
      </Card>
    );
  }
  if (!page.data) return <PageMessage>{t.common.genericError}</PageMessage>;

  const g = page.data;
  const socials = SOCIAL_KEYS.flatMap((k) => {
    const href = g.socials[k];
    return href && safeHref(href) ? [{ key: k, href }] : [];
  });

  return (
    <div className="flex flex-col gap-6">
      <Card className="overflow-hidden p-0">
        <div
          className="h-40 bg-cover bg-center sm:h-56"
          style={g.banner_url ? { backgroundImage: `url(${g.banner_url})` } : { background: "linear-gradient(135deg, var(--color-primary), var(--color-secondary))" }}
        />
        <div className="flex flex-col gap-4 px-6 pb-6 sm:flex-row sm:items-end sm:justify-between">
          <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:gap-5">
            {g.logo_url ? (
              <img src={g.logo_url} alt="" className="-mt-12 size-24 rounded-2xl border-4 border-surface object-cover" />
            ) : (
              <span className="-mt-12 grid size-24 place-items-center rounded-2xl border-4 border-surface bg-primary text-2xl font-bold text-white">
                {(g.tag ?? g.name).slice(0, 2).toUpperCase()}
              </span>
            )}
            <div className="min-w-0">
              <h1 className="text-3xl font-bold break-words">
                {g.name} {g.tag && <span className="text-muted">[{g.tag}]</span>}
              </h1>
              {g.motto && <p className="mt-1 text-primary">{g.motto}</p>}
              <p className="mt-2 text-sm text-muted">
                {t.panel.memberCount(g.member_count)} · {t.panel.foundedAt} {formatDate(g.created_at)}
              </p>
            </div>
          </div>
          <Link
            to="/painel"
            className={isMember ? `${linkButton} bg-primary text-white hover:brightness-110` : `${linkButton} border border-border hover:bg-surface-2`}
          >
            {isMember ? t.panel.goToPanel : t.panel.membersArea}
          </Link>
        </div>
        {socials.length > 0 && (
          <div className="flex flex-wrap gap-2 border-t border-border px-6 py-4">
            {socials.map((s) => (
              <a key={s.key} href={s.href} target="_blank" rel="noopener noreferrer nofollow"
                className="rounded-lg bg-secondary/15 px-3 py-1.5 text-sm text-secondary hover:bg-secondary/25">
                {t.branding.socialLabels[s.key]}
              </a>
            ))}
          </div>
        )}
      </Card>

      <div className="grid gap-6 lg:grid-cols-[1fr_320px]">
        <Card className="min-w-0">
          <h2 className="mb-4 text-lg font-semibold">{t.panel.about}</h2>
          {g.description_md ? <Markdown source={g.description_md} className="text-muted" /> : <p className="text-muted">{g.motto ?? "—"}</p>}
        </Card>

        <Card className="flex flex-col gap-5 lg:self-start">
          <h2 className="text-lg font-semibold">{t.panel.roster}</h2>
          {g.ranks.length === 0 && <p className="text-sm text-muted">{t.panel.noMembers}</p>}
          {g.ranks.map((r, i) => (
            <section key={i} className="flex flex-col gap-2">
              <h3 className="flex items-center justify-between text-sm font-medium text-primary">
                {r.name} <span className="text-xs text-muted">{r.members.length}</span>
              </h3>
              <ul className="flex flex-wrap gap-2">
                {r.members.map((m, j) => (
                  <li key={j} className="rounded-md bg-surface-2 px-2 py-1 text-sm">{m}</li>
                ))}
              </ul>
            </section>
          ))}
        </Card>
      </div>
    </div>
  );
}
