import type { FieldValues, Path, UseFormSetError } from "react-hook-form";

import { t } from "@/app/i18n";

import { ApiError } from "./api/client";

/** Joga os erros por campo da API (422) no formulário; devolve a mensagem geral, se sobrar. */
export function applyApiError<T extends FieldValues>(
  error: unknown,
  setError: UseFormSetError<T>,
  fields: readonly Path<T>[],
): string | null {
  if (!(error instanceof ApiError)) return t.common.genericError;
  let rest = false;
  for (const [field, message] of Object.entries(error.fields)) {
    if ((fields as readonly string[]).includes(field)) setError(field as Path<T>, { message });
    else rest = true;
  }
  return Object.keys(error.fields).length === 0 || rest ? error.message : null;
}
