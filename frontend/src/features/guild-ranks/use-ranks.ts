import { useMutation, useQueryClient } from "@tanstack/react-query";

import { del, patch, post, put } from "@/lib/api/client";
import type { Rank } from "@/lib/api/types";

import type { RankForm } from "./rules";

/** Mexer em ranks muda listas e fichas de membros e, talvez, as permissões de quem está logado. */
function useInvalidateRanks() {
  const qc = useQueryClient();
  return () =>
    Promise.all(
      [["ranks"], ["members"], ["guild"], ["public"]].map((queryKey) => qc.invalidateQueries({ queryKey })),
    );
}

export function useCreateRank() {
  const invalidate = useInvalidateRanks();
  return useMutation({ mutationFn: (f: RankForm) => post<Rank>("/ranks", f), onSuccess: invalidate });
}

export function useUpdateRank(id: string) {
  const invalidate = useInvalidateRanks();
  return useMutation({ mutationFn: (f: RankForm) => patch<Rank>(`/ranks/${id}`, f), onSuccess: invalidate });
}

export function useReorderRanks() {
  const qc = useQueryClient();
  const invalidate = useInvalidateRanks();
  return useMutation({
    mutationFn: (ids: string[]) => put<Rank[]>("/ranks/order", { ids }),
    onSuccess: async (ranks) => {
      qc.setQueryData(["ranks"], ranks);
      await invalidate();
    },
  });
}

export function useDeleteRank(id: string) {
  const invalidate = useInvalidateRanks();
  return useMutation({
    mutationFn: (reassignTo: string | null) =>
      del<void>(`/ranks/${id}${reassignTo ? `?reassign_to=${encodeURIComponent(reassignTo)}` : ""}`),
    onSuccess: invalidate,
  });
}
