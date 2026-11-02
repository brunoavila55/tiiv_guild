import { t } from "@/app/i18n";
import { Card } from "@/components/ui/card";

import { PanelGate } from "./panel-gate";

export function PanelHome() {
  return (
    <PanelGate>
      {(g) => (
        <div className="flex flex-col gap-6">
          <h1 className="text-2xl font-semibold">
            {g.name} {g.tag && <span className="text-muted">[{g.tag}]</span>}
          </h1>
          <Card className="flex flex-col gap-3">
            <p>
              {t.panel.youAre} <strong>{g.me.nickname}</strong> · <span className="text-primary">{g.me.rank_name}</span>
            </p>
            <div>
              <p className="mb-2 text-sm text-muted">{t.panel.permissions}</p>
              <div className="flex flex-wrap gap-2">
                {g.me.permissions.map((p) => (
                  <code key={p} className="rounded-md bg-surface-2 px-2 py-1 text-xs">
                    {p}
                  </code>
                ))}
              </div>
            </div>
          </Card>
        </div>
      )}
    </PanelGate>
  );
}
