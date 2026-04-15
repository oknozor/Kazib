FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .

RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash && \
    /usr/local/cargo/bin/cargo-binstall dioxus-cli --version 0.7.4 --no-confirm

RUN dx bundle --package kazib --web --release --debug-symbols=false

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates catatonit \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder --chown=root:root --chmod=755 \
     /app/target/dx/kazib/release/web/ /app

RUN mkdir -p /app/data && chown 65534:65534 /app/data

ENV PORT=8080 \
    IP=0.0.0.0

EXPOSE 8080
WORKDIR /app

USER 65534:65534

ENTRYPOINT ["/usr/bin/catatonit", "--", "/app/server"]
