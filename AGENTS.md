# AGENTS.md — GuildOS (nome provisório)

SaaS **ERP para guildas e comunidades de jogo**, multi-tenant, com **um subdomínio por guilda** (`{slug}.tiiv.com.br`).

O core **não conhece nenhum jogo**. Não existe tabela, coluna, seed, enum ou texto de jogo no core. Tudo que é de jogo (catálogo de jogos, classes, servidores, personagens com atributos de jogo) é um **módulo opcional** que a guilda ativa — e mesmo esse módulo é genérico: os jogos são dados cadastrados pelo superadmin, nunca código.

> Leia este arquivo inteiro antes de qualquer tarefa. Em caso de conflito entre este arquivo e um pedido pontual, pergunte.

---

## 1. Produto

- **Guilda = tenant.** Cada guilda tem um `slug` único que vira o subdomínio: `acme.tiiv.com.br`.
- **Usuário (conta) ≠ membro (vínculo com uma guilda).** Um usuário pode ser membro de várias guildas. Um membro pode existir sem conta (`user_id NULL`), para o líder cadastrar quem não usa o sistema.
- **Core** (vale para qualquer guilda, de qualquer jogo ou sem jogo): identidade visual, página pública, membros, ranks/permissões, histórico, campos customizados definidos pela própria guilda, convites.
- **Módulos** (opcionais, ativados por guilda): `games` (catálogo de jogos + personagens), e no futuro `events`, `raids`, `dkp`, `bank`, `recruitment`, `discord`…

## 2. Domínios e hosts

`BASE_DOMAIN` (config) = `tiiv.com.br` em produção, `guildos.localhost` em dev.

| Host | O quê |
|---|---|
| `BASE_DOMAIN`, `www.BASE_DOMAIN` | Landing page |
| `app.BASE_DOMAIN` | Conta: login, cadastro, "minhas guildas", criar guilda |
| `admin.BASE_DOMAIN` | Painel da plataforma (superadmin) |
| `{slug}.BASE_DOMAIN` | A guilda: página pública em `/`, painel em `/painel` |

- A API responde em **todos** os hosts no prefixo `/api`. O **host define o contexto**: rotas de tenant só existem em host de guilda; rotas `/api/admin/*` só em `admin.`.
- Subdomínios reservados (nunca viram slug): ver `domain::host::RESERVED_SUBDOMAINS`.
- Sessão: um único cookie com `Domain=COOKIE_DOMAIN` (o `BASE_DOMAIN`), então o login feito em `app.` vale em toda guilda.
- CSRF: requests que alteram estado com header `Origin` fora de `BASE_DOMAIN`/subdomínios → 403. Tenants nunca hospedam HTML/JS próprio (só imagens), porque todos os subdomínios são *same-site*.
- Domínio próprio por guilda (CNAME) é futuro; o resolvedor de host é o único ponto a mudar.

## 3. Stack

| Camada | Tecnologia |
|---|---|
| Backend | Rust stable, **Axum**, **Tokio**, **SQLx** (queries checadas em compile-time), serde, tracing, thiserror |
| Banco | PostgreSQL 16+, migrations `sqlx migrate` (aplicadas pela API ao subir) |
| Auth | Sessão em cookie HttpOnly + tabela `sessions` (token aleatório, só o SHA-256 no banco); senha com argon2. Discord OAuth planejado |
| Frontend | React + TypeScript + Vite, TanStack Router, TanStack Query, Tailwind v4, react-hook-form + zod |
| Storage | `object_store`: `STORAGE=local` (disco, servido pela API em `/api/media/…`; padrão em dev e no VPS) ou `STORAGE=s3` (S3/MinIO com bucket público) |
| Infra | Docker, Traefik (TLS wildcard `*.tiiv.com.br`), GitHub Actions → GHCR → VPS. **Não usar Supabase/BaaS.** |
| Contrato API | Tipos TS escritos à mão em `frontend/src/lib/api/types.ts` por enquanto; OpenAPI (`utoipa`) + client gerado quando a API estabilizar |

## 4. Repositório

