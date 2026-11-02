// Cliente HTTP base. O client tipado será gerado em `schema.d.ts` via `pnpm gen:api`.

export type ApiErrorBody = { code: string; message: string };

export class ApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly body: ApiErrorBody,
  ) {
    super(body.message);
  }
}

export async function apiFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const headers = new Headers(init?.headers);
  if (!headers.has("Content-Type")) headers.set("Content-Type", "application/json");

  const res = await fetch(`/api${path}`, { credentials: "include", ...init, headers });
  if (!res.ok) {
    const body = (await res.json().catch(() => ({
      code: "unknown",
      message: res.statusText,
    }))) as ApiErrorBody;
    throw new ApiError(res.status, body);
  }
  return (await res.json()) as T;
}
