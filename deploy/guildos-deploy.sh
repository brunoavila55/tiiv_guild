#!/bin/sh
# Instalado no servidor em /usr/local/bin/guildos-deploy e usado como forced command
# da chave SSH de deploy (authorized_keys do usuário `deploy`).
#
# Uso (via CI):  echo "$GHCR_TOKEN" | ssh deploy@host <sha-do-commit>
# - o SHA vem em SSH_ORIGINAL_COMMAND e vira a tag das imagens;
# - o token do GHCR chega pela stdin e só vive durante o pull.
set -eu

REPO=brunoavila55/tiiv_guild
DIR=/opt/guildos

SHA="${SSH_ORIGINAL_COMMAND:-}"
case "$SHA" in
  *[!0-9a-f]* | "") echo "uso: <sha de 40 caracteres>" >&2; exit 2 ;;
esac
[ "${#SHA}" -eq 40 ] || { echo "uso: <sha de 40 caracteres>" >&2; exit 2; }

IFS= read -r GHCR_TOKEN || true
[ -n "${GHCR_TOKEN:-}" ] || { echo "token do GHCR ausente na stdin" >&2; exit 2; }

cd "$DIR"
curl -fsSL "https://raw.githubusercontent.com/$REPO/$SHA/deploy/compose.prod.yml" -o docker-compose.yml.new
mv docker-compose.yml.new docker-compose.yml

if grep -q '^IMAGE_TAG=' .env; then
  sed -i "s/^IMAGE_TAG=.*/IMAGE_TAG=$SHA/" .env
else
  echo "IMAGE_TAG=$SHA" >> .env
fi

DOCKER_CONFIG=$(mktemp -d)
export DOCKER_CONFIG
trap 'rm -rf "$DOCKER_CONFIG"' EXIT
printf '%s' "$GHCR_TOKEN" | docker login ghcr.io -u deploy --password-stdin >/dev/null
docker compose pull --quiet
docker compose up -d --remove-orphans --wait --wait-timeout 120
docker image prune -f >/dev/null

echo "deploy ok: $SHA"
