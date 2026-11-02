// Host → contexto. Espelho de `backend/crates/domain/src/host.rs`.

export const BASE_DOMAIN: string = import.meta.env.VITE_BASE_DOMAIN ?? "guildos.localhost";

export const RESERVED_SUBDOMAINS = [
  "admin", "api", "app", "assets", "auth", "blog", "cdn", "dev", "docs", "help", "id", "login",
  "mail", "media", "staging", "static", "status", "suporte", "support", "test", "www",
] as const;

export type HostKind =
  | { kind: "root" }
  | { kind: "app" }
  | { kind: "admin" }
  | { kind: "tenant"; slug: string }
  | { kind: "unknown" };

export function classifyHost(hostname: string, base: string = BASE_DOMAIN): HostKind {
  const host = hostname.replace(/:\d+$/, "").replace(/\.$/, "").toLowerCase();
  const root = base.toLowerCase();
  if (host === root) return { kind: "root" };
  if (!host.endsWith(`.${root}`)) return { kind: "unknown" };
  const sub = host.slice(0, -(root.length + 1));
  if (sub === "www") return { kind: "root" };
  if (sub === "app") return { kind: "app" };
  if (sub === "admin") return { kind: "admin" };
  return slugError(sub) === null ? { kind: "tenant", slug: sub } : { kind: "unknown" };
}

/** Mensagem de erro do slug, ou `null` se válido. */
export function slugError(slug: string): string | null {
  if (slug.length < 3 || slug.length > 32) return "deve ter entre 3 e 32 caracteres";
  if (!/^[a-z0-9-]+$/.test(slug)) return "use apenas letras minúsculas, números e hífen";
  if (slug.startsWith("-") || slug.endsWith("-") || slug.includes("--"))
    return "hífen não pode estar no início, no fim nem repetido";
  if ((RESERVED_SUBDOMAINS as readonly string[]).includes(slug)) return "endereço reservado";
  return null;
}

/** Sugere um slug a partir do nome da guilda. */
export function slugify(name: string): string {
  return name
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 32)
    .replace(/-+$/, "");
}

function hostUrl(sub: string | null, path: string): string {
  const { protocol, port } = window.location;
  const host = sub ? `${sub}.${BASE_DOMAIN}` : BASE_DOMAIN;
  return `${protocol}//${host}${port ? `:${port}` : ""}${path}`;
}

export const appUrl = (path = "/") => hostUrl("app", path);
export const adminUrl = (path = "/") => hostUrl("admin", path);
export const guildUrl = (slug: string, path = "/") => hostUrl(slug, path);
export const rootUrl = (path = "/") => hostUrl(null, path);

/** Só aceita redirecionar para o próprio domínio base (evita open redirect). */
export function safeNext(next: string | undefined): string | null {
  if (!next) return null;
  try {
    const url = new URL(next);
    if (url.protocol !== "https:" && url.protocol !== "http:") return null;
    return classifyHost(url.hostname).kind === "unknown" ? null : url.toString();
  } catch {
    return null;
  }
}