```
/
├── AGENTS.md
├── docker-compose.yml          # dev: postgres, minio
├── backend/
│   ├── Cargo.toml              # workspace
│   ├── migrations/
│   └── crates/
│       ├── domain/             # regras puras, sem IO: host/slug, permissões, erros, validações
│       ├── db/                 # pool, transação de tenant, repositórios SQLx
│       └── api/                # binário Axum: config, extractors, features/<feature>.rs
├── frontend/src/
│   ├── app/                    # host → árvore de rotas, providers, textos (i18n)
│   ├── features/<feature>/     # landing, auth, guilds, guild-public, guild-panel, members, admin
│   ├── components/ui/          # componentes base
│   └── lib/                    # api client, utils
└── deploy/                     # compose de produção + script de deploy do VPS
```

Regra: **uma feature = um módulo** em `api/src/features/` e uma pasta em `frontend/src/features/`. Um módulo opcional novo entra sem tocar no core.

## 5. Multi-tenancy

- Banco e schema compartilhados, coluna **`guild_id`** em toda tabela de tenant.
- **Row Level Security** em toda tabela de tenant: `USING / WITH CHECK (guild_id = app_current_guild_id())`.
- A API conecta como dono do schema só para migrar; o pool de queries faz `SET ROLE guildos_app` (sem ownership, sem `BYPASSRLS`) em toda conexão.
- Request de tenant: `TenantCtx` resolve **host → guilda** (404), sessão (401), membro ativo (403) e permissões; toda query de tenant roda em `db::tenant_tx(pool, guild_id)`, que abre transação e faz `set_config('app.guild_id', …, true)`.
- Tabelas globais (sem `guild_id`, sem RLS): `users`, `sessions`, `guilds` (o registro de tenants: slug → id).
- Leituras cruzando tenants (ex.: "minhas guildas") só por função `SECURITY DEFINER` estreita e revisada (`user_guilds`).
- **Nunca** query de tenant fora de `tenant_tx`. Testes provam isolamento A/B por API e por SQL direto.

## 6. Modelo de dados

```text
-- global
users(id, email UNIQUE lower, password_hash, display_name, avatar_url, created_at)
sessions(id, user_id, token_hash UNIQUE, created_at, expires_at)
guilds(id, slug UNIQUE, name, tag, owner_user_id, created_at)

-- tenant (RLS)
guild_branding(guild_id PK, logo_url, banner_url, primary_color, secondary_color,
               motto, description_md, public_page_enabled, socials jsonb)
guild_ranks(id, guild_id, name, position, permissions text[])        -- menor position = topo
guild_members(id, guild_id, user_id NULL, nickname, rank_id, status,  -- active|trial|inactive|left|banned
              fields jsonb, notes, joined_at, left_at)                -- fields = campos customizados da guilda
member_history(id, guild_id, member_id, actor_member_id, action, payload jsonb, at)

-- planejado (NÃO criar antes da etapa do roadmap)
guild_settings(guild_id PK, member_fields jsonb)        -- schema dos campos customizados (P1)
guild_modules(guild_id, module, enabled, settings jsonb) -- módulos ativos (P1)
games / game_classes / game_servers / characters         -- módulo games (P2)
activities / activity_participants                       -- eventos, raids… (P3)
```

Regras:
- IDs UUID v7, gerados na aplicação. Timestamps `timestamptz`.
- Soft delete só onde há histórico (membros: `status = left`, `left_at`).
- Campos que variam por guilda/jogo vão em `jsonb` validado por schema (subconjunto de JSON Schema: `string`/`integer`/`number`/`boolean`, `enum`, `minimum`/`maximum`, `minLength`/`maxLength`, `required`, `title`). Nunca coluna dedicada.
- Ranks padrão de guilda nova são genéricos da plataforma (`domain::ranks::default_ranks`): Líder, Oficial, Membro, Recruta.

## 7. Permissões

- Strings granulares por rank: `guild.settings.edit`, `guild.branding.edit`, `members.view`, `members.manage`, `members.kick`, `ranks.manage`, `modules.manage`.
- Módulos acrescentam as suas (`games.manage`, `events.manage`…) sem refatorar o core.
- Dono da guilda tem todas. Checagem sempre no backend: `ctx.require(Permission::X)?`.
- **Superadmin** = e-mails em `SUPERADMIN_EMAILS`; é papel da plataforma (`admin.`), **não** dá acesso a dados de tenant.

## 8. Convenções

