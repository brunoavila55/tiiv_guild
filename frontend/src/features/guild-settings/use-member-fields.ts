import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { get, put, retryUnlessClientError } from "@/lib/api/client";
import type { ObjectSchema } from "@/lib/api/types";

const key = ["member-fields"] as const;

/** Schema dos campos customizados de membro (quem vê membros pode ler). */
export function useMemberFields() {
  return useQuery({
    queryKey: key,
    queryFn: () => get<ObjectSchema>("/settings/member-fields"),
    retry: retryUnlessClientError,
    staleTime: 60_000,
  });
}

export function useSaveMemberFields() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (schema: ObjectSchema) => put<ObjectSchema>("/settings/member-fields", schema),
    onSuccess: async (saved) => {
      qc.setQueryData(key, saved);
      // As fichas mostram só os campos do schema atual.
      await qc.invalidateQueries({ queryKey: ["members"] });
    },
  });
}
