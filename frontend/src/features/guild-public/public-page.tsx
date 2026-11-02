import { Link } from "@tanstack/react-router";

import { t } from "@/app/i18n";
import { Card } from "@/components/ui/card";
import { useTenant } from "@/features/guild-panel/use-tenant";

/** `{slug}.BASE_DOMAIN/` — a página pública completa (branding + membros) entra no P1. */
export function PublicPage() {
  const tenant = useTenant();
  const g = tenant.data;
  if (!g) return null;
  return (
    <div className="flex flex-col items-center gap-6 py-16 text-center">
      <h1 className="text-4xl font-bold">
        {g.name} {g.tag && <span className="text-muted">[{g.tag}]</span>}
      </h1>
      {!g.public_page_enabled && <Card className="max-w-md text-muted">{t.panel.publicDisabled}</Card>}
      <Link
        to="/painel"
        className="inline-flex h-10 items-center rounded-lg bg-primary px-4 text-sm font-medium text-white hover:brightness-110"
      >
        {t.panel.goToPanel}
      </Link>
    </div>
  );
}
