import { t } from "@/app/i18n";
import type { MemberStatus } from "@/lib/api/types";
import { cn } from "@/lib/utils";

const tone: Record<MemberStatus, string> = {
  active: "bg-secondary/15 text-secondary",
  trial: "bg-primary/15 text-primary",
  inactive: "bg-surface-2 text-muted",
  left: "bg-surface-2 text-muted",
  banned: "bg-danger/15 text-danger",
};

export function StatusBadge({ status }: { status: MemberStatus }) {
  return <span className={cn("rounded-full px-2 py-0.5 text-xs font-medium", tone[status])}>{t.memberStatus[status]}</span>;
}
