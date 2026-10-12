#!/usr/bin/env bash
# Non-interactive installer for a single Dissedoff server.
#
# Designed to be run unattended — by a human, a provisioning script, or an
# AI agent following docs/AI_SETUP.md — with zero prompts. Every setting is
# an environment variable with a sane default; nothing is asked for on
# stdin. Safe to re-run: if the named container already exists, it is left
# alone and the script just reports its status.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/bushibot/dissedoff/master/scripts/install.sh | bash
#   # or, with overrides:
#   DISSEDOFF_PORT=9000 DISSEDOFF_DATA_VOLUME=my-dissedoff-data bash install.sh
#
# Review the script before piping it into a shell you don't control —
# that's good practice for any installer, not specific to this one.

set -euo pipefail

IMAGE="${DISSEDOFF_IMAGE:-ghcr.io/bushibot/dissedoff:latest}"
CONTAINER_NAME="${DISSEDOFF_CONTAINER_NAME:-dissedoff}"
PORT="${DISSEDOFF_PORT:-8080}"
VOICE_PORT="${DISSEDOFF_VOICE_PORT:-64738}"
DATA_VOLUME="${DISSEDOFF_DATA_VOLUME:-dissedoff-data}"
SERVER_NAME="${DISSEDOFF_SERVER_NAME:-My Dissedoff Server}"
ENABLE_VOICE="${DISSEDOFF_ENABLE_VOICE:-true}"

log() { echo "[dissedoff-install] $*" >&2; }

if ! command -v docker >/dev/null 2>&1; then
  log "ERROR: docker is required and was not found on PATH."
  log "Install Docker first: https://docs.docker.com/engine/install/"
  exit 1
fi

if docker inspect "$CONTAINER_NAME" >/dev/null 2>&1; then
  log "Container '$CONTAINER_NAME' already exists — not touching it."
  STATUS=$(docker inspect -f '{{.State.Status}}' "$CONTAINER_NAME")
  log "Current status: $STATUS"
  if [ "$STATUS" != "running" ]; then
    log "Starting it..."
    docker start "$CONTAINER_NAME" >/dev/null
  fi
else
  log "Pulling $IMAGE ..."
  if ! docker pull "$IMAGE" 2>/dev/null; then
    log "Could not pull $IMAGE (not published yet, or offline)."
    if [ -f "./Dockerfile" ]; then
      log "Found a local Dockerfile in $(pwd) — building $IMAGE from it instead."
      docker build -t "$IMAGE" .
    else
      log "ERROR: no image available and no local ./Dockerfile to build from."
      log "Run this script from inside a checkout of the dissedoff repo, or"
      log "wait for a published release image, or set DISSEDOFF_IMAGE to one"
      log "you've already built."
      exit 1
    fi
  fi

  log "Creating volume '$DATA_VOLUME' (no-op if it already exists)..."
  docker volume create "$DATA_VOLUME" >/dev/null

  log "Starting container '$CONTAINER_NAME' on port $PORT (voice: $VOICE_PORT)..."
  docker run -d \
    --name "$CONTAINER_NAME" \
    --restart unless-stopped \
    -p "${PORT}:8080" \
    -p "${VOICE_PORT}:64738/udp" \
    -p "${VOICE_PORT}:64738/tcp" \
    -v "${DATA_VOLUME}:/data" \
    -e "DISSEDOFF_SERVER_NAME=${SERVER_NAME}" \
    -e "DISSEDOFF_ENABLE_VOICE=${ENABLE_VOICE}" \
    -e "DISSEDOFF_VOICE_PORT=64738" \
    "$IMAGE" >/dev/null
fi

log "Waiting for the server to answer on http://localhost:${PORT}/health ..."
for _ in $(seq 1 30); do
  if curl -fsS "http://localhost:${PORT}/health" >/dev/null 2>&1; then
    log "Server is up."
    echo
    echo "Dissedoff is running:"
    echo "  Web UI:        http://localhost:${PORT}/"
    echo "  Health check:  http://localhost:${PORT}/health"
    echo "  Voice port:    ${VOICE_PORT} (tcp+udp)"
    echo
    echo "Create the first (owner) account non-interactively with:"
    echo "  curl -s -X POST http://localhost:${PORT}/api/v1/auth/register \\"
    echo "    -H 'Content-Type: application/json' \\"
    echo "    -d '{\"username\":\"owner\",\"password\":\"CHANGE-ME-8-CHARS-MIN\"}'"
    echo
    exit 0
  fi
  sleep 1
done

log "ERROR: server did not come up within 30s. Check logs with:"
log "  docker logs ${CONTAINER_NAME}"
exit 1
