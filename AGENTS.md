# CLAUDE.md — GuildOS (nome provisório)

SaaS **ERP para guildas de MMORPG**, multi-tenant. Primeiro jogo usado: **Perfect World** (PW), mas o produto **não é um gerenciador de PW**. A arquitetura é **game-agnostic** desde o dia 1: tudo que é específico de jogo é **dado do catálogo**, cadastrado pelo superadmin da plataforma (jogos, classes, servidores, campos de personagem, ranks padrão), nunca hardcoded no core nem em seeds do repositório.

> Leia este arquivo inteiro antes de qualquer tarefa. Em caso de conflito entre este arquivo e um pedido pontual, pergunte.

---

## 1. Visão do produto

- Uma **guilda** = um **tenant**. Um usuário pode pertencer a várias guildas (inclusive de jogos diferentes).
- Escopo atual (MVP):
  1. **Visual da guilda** — identidade (nome, tag, logo, banner, cores, tema), página pública da guilda.
  2. **Gestão de players** — membros, personagens (alts), classes, níveis, cargos/ranks, status, histórico.
- Escopo futuro (a base já precisa suportar, **não implementar agora**): eventos, raids, dungeons, presença/DKP, loot, recrutamento, banco da guilda, notificações (Discord), relatórios.

## 2. Stack

| Camada | Tecnologia |
|---|---|
| Backend | Rust (stable), **Axum**, **Tokio**, **SQLx** (queries checadas em compile-time), **serde**, **tracing** |
| Banco | PostgreSQL 16+, migrations via `sqlx migrate` |
| Auth | Sessão via cookie HttpOnly + tabela `sessions` (argon2 p/ senha). OAuth Discord planejado (P2) |
| Frontend | React + TypeScript + Vite, TanStack Query, TanStack Router, Tailwind, shadcn/ui, react-hook-form + zod |
| Storage de imagens | S3-compatível (MinIO em dev/self-hosted) |
| Infra | Docker + Traefik, CI no GitHub Actions. **Não usar Supabase.** |
| Contrato API | OpenAPI gerado pelo backend (`utoipa`) → client TS gerado (`openapi-typescript`) |

## 3. Estrutura do repositório

```
/
├── CLAUDE.md
├── docker-compose.yml          # postgres, minio, api, web, traefik
├── backend/
│   ├── Cargo.toml              # workspace
│   ├── crates/
│   │   ├── api/                # binário Axum: rotas, extractors, middlewares
│   │   ├── domain/             # entidades, regras, erros de domínio (sem IO)
│   │   ├── db/                 # repositórios SQLx, migrations
│   │   ├── games/              # regras genéricas do catálogo: validação de definição de jogo e character_schema
│   │   └── shared/             # tipos comuns (ids, paginação, erros)
│   └── migrations/
├── frontend/
│   └── src/
│       ├── app/                # rotas, layout, providers
│       ├── features/           # guild-branding/, members/, characters/, admin-catalog/, ... (1 pasta por feature)
│       ├── components/ui/      # shadcn
│       └── lib/api/            # client gerado + hooks TanStack Query
└── deploy/                     # compose de produção + script de deploy do VPS
```

Visual por jogo/classe (ícones, cores) também é dado do catálogo (`metadata` jsonb), não pasta no frontend.

Regra: **uma feature = um módulo** no backend (`api/src/features/<feature>/`) e uma pasta em `frontend/src/features/<feature>/`. Features futuras (events, raids, dungeons) entram como novos módulos sem tocar nos existentes.

## 4. Multi-tenancy

- Estratégia: **banco compartilhado, schema compartilhado, coluna `guild_id`** em toda tabela de tenant.
- Defesa em profundidade com **Row Level Security** no Postgres:
  - Cada request abre transação e executa `SET LOCAL app.guild_id = '<uuid>'`.
  - Policies: `USING (guild_id = current_setting('app.guild_id')::uuid)`.
  - A role da aplicação **não** é owner das tabelas (RLS não é bypassado).
- Resolução do tenant: rota `/api/guilds/{guild_slug}/...` → extractor `TenantCtx { guild_id, member_id, permissions }` que valida que o usuário é membro.
- Tabelas globais (sem `guild_id`): `users`, `sessions`, `games`, `game_classes`, `game_servers`, `game_server_classes`, `plans`.
- **Nunca** escrever query de tenant sem passar pelo `TenantCtx`. Testes devem provar isolamento entre guildas.

## 5. Modelo de dados (base)

