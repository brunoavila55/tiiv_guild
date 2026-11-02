-- Sessões de login (P0.3). Tabela global: a sessão pertence ao usuário, não à guilda.
-- Guardamos só o SHA-256 do token; o token em si vive apenas no cookie do navegador.

CREATE TABLE sessions (
    id         uuid PRIMARY KEY,
    user_id    uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    token_hash bytea NOT NULL UNIQUE,
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL,
    user_agent text
);
CREATE INDEX sessions_user_idx ON sessions (user_id);

GRANT SELECT, INSERT, DELETE ON sessions TO guildos_app;
