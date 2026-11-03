-- Gestão de ranks (P1.5).

-- Reordenar troca posições entre linhas num único UPDATE: a unicidade da posição
-- passa a ser checada no fim do comando, não linha a linha.
ALTER TABLE guild_ranks
    DROP CONSTRAINT guild_ranks_guild_id_position_key,
    ADD CONSTRAINT guild_ranks_guild_id_position_key
        UNIQUE (guild_id, position) DEFERRABLE INITIALLY IMMEDIATE;

-- Nome de rank único na guilda, sem diferenciar maiúsculas.
CREATE UNIQUE INDEX guild_ranks_name_key ON guild_ranks (guild_id, lower(name));
