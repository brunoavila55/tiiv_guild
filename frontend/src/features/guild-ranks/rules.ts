// Regras de ranks do lado do cliente (espelho de `domain::ranks`); o backend revalida tudo.
import { z } from "zod";

import type { Rank } from "@/lib/api/types";

/** Permissões do core, na ordem canônica do backend (`Permission::ALL`). */
export const PERMISSIONS = [
  "guild.settings.edit",
  "guild.branding.edit",
  "members.view",
  "members.manage",
  "members.kick",
  "ranks.manage",
  "modules.manage",
] as const;

export const rankSchema = z.object({
  name: z.string().trim().min(1, "deve ter entre 1 e 32 caracteres").max(32, "deve ter entre 1 e 32 caracteres"),
  permissions: z.array(z.string()),
});
export type RankForm = z.infer<typeof rankSchema>;

/** Só troca de lugar com o vizinho se o usuário pode mexer nos dois. */
export function canMove(ranks: readonly Rank[], index: number, delta: -1 | 1): boolean {
  const other = ranks[index + delta];
  return Boolean(ranks[index]?.can_manage && other?.can_manage);
}

/** Ids na ordem nova, depois de mover o rank `index` uma casa para cima (-1) ou baixo (1). */
export function movedIds(ranks: readonly Rank[], index: number, delta: -1 | 1): string[] {
  const ids = ranks.map((r) => r.id);
  const [id] = ids.splice(index, 1);
  if (id !== undefined) ids.splice(index + delta, 0, id);
  return ids;
}

/** Ranks que podem receber os membros do apagado: os outros que o usuário pode atribuir. */
export function reassignTargets(ranks: readonly Rank[], removed: Rank): Rank[] {
  return ranks.filter((r) => r.id !== removed.id && r.can_manage);
}

/** Uma permissão só pode ser marcada/desmarcada por quem a tem (o dono tem todas). */
export function canToggle(mine: readonly string[], permission: string): boolean {
  return mine.includes(permission);
}
