-- O papel `guildos_app` é do cluster e pode servir a outros bancos: não é removido aqui.
DROP FUNCTION IF EXISTS user_guilds(uuid);
DROP TABLE IF EXISTS member_history;
DROP TABLE IF EXISTS guild_members;
DROP TYPE IF EXISTS member_status;
DROP TABLE IF EXISTS guild_ranks;
DROP TABLE IF EXISTS guild_branding;
DROP TABLE IF EXISTS guilds;
DROP TABLE IF EXISTS sessions;
DROP TABLE IF EXISTS users;
DROP FUNCTION IF EXISTS app_current_guild_id();