```text
users(id, email, password_hash, display_name, avatar_url, created_at)
games(id, slug, name, active, level_min, level_max,
      character_schema jsonb, default_ranks jsonb, created_at)  -- tudo cadastrado pelo superadmin
game_classes(id, game_id, slug, name, role_hint, metadata jsonb, active) -- role_hint: tank|healer|dps|support
game_servers(id, game_id, slug, name, region, active)
game_server_classes(game_id, server_id, class_id)  -- classes de um servidor (versões do jogo); vazio = todas

guilds(id, game_id, server_id, slug UNIQUE, name, tag, faction,
       created_at, owner_user_id, plan_id)
guild_branding(guild_id PK, logo_url, banner_url, primary_color, secondary_color,
               theme, motto, description_md, public_page_enabled, socials jsonb)

guild_ranks(id, guild_id, name, position, permissions text[])   -- ex: Líder, Vice, Diretor, Membro
guild_members(id, guild_id, user_id NULL, nickname, rank_id, status,  -- active|trial|inactive|left|banned
              joined_at, left_at, notes)
characters(id, guild_id, member_id, game_class_id, name, level, is_main,
           attributes jsonb, created_at, updated_at)            -- attributes = campos do character_schema do jogo
member_history(id, guild_id, member_id, actor_member_id, action, payload jsonb, at)

-- Reservado p/ futuro (NÃO criar ainda, só respeitar o design):
-- activities(id, guild_id, kind, ...)  kind: event|raid|dungeon|war  (tabela base + extensões)
-- activity_participants(activity_id, character_id, status, ...)
```

Regras de dados:
- IDs: UUID v7. Timestamps `timestamptz`. Soft delete só onde há histórico (membros).
- Campos específicos de jogo (cultivo, reino, ataque/defesa no PW etc.) vão em `attributes jsonb`, **validados pelo `character_schema` do jogo** — nunca colunas dedicadas no core.
- Itens do catálogo não são apagados (podem estar referenciados): usa-se `active = false`.
- Usuário (conta) ≠ membro (vínculo com guilda) ≠ personagem. Um membro pode existir sem conta (`user_id NULL`) para o líder cadastrar players que não usam o sistema.

## 6. Catálogo de jogos (superadmin)

- O **superadmin da plataforma** cadastra e edita pelo painel (`/api/admin/games/...`): jogos (faixa de nível, `character_schema`, ranks padrão), classes e servidores. Classes e servidores mudam por jogo **e por versão do jogo** (servidores privados), por isso são dados, nunca código.
- Superadmin = e-mails em `SUPERADMIN_EMAILS` (config do servidor). Extractor `SuperAdmin` (401 sem sessão, 403 sem o papel). `/api/me` expõe `is_superadmin`.
- `character_schema`: subconjunto de JSON Schema (`string`/`integer`/`number`/`boolean`, `enum`, `minimum`/`maximum`, `minLength`/`maxLength`, `required`, `additionalProperties: false`; `title`/`x-enumLabels` para a UI), validado ao salvar e usado para validar `attributes` (crate `games`). A ordem das propriedades é a ordem do formulário.
- `default_ranks`: ranks criados em toda guilda nova do jogo; o rank do topo (menor `position`) precisa de todas as permissões.
- Leitura pública: `GET /api/games` e `GET /api/games/{slug}` (só itens ativos).
- Frontend renderiza o formulário de personagem a partir de `character_schema`.
- Adicionar um jogo novo = **cadastro no painel. Zero mudança de código.** Isso é critério de aceite da arquitetura.

## 7. Permissões

- Permissões granulares por rank (strings): `guild.branding.edit`, `members.view`, `members.manage`, `members.kick`, `ranks.manage`, `characters.manage_own`, `characters.manage_any`.
- Futuras: `events.*`, `raids.*`, `dungeons.*` — adicionar ao enum, não refatorar.
- Dono da guilda tem todas. Checagem sempre no backend via `TenantCtx::require(perm)`.
- Superadmin é papel da **plataforma** (catálogo de jogos), não da guilda: não dá acesso a dados de tenant.

## 8. Convenções de código

**Rust**
- `cargo fmt` + `cargo clippy -- -D warnings` devem passar.
- Erros: `thiserror` no domínio, mapeados para `ApiError` com JSON `{ "code": "...", "message": "..." }` e status HTTP coerente.
- Nada de `unwrap()`/`expect()` fora de testes e startup.
- Handlers finos: validação → serviço de domínio → repositório.
- Queries com `sqlx::query_as!` (checadas). `SQLX_OFFLINE=true` no CI.

**Frontend**
- TypeScript strict. Sem `any`.
- Dados do servidor só via TanStack Query; estado de UI local com `useState`/zustand se necessário.
- Cores da guilda aplicadas via CSS variables (`--guild-primary`, `--guild-secondary`) no layout do tenant.
- UI em **pt-BR**; textos centralizados para futura i18n.

**Geral**
- Commits: Conventional Commits.
- Toda migration é reversível ou explicitamente marcada como irreversível.
- Não adicionar dependência nova sem justificar no PR.

