FROM rust:1-bookworm AS build
WORKDIR /app
ENV SQLX_OFFLINE=true
COPY . .
RUN cargo build --release -p api

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN useradd --system --uid 10001 app \
    && mkdir -p /var/lib/guildos/media \
    && chown app /var/lib/guildos/media
# Uploads (STORAGE=local); em produção é um volume nomeado, que herda o dono daqui.
ENV STORAGE_DIR=/var/lib/guildos/media
COPY --from=build /app/target/release/api /usr/local/bin/api
USER app
EXPOSE 8080
CMD ["api"]
