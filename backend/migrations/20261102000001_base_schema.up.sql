-- Schema base (P0.2): tabelas globais, tabelas de tenant e Row Level Security.
--
-- Papéis:
--   * o usuário que roda as migrations é o dono das tabelas;
--   * a API executa as queries como `guildos_app` (sem ownership, sem BYPASSRLS),
--     então as policies sempre se aplicam a ela.
-- Tenant: cada transação da API faz `SET LOCAL app.guild_id = '<uuid>'`.

-- ---------------------------------------------------------------------------
-- Papel da aplicação
-- ---------------------------------------------------------------------------

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'guildos_app') THEN
        CREATE ROLE guildos_app NOLOGIN NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE;
    END IF;
    -- O dono precisa poder fazer `SET ROLE guildos_app`.
    IF NOT pg_has_role(current_user, 'guildos_app', 'MEMBER') THEN
        EXECUTE format('GRANT guildos_app TO %I', current_user);
    END IF;
END
$$;

-- Tenant da transação atual; NULL quando não definido (nenhuma linha de tenant visível).
CREATE FUNCTION app_current_guild_id() RETURNS uuid
    LANGUAGE sql STABLE
    AS $$ SELECT NULLIF(current_setting('app.guild_id', true), '')::uuid $$;

-- ---------------------------------------------------------------------------
-- Tabelas globais (sem guild_id)
-- ---------------------------------------------------------------------------

