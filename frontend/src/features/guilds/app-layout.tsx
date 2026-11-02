import { Link, Outlet } from "@tanstack/react-router";

import { t } from "@/app/i18n";
import { Button } from "@/components/ui/button";
import { useLogout, useMe } from "@/features/auth/use-me";

/** Layout de `app.`: conta do usuário. */
export function AppLayout() {
  const me = useMe();
  const logout = useLogout();
  return (
    <div className="mx-auto min-h-screen max-w-5xl px-4">
      <header className="flex h-16 items-center justify-between border-b border-border">
        <Link to="/" className="text-lg font-semibold">
          {t.brand}
        </Link>
        {me.data && (
          <div className="flex items-center gap-3 text-sm">
            <span className="text-muted">{me.data.user.display_name}</span>
            <Button variant="ghost" onClick={() => logout.mutate()}>
              {t.auth.logout}
            </Button>
          </div>
        )}
      </header>
      <main className="py-8">
        <Outlet />
      </main>
    </div>
  );
}
