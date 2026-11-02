// Cliente HTTP da API. A API está sempre em `/api` do host atual: o host define o tenant.

export class ApiError extends Error {
  constructor(
    readonly status: number,
    readonly code: string,
    message: string,
    readonly fields: Record<string, string> = {},
  ) {
    super(message);
  }
}

interface ErrorBody {
  code?: string;
  message?: string;
  fields?: Record<string, string>;
}

export async function api<T>(method: string, path: string, body?: unknown): Promise<T> {
  const isForm = body instanceof FormData;
  const res = await fetch(`/api${path}`, {
    method,
    credentials: "include",
    headers: body === undefined || isForm ? undefined : { "Content-Type": "application/json" },
    body: body === undefined ? undefined : isForm ? body : JSON.stringify(body),
  });
  if (res.status === 204) return undefined as T;
  const data: unknown = await res.json().catch(() => null);
  if (!res.ok) {
    const err = (data ?? {}) as ErrorBody;
    throw new ApiError(res.status, err.code ?? "unknown", err.message ?? res.statusText, err.fields);
  }
  return data as T;
}

export const get = <T>(path: string) => api<T>("GET", path);
export const post = <T>(path: string, body: unknown = {}) => api<T>("POST", path, body);
export const put = <T>(path: string, body: unknown) => api<T>("PUT", path, body);
export const patch = <T>(path: string, body: unknown) => api<T>("PATCH", path, body);
export const del =<T>(path: string) => api<T>("DELETE", path);

/** Envia um arquivo no campo `file` (multipart). */
export function upload<T>(path: string, file: File) {
  const form = new FormData();
  form.append("file", file);
  return api<T>("PUT", path, form);
}

/** Não tenta de novo em erros de cliente (401/403/404…). */
export function retryUnlessClientError(failureCount: number, error: unknown) {
  if (error instanceof ApiError && error.status < 500) return false;
  return failureCount < 2;
}
