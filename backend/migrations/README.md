Migrations do `sqlx migrate` (formato reversível: `<versão>_<nome>.up.sql` / `.down.sql`).

Criar: `sqlx migrate add -r <nome>` dentro de `backend/`. A API também aplica as pendentes ao subir.

Tabelas de tenant têm `guild_id` + RLS (`tenant_isolation`); toda tabela nova de tenant precisa de `ENABLE ROW LEVEL SECURITY`, a policy e os `GRANT`s para `guildos_app`.
