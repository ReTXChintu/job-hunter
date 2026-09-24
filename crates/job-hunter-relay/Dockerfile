# Multi-stage build for the Job Hunter relay. Produces a small runtime image
# with just the compiled binary; the SQLite database should live on a
# mounted volume (see docs/relay.md) so it survives redeploys.

FROM rust:1.88-bookworm AS build
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates/job-hunter-relay/Cargo.toml crates/job-hunter-relay/Cargo.toml
# A trimmed workspace manifest so this build stage does not need the desktop
# app or job-hunter-core's own (much larger) dependency tree.
RUN sed -i 's#members = \[.*#members = ["crates/job-hunter-relay"]#' Cargo.toml \
    && sed -i '/crates\/job-hunter-core/d;/apps\/desktop/d' Cargo.toml
COPY crates/job-hunter-relay/src crates/job-hunter-relay/src
RUN cargo build --release -p job-hunter-relay

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /build/target/release/job-hunter-relay /usr/local/bin/job-hunter-relay
ENV JOB_HUNTER_RELAY_BIND=0.0.0.0:8787
ENV JOB_HUNTER_RELAY_DB=/data/relay.sqlite3
VOLUME ["/data"]
EXPOSE 8787
ENTRYPOINT ["/usr/local/bin/job-hunter-relay"]
