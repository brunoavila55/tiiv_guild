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

export interface Member {
  id: string;
  nickname: string;
  has_account: boolean;
  rank_id: string;
  rank_name: string;
  status: MemberStatus;
  joined_at: string;
}

export interface AdminGuild {
  slug: string;
  name: string;
  tag: string | null;
  owner_email: string;
  created_at: string;
}
