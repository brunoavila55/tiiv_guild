import { describe, expect, it } from "vitest";

import type { Rank } from "@/lib/api/types";

import { canMove, canToggle, movedIds, rankSchema, reassignTargets } from "./rules";

const rank = (id: string, can_manage: boolean): Rank => ({
  id, name: id, position: 0, permissions: [], member_count: 0, can_manage,
});
const ranks = [rank("lider", false), rank("oficial", false), rank("membro", true), rank("recruta", true)];

describe("ranks", () => {
  it("move só entre ranks gerenciáveis", () => {
    expect(canMove(ranks, 2, 1)).toBe(true);
    expect(canMove(ranks, 2, -1)).toBe(false);
    expect(canMove(ranks, 3, 1)).toBe(false);
    expect(movedIds(ranks, 3, -1)).toEqual(["lider", "oficial", "recruta", "membro"]);
  });

  it("realoca só para outro rank atribuível", () => {
    expect(reassignTargets(ranks, ranks[2]!).map((r) => r.id)).toEqual(["recruta"]);
  });

  it("valida nome e permissões editáveis", () => {
    expect(rankSchema.safeParse({ name: "  ", permissions: [] }).success).toBe(false);
    expect(rankSchema.safeParse({ name: " Veterano ", permissions: [] }).data?.name).toBe("Veterano");
    expect(canToggle(["members.view"], "members.view")).toBe(true);
    expect(canToggle(["members.view"], "ranks.manage")).toBe(false);
  });
});
