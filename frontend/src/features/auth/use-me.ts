import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { appUrl } from "@/app/host";
import { get, post, retryUnlessClientError } from "@/lib/api/client";
import type { Me } from "@/lib/api/types";

export const meKey = ["me"] as const;

export function useMe() {
  return useQuery({ queryKey: meKey, queryFn: () => get<Me>("/me"), retry: retryUnlessClientError });
}

export function useLogout() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => post<void>("/auth/logout"),
    onSuccess: () => {
      qc.clear();
      window.location.href = appUrl("/entrar");
    },
  });
}

/** Link para o login em `app.` que volta para a página atual depois. */
export function loginUrlReturningHere() {
  return appUrl(`/entrar?next=${encodeURIComponent(window.location.href)}`);
}
