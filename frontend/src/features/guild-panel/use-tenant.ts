import { useQuery } from "@tanstack/react-query";

import { get, retryUnlessClientError } from "@/lib/api/client";
import type { Guild, Tenant } from "@/lib/api/types";

/** Identidade pública da guilda do host atual. */
export function useTenant() {
  return useQuery({ queryKey: ["tenant"], queryFn: () => get<Tenant>("/tenant"), retry: retryUnlessClientError, staleTime: 60_000 });
}

/** Guilda + vínculo do usuário logado (401 sem login, 403 se não é membro). */
export function useGuild() {
  return useQuery({ queryKey: ["guild"], queryFn: () => get<Guild>("/guild"), retry: retryUnlessClientError });
}