## 9. Comandos

```bash
docker compose up -d postgres minio        # infra local
cd backend && sqlx migrate run             # migrations
cargo run -p api                           # API em :8080
cargo test --workspace                     # testes (usa banco de teste)
cd frontend && pnpm dev                    # web em :5173
pnpm gen:api                               # regenera client a partir do OpenAPI
```

## 10. Testes

- Backend: testes de integração com Postgres real (`sqlx::test`), um banco por teste.
- **Teste obrigatório de isolamento**: guilda A nunca lê/escreve dado da guilda B (por API e por query direta com RLS).
- Frontend: Vitest para lógica; Playwright para fluxos críticos (P1).

---

## 11. Roadmap

### P0 — Fundação (bloqueia tudo)
1. **Workspace e infra**: estrutura de pastas da seção 3, docker-compose, CI (fmt, clippy, test, build web).
   - Aceite: `docker compose up` + `cargo run` + `pnpm dev` sobem sem erro; CI verde num PR vazio.
2. **Schema base + RLS**: tabelas da seção 5 (exceto reservadas), policies RLS, role de app sem bypass.
   - Aceite: teste prova que query sem `app.guild_id` não retorna linhas de tenant; teste de isolamento A/B passa.
3. **Auth**: registro, login, logout, `/me`, sessão por cookie.
   - Aceite: senha com argon2; cookie HttpOnly+Secure+SameSite=Lax; rota protegida retorna 401 sem sessão.
4. **Catálogo de jogos pelo superadmin**: CRUD de jogos/classes/servidores (com filtro de classes por servidor), `character_schema` e ranks padrão por jogo.
   - Aceite: superadmin cadastra um jogo e `GET /api/games/{slug}` retorna classes e `character_schema`; um segundo jogo entra só por cadastro; não-superadmin recebe 403.
5. **Criação de guilda + TenantCtx**: criar guilda escolhendo jogo/servidor; criador vira dono; ranks padrão criados.
   - Aceite: extractor rejeita não-membro com 403; slug único; ranks padrão do jogo aplicados.

### P1 — MVP (visual + players)
1. **Visual da guilda**: editar logo, banner (upload S3), cores, tema, lema, descrição (markdown), redes sociais.
   - Aceite: só quem tem `guild.branding.edit` edita; upload valida tipo/tamanho (≤2 MB, png/jpg/webp); cores aplicadas no layout em tempo real; preview antes de salvar.
2. **Página pública da guilda** (`/g/{slug}`): branding + lista de membros ativos (nick, classe, nível, rank), se `public_page_enabled`.
   - Aceite: desligada → 404; não expõe e-mail nem notas internas.
3. **Gestão de membros**: listar (filtros por rank/status/classe, busca, paginação), adicionar (com ou sem conta), editar rank/status/notas, remover (soft).
   - Aceite: permissões respeitadas; toda alteração gera registro em `member_history`.
4. **Personagens**: CRUD de personagens por membro, main/alts, formulário dinâmico a partir do `character_schema`.
   - Aceite: `attributes` inválido → 422 com campo; exatamente um `is_main` por membro.
5. **Ranks**: CRUD de ranks, ordenação, edição de permissões.
   - Aceite: não é possível remover a última permissão de gestão do dono; não remove rank em uso sem realocar membros.
6. **Convites**: link de convite com expiração para um usuário entrar e vincular-se a um membro existente.
   - Aceite: link expira; uso único (ou N usos configurável).

### P2 — Preparação para atividades
1. **Base `activities`** (tabela única com `kind` + extensões) e `activity_participants`; sem UI completa ainda.
   - Aceite: criar um `kind` novo não requer migration no core além da extensão do próprio kind.
2. **Eventos simples**: agenda da guilda, inscrição de personagens.
3. **Login com Discord** e webhook de notificação para canal da guilda.
4. **Histórico/auditoria** visível no painel.

### P3 — Expansão
1. Raids e dungeons (composição por `role_hint`, lista de espera, presença).
2. DKP/pontos de presença e loot.
3. Segundo jogo em uso real (validar o catálogo na prática).
4. Planos/billing por guilda, limites por plano.
5. Dashboard de métricas da guilda (atividade, crescimento, distribuição de classes).

---

## 12. O que NÃO fazer

- Não criar colunas específicas de Perfect World (ou de qualquer jogo) no core.
- Não colocar dados de jogo (classes, servidores, campos) no código ou em seeds: é cadastro do superadmin.
- Não implementar eventos/raids/dungeons antes do P1 estar fechado.
- Não usar Supabase nem BaaS equivalente.
- Não acessar dados de tenant fora do `TenantCtx`.
- Não guardar segredos no repositório (`.env.example` só com placeholders).