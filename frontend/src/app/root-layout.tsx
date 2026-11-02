import { Outlet } from "@tanstack/react-router";

import { t } from "@/app/i18n";

export function RootLayout() {
  return (
    <div className="min-h-screen">
      <header className="border-b border-zinc-800 px-6 py-4">
        <span className="font-semibold text-guild-primary">{t.app.name}</span>
      </header>
      <main className="mx-auto max-w-5xl px-6 py-8">
        <Outlet />
      </main>
    </div>
  );
}
