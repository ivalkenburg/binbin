FROM node:24-bookworm-slim AS web
WORKDIR /build/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY Cargo.toml ../Cargo.toml
COPY web/ ./
RUN npm run build

FROM rust:1.97-slim-bookworm AS build
WORKDIR /build
RUN apt-get update && apt-get install -y --no-install-recommends build-essential && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY --from=web /build/web/dist ./web/dist
RUN cargo build --locked --release -p binbin-server --bin binbin-server

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 binbin && useradd --uid 10001 --gid binbin --no-create-home binbin \
    && mkdir /data && chown binbin:binbin /data
COPY --from=build /build/target/release/binbin-server /usr/local/bin/binbin-server
ENV BINBIN_DATA=/data BINBIN_BIND=0.0.0.0:8080 BINBIN_PUBLIC_URL=http://localhost:8080
USER binbin
VOLUME ["/data"]
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s CMD curl --fail --silent http://127.0.0.1:8080/healthz || exit 1
ENTRYPOINT ["binbin-server"]
CMD ["serve"]
