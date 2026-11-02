# GuildOS

ERP SaaS multi-tenant para guildas e comunidades de jogo, com um subdomínio por guilda (`{slug}.tiiv.com.br`). O core não depende de nenhum jogo. Arquitetura e convenções: [AGENTS.md](AGENTS.md).

## Pré-requisitos

Rust stable, Node 26 + pnpm 12, PostgreSQL 16 (Docker ou nativo), `sqlx-cli` (`cargo install sqlx-cli --no-default-features --features rustls,postgres`).

## Rodando local

```bash
cp .env.example .env && cp .env.example backend/.env   # e troque os valores
docker compose up -d postgres
cd backend && cargo run -p api                          # API em :8080, migra ao subir
cd frontend && pnpm install && pnpm dev                 # http://app.guildos.localhost:5173
```

Sem Docker: crie no Postgres nativo a role `guildos` (`LOGIN CREATEDB`, necessário para o `sqlx::test`) e o banco. A migration cria o papel `guildos_app` (usado pelo RLS) se o usuário puder; senão, como superusuário: `CREATE ROLE guildos_app NOLOGIN; GRANT guildos_app TO guildos;`.

### Hosts em dev

`*.localhost` resolve para 127.0.0.1 no Chrome/Firefox sem mexer em DNS:

| URL | O quê |
|---|---|
| http://guildos.localhost:5173 | landing |
| http://app.guildos.localhost:5173 | login, cadastro, minhas guildas, criar guilda |
| http://admin.guildos.localhost:5173 | painel da plataforma (`SUPERADMIN_EMAILS`) |
| http://{slug}.guildos.localhost:5173 | a guilda (`/` público, `/painel`) |

O Vite repassa `/api` para a API preservando o `Host`, que é o que define o tenant.

## Testes

```bash
cd backend && cargo test --workspace     # um banco por teste (sqlx::test)
cd frontend && pnpm test
```

Mudou uma query SQLx? Rode `cargo sqlx prepare --workspace` com o banco migrado e comite `backend/.sqlx/`.
