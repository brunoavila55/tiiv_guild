Migrations do `sqlx migrate` (formato reversível: `<versão>_<nome>.up.sql` / `.down.sql`).

Criar: `sqlx migrate add -r <nome>` dentro de `backend/`. O schema base + RLS entra no P0.2.
