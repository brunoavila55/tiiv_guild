-- Catálogo de jogos gerenciado pelo superadmin (em vez de seeds no código).
-- Tudo que é específico de jogo vira dado: faixa de nível, campos de personagem
-- (character_schema) e ranks padrão por jogo; classes/servidores ganham `active`
-- (não se apaga o que pode estar referenciado) e servidores podem restringir classes.

ALTER TABLE games
    ADD CONSTRAINT games_slug_check CHECK (slug ~ '^[a-z0-9](?:[a-z0-9-]{0,38}[a-z0-9])?$'),
    ADD COLUMN level_min integer NOT NULL DEFAULT 1,
    ADD COLUMN level_max integer NOT NULL DEFAULT 100,
    ADD COLUMN character_schema jsonb NOT NULL
        DEFAULT '{"type": "object", "additionalProperties": false, "properties": {}}',
    -- Ranks criados em toda guilda nova do jogo. Padrão genérico do core.
    ADD COLUMN default_ranks jsonb NOT NULL DEFAULT '[
        {"name": "Líder", "position": 0, "permissions": ["guild.branding.edit", "members.view", "members.manage", "members.kick", "ranks.manage", "characters.manage_own", "characters.manage_any"]},
        {"name": "Membro", "position": 1, "permissions": ["members.view", "characters.manage_own"]}
    ]',
    ADD COLUMN created_at timestamptz NOT NULL DEFAULT now(),
    ADD CONSTRAINT games_level_range_check CHECK (level_min >= 1 AND level_min <= level_max);

ALTER TABLE game_classes
    ADD CONSTRAINT game_classes_slug_check CHECK (slug ~ '^[a-z0-9](?:[a-z0-9_-]{0,38}[a-z0-9])?$'),
    ADD COLUMN active boolean NOT NULL DEFAULT true;

ALTER TABLE game_servers
    ADD CONSTRAINT game_servers_slug_check CHECK (slug ~ '^[a-z0-9](?:[a-z0-9_-]{0,38}[a-z0-9])?$'),
    ADD COLUMN active boolean NOT NULL DEFAULT true;

-- Classes disponíveis em um servidor (versões diferentes do jogo).
-- Servidor sem linhas aqui = todas as classes ativas do jogo.
CREATE TABLE game_server_classes (
    game_id   uuid NOT NULL,
    server_id uuid NOT NULL,
    class_id  uuid NOT NULL,
    PRIMARY KEY (server_id, class_id),
    FOREIGN KEY (game_id, server_id) REFERENCES game_servers (game_id, id) ON DELETE CASCADE,
    FOREIGN KEY (game_id, class_id) REFERENCES game_classes (game_id, id) ON DELETE CASCADE
);

-- O superadmin edita o catálogo pela API (checagem de papel na aplicação).
GRANT INSERT, UPDATE ON games, game_classes, game_servers TO guildos_app;
GRANT SELECT, INSERT, DELETE ON game_server_classes TO guildos_app;
