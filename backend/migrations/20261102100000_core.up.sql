-- Core multi-tenant: registro global (usuários, sessões, guildas) e tabelas de tenant com RLS.
--
-- Quem roda as migrations é o dono das tabelas. A API executa as queries como
-- `guildos_app` (sem ownership, sem BYPASSRLS), então as policies sempre valem para ela.
-- Tenant da transação: `set_config('app.guild_id', '<uuid>', true)`.

-- ---------------------------------------------------------------------------
-- Papel da aplicação (do cluster: bancos migrando em paralelo disputam a criação)
-- ---------------------------------------------------------------------------
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'guildos_app') THEN
        BEGIN
            CREATE ROLE guildos_app NOLOGIN NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE;
        EXCEPTION
            WHEN duplicate_object OR unique_violation THEN NULL;
        END;
    END IF;
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
-- Global
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

CREATE TABLE sessions (
    id         uuid PRIMARY KEY,
    user_id    uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    token_hash bytea NOT NULL UNIQUE,
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL
);
CREATE INDEX sessions_user_id_idx ON sessions (user_id);

-- Registro de tenants: resolve host → guilda antes de existir contexto de tenant.
CREATE TABLE guilds (
    id            uuid PRIMARY KEY,
    slug          text NOT NULL UNIQUE CHECK (slug ~ '^[a-z0-9](-?[a-z0-9])+$'),
    name          text NOT NULL,
    tag           text,
    owner_user_id uuid NOT NULL REFERENCES users (id),
    created_at    timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX guilds_owner_user_id_idx ON guilds (owner_user_id);

-- ---------------------------------------------------------------------------
-- Tenant
-- ---------------------------------------------------------------------------
CREATE TABLE guild_branding (
    guild_id            uuid PRIMARY KEY REFERENCES guilds (id) ON DELETE CASCADE,
    logo_url            text,
    banner_url          text,
    primary_color       text,
    secondary_color     text,
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
    UNIQUE (guild_id, position)
);

CREATE TYPE member_status AS ENUM ('active', 'trial', 'inactive', 'left', 'banned');

CREATE TABLE guild_members (
    id        uuid PRIMARY KEY,
    guild_id  uuid NOT NULL REFERENCES guilds (id) ON DELETE CASCADE,
    user_id   uuid REFERENCES users (id),
    nickname  text NOT NULL,
    rank_id   uuid NOT NULL,
    status    member_status NOT NULL DEFAULT 'active',
    fields    jsonb NOT NULL DEFAULT '{}',
    notes     text,
    joined_at timestamptz NOT NULL DEFAULT now(),
    left_at   timestamptz,
    UNIQUE (guild_id, id),
    -- rank precisa ser da mesma guilda
    FOREIGN KEY (guild_id, rank_id) REFERENCES guild_ranks (guild_id, id)
);
-- uma conta vira no máximo um membro por guilda
CREATE UNIQUE INDEX guild_members_user_key ON guild_members (guild_id, user_id) WHERE user_id IS NOT NULL;
CREATE INDEX guild_members_user_id_idx ON guild_members (user_id) WHERE user_id IS NOT NULL;

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
CREATE INDEX member_history_member_idx ON member_history (guild_id, member_id, at DESC);

-- ---------------------------------------------------------------------------
-- RLS
-- ---------------------------------------------------------------------------
DO $$
DECLARE
    t text;
BEGIN
    FOREACH t IN ARRAY ARRAY['guild_branding', 'guild_ranks', 'guild_members', 'member_history'] LOOP
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format(
            'CREATE POLICY tenant_isolation ON %I USING (guild_id = app_current_guild_id()) '
            'WITH CHECK (guild_id = app_current_guild_id())', t);
    END LOOP;
END
$$;

-- Guildas ativas de um usuário, cruzando tenants. Única leitura que ignora o RLS:
-- devolve só o necessário para a tela "minhas guildas".
CREATE FUNCTION user_guilds(p_user_id uuid)
    RETURNS TABLE (guild_id uuid, slug text, name text, tag text, is_owner boolean, rank_name text)
    LANGUAGE sql STABLE SECURITY DEFINER
    SET search_path = public, pg_temp
    AS $$
        SELECT g.id, g.slug, g.name, g.tag, g.owner_user_id = p_user_id, r.name
          FROM guild_members m
          JOIN guilds g ON g.id = m.guild_id
          JOIN guild_ranks r ON r.id = m.rank_id
         WHERE m.user_id = p_user_id
           AND m.status IN ('active', 'trial')
         ORDER BY g.name
    $$;
REVOKE ALL ON FUNCTION user_guilds(uuid) FROM PUBLIC;

-- ---------------------------------------------------------------------------
-- Grants do papel da aplicação
-- ---------------------------------------------------------------------------
GRANT USAGE ON SCHEMA public TO guildos_app;
GRANT USAGE ON TYPE member_status TO guildos_app;
GRANT SELECT, INSERT, UPDATE ON users TO guildos_app;
GRANT SELECT, INSERT, DELETE ON sessions TO guildos_app;
GRANT SELECT, INSERT, UPDATE ON guilds TO guildos_app;
GRANT SELECT, INSERT, UPDATE ON guild_branding, guild_members TO guildos_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON guild_ranks TO guildos_app;
GRANT SELECT, INSERT ON member_history TO guildos_app;
GRANT EXECUTE ON FUNCTION app_current_guild_id() TO guildos_app;
GRANT EXECUTE ON FUNCTION user_guilds(uuid) TO guildos_app;
