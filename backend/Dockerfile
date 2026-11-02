# Build a partir da raiz do repositório (contexto `.`, `-f backend/Dockerfile`):
# a API embute os seeds de `game-packs/` em tempo de compilação.
FROM rust:1-bookworm AS build
WORKDIR /app
ENV SQLX_OFFLINE=true
COPY backend backend
COPY game-packs game-packs
WORKDIR /app/backend
RUN cargo build --release -p api

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN useradd --system --uid 10001 app
COPY --from=build /app/backend/target/release/api /usr/local/bin/api
USER app
EXPOSE 8080
CMD ["api"]
