import type { ReactNode } from "react";

import { appUrl } from "@/app/host";
import { t } from "@/app/i18n";
import { ButtonLink } from "@/components/ui/button";
import { Card, PageMessage } from "@/components/ui/card";
import { loginUrlReturningHere } from "@/features/auth/use-me";
import { ApiError } from "@/lib/api/client";
import type { Guild } from "@/lib/api/types";

import { useGuild } from "./use-tenant";

/** Só renderiza o painel para membros; senão pede login ou avisa que não é membro. */
export function PanelGate({ children }: { children: (guild: Guild) => ReactNode }) {
  const guild = useGuild();
  if (guild.isPending) return <PageMessage>{t.common.loading}</PageMessage>;
  if (guild.error instanceof ApiError && guild.error.status === 401) {
    return (
      <Card className="mx-auto max-w-md text-center">
        <p className="mb-4 text-muted">{t.auth.loginToContinue}</p>
        <ButtonLink href={loginUrlReturningHere()}>{t.auth.submitLogin}</ButtonLink>
      </Card>
    );
  }
  if (guild.error instanceof ApiError && guild.error.status === 403) {
    return (
      <Card className="mx-auto max-w-md text-center">
        <p className="mb-4 text-muted">{t.panel.notMember}</p>
        <ButtonLink variant="outline" href={appUrl("/")}>
          {t.guilds.mine}
        </ButtonLink>
      </Card>
    );
  }
  if (!guild.data) return <PageMessage>{t.common.genericError}</PageMessage>;
  return <>{children(guild.data)}</>;
}
