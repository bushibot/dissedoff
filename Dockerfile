# Single-container build: everything needed to self-host a Dissedoff server
# lands in one image, including Murmur (voice) as a bundled, supervised
# child process — see src/voice/murmur_supervisor.rs.

FROM rust:1-slim AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY server ./server
COPY apps ./apps
# `apps/cli` is a workspace member, so cargo needs its manifest to resolve
# the workspace even though only dissedoff-server is built into this image.
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
