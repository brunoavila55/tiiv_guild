import { appUrl, BASE_DOMAIN } from "@/app/host";
import { t } from "@/app/i18n";
import { ButtonLink } from "@/components/ui/button";
import { Card } from "@/components/ui/card";

export function LandingPage() {
  return (
    <div className="mx-auto flex min-h-screen max-w-5xl flex-col px-4">
      <header className="flex h-16 items-center justify-between">
        <span className="text-lg font-semibold">{t.brand}</span>
        <ButtonLink variant="ghost" href={appUrl("/entrar")}>
          {t.landing.login}
        </ButtonLink>
      </header>
      <main className="flex flex-1 flex-col items-center justify-center gap-10 py-16 text-center">
        <h1 className="max-w-2xl text-4xl font-bold tracking-tight sm:text-5xl">{t.tagline}</h1>
        <ButtonLink href={appUrl("/cadastro")} className="h-12 px-6 text-base">
          {t.landing.cta}
        </ButtonLink>
        <div className="grid w-full gap-4 text-left sm:grid-cols-3">
          {t.landing.features.map(([title, body]) => (
            <Card key={title}>
              <h2 className="mb-2 font-semibold">{title}</h2>
              <p className="text-sm text-muted">{body.replace("{base}", BASE_DOMAIN)}</p>
            </Card>
          ))}
        </div>
      </main>
    </div>
  );
}
