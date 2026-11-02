# GuildOS

ERP SaaS multi-tenant para guildas de MMORPG (primeiro jogo: Perfect World). Guia de arquitetura e convenções: [AGENTS.md](AGENTS.md).

## Pré-requisitos

Rust stable, Node 26 + pnpm 12, Docker, `sqlx-cli` (`cargo install sqlx-cli --no-default-features --features rustls,postgres`).

## Rodando local

```bash
cp .env.example .env && cp .env.example backend/.env   # e troque os valores
docker compose up -d postgres minio
cd backend && sqlx migrate run && cargo run -p api      # API em :8080 (GET /api/health)
cd frontend && pnpm install && pnpm dev                 # web em :5173 (proxy /api → :8080)
```

Stack completa atrás do Traefik: `docker compose up --build` → http://localhost:8000.
