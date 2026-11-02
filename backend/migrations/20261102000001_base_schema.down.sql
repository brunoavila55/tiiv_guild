-- Reverte o schema base. O papel `guildos_app` é do cluster (compartilhado entre bancos,
-- inclusive os de teste), então não é removido aqui.

DROP TABLE IF EXISTS member_history;
DROP TABLE IF EXISTS characters;
DROP TABLE IF EXISTS guild_members;
DROP TABLE IF EXISTS guild_ranks;
DROP TABLE IF EXISTS guild_branding;
DROP TABLE IF EXISTS guilds;
DROP TABLE IF EXISTS game_servers;
DROP TABLE IF EXISTS game_classes;
DROP TABLE IF EXISTS games;
DROP TABLE IF EXISTS plans;
DROP TABLE IF EXISTS users;

DROP FUNCTION IF EXISTS app_current_guild_id();

REVOKE USAGE ON SCHEMA public FROM guildos_app;
