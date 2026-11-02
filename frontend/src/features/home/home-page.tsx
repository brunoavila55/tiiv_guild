import { t } from "@/app/i18n";
import { useHealth } from "@/lib/api/health";
import { cn } from "@/lib/utils";

export function HomePage() {
  const health = useHealth();
  const label = health.isPending ? t.health.checking : health.isSuccess ? t.health.ok : t.health.down;

  return (
    <section className="space-y-4">
      <h1 className="text-3xl font-bold">{t.app.name}</h1>
      <p className="text-zinc-400">{t.app.tagline}</p>
      <p className="flex items-center gap-2 text-sm">
        <span
          className={cn(
            "size-2 rounded-full",
            health.isPending && "bg-zinc-500",
            health.isSuccess && "bg-emerald-500",
            health.isError && "bg-red-500",
          )}
        />
        {label}
      </p>
    </section>
  );
}