**Rust**
- `cargo fmt` e `cargo clippy --workspace --all-targets -- -D warnings` passam.
- Sem `unwrap()`/`expect()` fora de testes e startup.
- Erros: `domain::Error` (thiserror) → `ApiError` → JSON `{ "code", "message", "fields"? }` com status coerente.
- Handlers finos: extractor/validação → regra de domínio → repositório.
- Queries com `sqlx::query!`/`query_as!`; após mudar query rode `cargo sqlx prepare --workspace` e comite `.sqlx/` (CI usa `SQLX_OFFLINE=true`).

**Frontend**
- TypeScript strict, sem `any`.
- Dados do servidor só via TanStack Query.
- Cores da guilda via CSS variables `--guild-primary` / `--guild-secondary` no layout do tenant.
- UI em **pt-BR**, textos em `src/app/i18n.ts`.
- Host → contexto em `src/app/host.ts` (espelho de `domain::host`).

**Geral**
- Conventional Commits. Migrations reversíveis (`.up.sql`/`.down.sql`).
- Dependência nova só com justificativa no PR. Nenhum segredo no repositório.

## 9. Comandos

```bash
docker compose up -d postgres            # ou Postgres 16 nativo (ver README)
cd backend && cargo run -p api           # API em :8080, migra ao subir
cargo test --workspace                   # sqlx::test: um banco por teste
cd frontend && pnpm dev                  # http://app.guildos.localhost:5173
```

## 10. Testes

- Backend: integração com Postgres real (`#[sqlx::test]`).
- **Obrigatório:** isolamento — guilda A nunca lê/escreve dado da guilda B (por API, via host, e por SQL com RLS).
- Frontend: Vitest para lógica (host, validações); Playwright nos fluxos críticos a partir do P1.

---

## 11. Roadmap

### P0 — Fundação
1. Workspace, docker-compose, CI (fmt, clippy, test, build web), deploy.
2. Schema base + RLS + papel `guildos_app`. Aceite: sem `app.guild_id` nenhuma linha de tenant; isolamento A/B.
3. Auth: registro, login, logout, `/api/me`, cookie compartilhado entre subdomínios. Aceite: argon2; HttpOnly+Secure+SameSite=Lax; 401 sem sessão.
4. Resolução de tenant por subdomínio + criação de guilda. Aceite: slug validado/reservado/único; criador vira dono com ranks padrão; não-membro → 403; host inexistente → 404.
5. Painel mínimo da guilda (dados da guilda, lista de membros) e painel do superadmin (lista de guildas).

### P1 — MVP
1. Visual da guilda: logo/banner (upload S3, ≤2 MB, png/jpg/webp), cores, lema, descrição markdown, redes; preview.
2. Página pública (`{slug}.BASE_DOMAIN/`): branding + membros ativos; desligada → 404; nunca expõe e-mail/notas.
3. Gestão de membros: filtros, busca, paginação, adicionar (com/sem conta), editar rank/status/notas, remover (soft); tudo gera `member_history`.
4. Campos customizados de membro definidos pela guilda (`guild_settings.member_fields`); 422 com campo inválido.
5. Ranks: CRUD, ordenação, permissões; dono nunca perde gestão; rank em uso não é removido sem realocar.
6. Convites com expiração e N usos para vincular conta a membro.
7. Infra de módulos (`guild_modules`, ativar/desativar por guilda, `modules.manage`).

### P2 — Módulo `games`
1. Catálogo pelo superadmin: jogos, classes, servidores (e classes por servidor), `character_schema`.
2. Guilda ativa o módulo e escolhe jogo/servidor; personagens (main/alts) por membro com formulário a partir do schema.
   Aceite: segundo jogo entra só por cadastro, zero código.

### P3 — Atividades e expansão
Eventos/agenda, raids e dungeons, presença/DKP, loot, Discord (login + webhook), domínio próprio por guilda, planos/billing, métricas.

## 12. O que NÃO fazer

- Nada de jogo no core: sem colunas, enums, seeds ou textos de um jogo específico.
- Não implementar módulo antes da etapa dele no roadmap.
- Não acessar dados de tenant fora de `TenantCtx`/`tenant_tx`.
- Não usar Supabase nem BaaS.
- Não guardar segredos no repositório (`.env.example` só com placeholders).
