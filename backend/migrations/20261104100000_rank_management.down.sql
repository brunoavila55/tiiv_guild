DROP INDEX guild_ranks_name_key;

ALTER TABLE guild_ranks
    DROP CONSTRAINT guild_ranks_guild_id_position_key,
    ADD CONSTRAINT guild_ranks_guild_id_position_key UNIQUE (guild_id, position);