CREATE TABLE users (
    id            uuid PRIMARY KEY,
    email         text NOT NULL,
    password_hash text,
    display_name  text NOT NULL,
    avatar_url    text,
    created_at    timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX users_email_key ON users (lower(email));

CREATE TABLE plans (
    id         uuid PRIMARY KEY,
    slug       text NOT NULL UNIQUE,
    name       text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE games (
    id     uuid PRIMARY KEY,
    slug   text NOT NULL UNIQUE,
    name   text NOT NULL,
    active boolean NOT NULL DEFAULT true
);

CREATE TABLE game_classes (
    id        uuid PRIMARY KEY,
    game_id   uuid NOT NULL REFERENCES games (id) ON DELETE CASCADE,
    slug      text NOT NULL,
    name      text NOT NULL,
    role_hint text NOT NULL CHECK (role_hint IN ('tank', 'healer', 'dps', 'support')),
    metadata  jsonb NOT NULL DEFAULT '{}',
    UNIQUE (game_id, slug),
    UNIQUE (game_id, id)
);

CREATE TABLE game_servers (
    id      uuid PRIMARY KEY,
    game_id uuid NOT NULL REFERENCES games (id) ON DELETE CASCADE,
    slug    text NOT NULL,
    name    text NOT NULL,
    region  text,
    UNIQUE (game_id, slug),
    UNIQUE (game_id, id)
);

-- ---------------------------------------------------------------------------
-- Tabelas de tenant
-- FKs compostas (guild_id, id) impedem referência a linhas de outra guilda.
-- ---------------------------------------------------------------------------

CREATE TABLE guilds (
    id            uuid PRIMARY KEY,
    game_id       uuid NOT NULL REFERENCES games (id),
    server_id     uuid,
    slug          text NOT NULL UNIQUE CHECK (slug ~ '^[a-z0-9](?:[a-z0-9-]{0,38}[a-z0-9])?$'),
    name          text NOT NULL,
    tag           text,
    faction       text,
    owner_user_id uuid NOT NULL REFERENCES users (id),
    plan_id       uuid REFERENCES plans (id),
    created_at    timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (game_id, server_id) REFERENCES game_servers (game_id, id)
);

CREATE TABLE guild_branding (
    guild_id            uuid PRIMARY KEY REFERENCES guilds (id) ON DELETE CASCADE,
    logo_url            text,
    banner_url          text,
    primary_color       text,
    secondary_color     text,
    theme               text,
    motto               text,
    description_md      text,
    public_page_enabled boolean NOT NULL DEFAULT false,
    socials             jsonb NOT NULL DEFAULT '{}'
);

CREATE TABLE guild_ranks (
    id          uuid PRIMARY KEY,
    guild_id    uuid NOT NULL REFERENCES guilds (id) ON DELETE CASCADE,
    name        text NOT NULL,
    position    integer NOT NULL,
    permissions text[] NOT NULL DEFAULT '{}',
    UNIQUE (guild_id, id),
    UNIQUE (guild_id, name)
);

CREATE TABLE guild_members (
    id        uuid PRIMARY KEY,
    guild_id  uuid NOT NULL REFERENCES guilds (id) ON DELETE CASCADE,
    user_id   uuid REFERENCES users (id),
    nickname  text NOT NULL,
    rank_id   uuid NOT NULL,
    status    text NOT NULL DEFAULT 'active'
              CHECK (status IN ('active', 'trial', 'inactive', 'left', 'banned')),
    joined_at timestamptz NOT NULL DEFAULT now(),
    left_at   timestamptz,
    notes     text,
    UNIQUE (guild_id, id),
    FOREIGN KEY (guild_id, rank_id) REFERENCES guild_ranks (guild_id, id)
);
-- Uma conta vincula-se a no máximo um membro por guilda.
CREATE UNIQUE INDEX guild_members_user_key ON guild_members (guild_id, user_id) WHERE user_id IS NOT NULL;
CREATE INDEX guild_members_rank_idx ON guild_members (guild_id, rank_id);

CREATE TABLE characters (
    id            uuid PRIMARY KEY,
    guild_id      uuid NOT NULL REFERENCES guilds (id) ON DELETE CASCADE,
    member_id     uuid NOT NULL,
    game_class_id uuid NOT NULL REFERENCES game_classes (id),
    name          text NOT NULL,
    level         integer NOT NULL,
    is_main       boolean NOT NULL DEFAULT false,
    attributes    jsonb NOT NULL DEFAULT '{}',
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (guild_id, member_id) REFERENCES guild_members (guild_id, id) ON DELETE CASCADE
);
-- No máximo um main por membro (o "pelo menos um" é regra de domínio).
CREATE UNIQUE INDEX characters_one_main_key ON characters (member_id) WHERE is_main;
CREATE INDEX characters_member_idx ON characters (guild_id, member_id);

CREATE TABLE member_history (
    id              uuid PRIMARY KEY,
    guild_id        uuid NOT NULL REFERENCES guilds (id) ON DELETE CASCADE,
    member_id       uuid NOT NULL,
    actor_member_id uuid,
    action          text NOT NULL,
    payload         jsonb NOT NULL DEFAULT '{}',
    at              timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (guild_id, member_id) REFERENCES guild_members (guild_id, id),
    FOREIGN KEY (guild_id, actor_member_id) REFERENCES guild_members (guild_id, id)
);
CREATE INDEX member_history_member_idx ON member_history (guild_id, member_id, at);

-- ---------------------------------------------------------------------------
-- Row Level Security
-- ---------------------------------------------------------------------------

ALTER TABLE guilds ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON guilds
    USING (id = app_current_guild_id())
    WITH CHECK (id = app_current_guild_id());

DO $$
DECLARE
    t text;
BEGIN
    FOREACH t IN ARRAY ARRAY['guild_branding', 'guild_ranks', 'guild_members', 'characters', 'member_history']
    LOOP
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format(
            'CREATE POLICY tenant_isolation ON %I '
            'USING (guild_id = app_current_guild_id()) '
            'WITH CHECK (guild_id = app_current_guild_id())',
            t
        );
    END LOOP;
END
$$;

-- ---------------------------------------------------------------------------
-- Privilégios do papel da aplicação
-- ---------------------------------------------------------------------------

GRANT USAGE ON SCHEMA public TO guildos_app;
GRANT EXECUTE ON FUNCTION app_current_guild_id() TO guildos_app;

-- Catálogos são populados pelos seeds (como dono); a API só lê.
GRANT SELECT ON games, game_classes, game_servers, plans TO guildos_app;
GRANT SELECT, INSERT, UPDATE ON users TO guildos_app;

GRANT SELECT, INSERT, UPDATE, DELETE
    ON guilds, guild_branding, guild_ranks, guild_members, characters
    TO guildos_app;

-- Histórico é append-only para a aplicação.
GRANT SELECT, INSERT ON member_history TO guildos_app;
