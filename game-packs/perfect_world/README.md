Seeds do game pack **Perfect World**, embutidos no binário da API (`games` crate) e
sincronizados em `games` / `game_classes` / `game_servers` a cada startup.

| Arquivo | Conteúdo |
|---|---|
| `game.json` | slug, nome e faixa de nível |
| `classes.json` | classes (`slug` estável, nome exibido, `role_hint`, raça em `metadata`) |
| `races.json` | raças (referenciadas por `metadata.race` das classes) |
| `servers.json` | servidores |
| `ranks.json` | ranks criados em toda guilda nova (permissões do core) |
| `character_schema.json` | campos de `characters.attributes` (subconjunto de JSON Schema: `string`/`integer`/`number`/`boolean`, `enum`, `minimum`/`maximum`, `minLength`/`maxLength`, `required`, `additionalProperties: false`; `title` e `x-enumLabels` são só para a UI) |

O `slug` de classes/servidores é a identidade: renomear o `name` é seguro, trocar o `slug` cria outro registro.
