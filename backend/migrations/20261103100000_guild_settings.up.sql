-- Configurações da guilda. `member_fields` é o schema (subconjunto de JSON Schema) dos
-- campos customizados de membro, cujos valores ficam em `guild_members.fields`.
-- Sem linha = schema vazio; a API cria a linha ao salvar.
CREATE TABLE guild_settings (
    guild_id      uuid PRIMARY KEY REFERENCES guilds (id) ON DELETE CASCADE,
    member_fields jsonb NOT NULL DEFAULT '{"type": "object", "properties": {}}'
);

ALTER TABLE guild_settings ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON guild_settings
    USING (guild_id = app_current_guild_id())
    WITH CHECK (guild_id = app_current_guild_id());

GRANT SELECT, INSERT, UPDATE ON guild_settings TO guildos_app;
