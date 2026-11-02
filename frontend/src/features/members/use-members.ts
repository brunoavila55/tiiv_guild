import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { del, get, patch, post, retryUnlessClientError } from "@/lib/api/client";
import type { HistoryEntry, MemberDetail, MemberPage, Rank } from "@/lib/api/types";

import type { EditMemberForm, NewMemberForm } from "./rules";

export function useMemberPage(query: string) {
  return useQuery({
    queryKey: ["members", "page", query],
    queryFn: () => get<MemberPage>(`/members${query}`),
    retry: retryUnlessClientError,
    placeholderData: keepPreviousData,
  });
}

export function useRanks() {
  return useQuery({ queryKey: ["ranks"], queryFn: () => get<Rank[]>("/ranks"), retry: retryUnlessClientError, staleTime: 60_000 });
}

export function useMember(id: string) {
  return useQuery({ queryKey: ["members", "detail", id], queryFn: () => get<MemberDetail>(`/members/${id}`), retry: retryUnlessClientError });
}

export function useMemberHistory(id: string) {
  return useQuery({
    queryKey: ["members", "history", id],
    queryFn: () => get<HistoryEntry[]>(`/members/${id}/history`),
    retry: retryUnlessClientError,
  });
}

/** Toda escrita invalida listas, fichas e históricos (e a página pública, que lista os ativos). */
function useInvalidateMembers() {
  const qc = useQueryClient();
  return () => Promise.all([qc.invalidateQueries({ queryKey: ["members"] }), qc.invalidateQueries({ queryKey: ["public"] })]);
}

export function useAddMember() {
  const invalidate = useInvalidateMembers();
  return useMutation({
    mutationFn: (f: NewMemberForm) =>
      post<MemberDetail>("/members", { ...f, email: f.email || null, notes: f.notes || null }),
    onSuccess: invalidate,
  });
}

export function useUpdateMember(id: string) {
  const invalidate = useInvalidateMembers();
  return useMutation({
    mutationFn: (f: EditMemberForm) => patch<MemberDetail>(`/members/${id}`, f),
    onSuccess: invalidate,
  });
}

export function useRemoveMember(id: string) {
  const invalidate = useInvalidateMembers();
  return useMutation({ mutationFn: () => del<void>(`/members/${id}`), onSuccess: invalidate });
}
