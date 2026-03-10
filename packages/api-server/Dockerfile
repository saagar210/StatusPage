FROM rust:slim-bookworm AS builder

WORKDIR /app

RUN apt-get update \
  && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
  && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY apps/monitor/Cargo.toml apps/monitor/Cargo.toml
COPY packages/api-server/Cargo.toml packages/api-server/Cargo.toml
COPY packages/shared/Cargo.toml packages/shared/Cargo.toml

COPY apps/monitor apps/monitor
COPY packages/api-server packages/api-server
COPY packages/shared packages/shared
COPY migrations migrations

RUN cargo build --release -p api-server --bin api-server --bin migrate

FROM debian:bookworm-slim AS runner

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates curl \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/api-server /usr/local/bin/api-server
COPY --from=builder /app/target/release/migrate /usr/local/bin/migrate

ENV API_HOST=0.0.0.0
ENV API_PORT=4000

EXPOSE 4000

CMD ["api-server"]
