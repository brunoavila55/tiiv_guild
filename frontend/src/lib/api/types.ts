// Contrato da API (espelha os DTOs de `backend/crates/api/src/features`).

export type MemberStatus = "active" | "trial" | "inactive" | "left" | "banned";

export interface User {
  id: string;
  email: string;
  display_name: string;
  avatar_url: string | null;
}

export interface MyGuild {
  slug: string;
  name: string;
  tag: string | null;
  is_owner: boolean;
  rank_name: string;
}

export interface Me {
  user: User;
  is_superadmin: boolean;
  guilds: MyGuild[];
}

export interface SlugCheck {
  available: boolean;
  reason: string | null;
}

export interface Tenant {
  slug: string;
  name: string;
  tag: string | null;
  logo_url: string | null;
  primary_color: string | null;
  secondary_color: string | null;
  public_page_enabled: boolean;
}

export interface Guild {
  slug: string;
  name: string;
  tag: string | null;
  created_at: string;
  me: {
    member_id: string;
    nickname: string;
    rank_name: string;
    is_owner: boolean;
    permissions: string[];
  };
}

export const SOCIAL_KEYS = [
  "discord", "website", "youtube", "twitch", "instagram", "x", "tiktok", "facebook",
] as const;
export type SocialKey = (typeof SOCIAL_KEYS)[number];

export interface Branding {
  logo_url: string | null;
  banner_url: string | null;
  primary_color: string | null;
  secondary_color: string | null;
  motto: string | null;
  description_md: string | null;
  public_page_enabled: boolean;
  socials: Partial<Record<SocialKey, string>>;
}

export type ImageSlot = "logo" | "banner";

/** Página pública da guilda: só o que ela publicou, sem nada privado dos membros. */
export interface PublicPage {
  slug: string;
  name: string;
  tag: string | null;
  created_at: string;
  logo_url: string | null;
  banner_url: string | null;
  primary_color: string | null;
  secondary_color: string | null;
  motto: string | null;
  description_md: string | null;
  socials: Partial<Record<SocialKey, string>>;
  member_count: number;
  ranks: { name: string; members: string[] }[];
}

export interface Member {
  id: string;
  nickname: string;
  has_account: boolean;
  rank_id: string;
  rank_name: string;
  status: MemberStatus;
  joined_at: string;
}

export interface MemberPage {
  items: Member[];
  total: number;
  page: number;
  per_page: number;
}

/** Ficha do membro no painel. `notes` só vem para quem tem `members.manage`. */
export interface MemberDetail {
  id: string;
  nickname: string;
  has_account: boolean;
  account_name: string | null;
  is_owner: boolean;
  rank_id: string;
  rank_name: string;
  status: MemberStatus;
  notes: string | null;
  joined_at: string;
  left_at: string | null;
  can_edit: boolean;
  can_remove: boolean;
}

export interface Rank {
  id: string;
  name: string;
  position: number;
  permissions: string[];
}

/** `payload` de `member.updated`: campo → `{ from, to }` (notas só como `true`). */
export interface HistoryEntry {
  id: string;
  action: string;
  payload: Record<string, unknown>;
  actor_nickname: string | null;
  at: string;
}

export interface AdminGuild {
  slug: string;
  name: string;
  tag: string | null;
  owner_email: string;
  created_at: string;
}
