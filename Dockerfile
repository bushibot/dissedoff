# Single-container build: everything needed to self-host a Dissedoff server
# lands in one image. Voice (Murmur, bundled as a managed child process) is
# added here in Phase 2 — this Phase 1 image is text/chat only.

FROM rust:1-slim AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY server ./server
RUN cargo build --release -p dissedoff-server

FROM debian:stable-slim AS runtime
# mumble-server provides the actual Murmur binary (installed as
# /usr/bin/mumble-server), bundled here so voice ships in this same single
# container — launched and supervised by dissedoff-server itself, never
# started via mumble-server's own systemd unit.
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates mumble-server \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/dissedoff-server /usr/local/bin/dissedoff-server

ENV DISSEDOFF_DATA_DIR=/data
ENV DISSEDOFF_BIND_ADDR=0.0.0.0:8080
VOLUME ["/data"]
EXPOSE 8080
EXPOSE 64738

ENTRYPOINT ["/usr/local/bin/dissedoff-server"]
