REVOKE INSERT, UPDATE ON games, game_classes, game_servers FROM guildos_app;
DROP TABLE IF EXISTS game_server_classes;

ALTER TABLE game_servers DROP COLUMN active, DROP CONSTRAINT game_servers_slug_check;
ALTER TABLE game_classes DROP COLUMN active, DROP CONSTRAINT game_classes_slug_check;
ALTER TABLE games
    DROP CONSTRAINT games_level_range_check,
    DROP COLUMN created_at,
    DROP COLUMN default_ranks,
    DROP COLUMN character_schema,
    DROP COLUMN level_max,
    DROP COLUMN level_min,
    DROP CONSTRAINT games_slug_check;
