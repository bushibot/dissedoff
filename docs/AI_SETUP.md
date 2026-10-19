# Setting up Dissedoff non-interactively (for scripts and AI agents)

This document exists so a script, a provisioning tool, or an AI coding agent can
stand up a working Dissedoff server without any human in the loop and without
reading the rest of the repo. Every step below is a single non-interactive
command with a verifiable outcome.

## 1. Prerequisites

- Docker, reachable by whoever runs these commands (`docker version` succeeds).
- Network access to pull `ghcr.io/bushibot/dissedoff:latest` (a published image
  exists as of v0.1.0 — see [Releases](https://github.com/bushibot/dissedoff/releases)).
  No local checkout or build is needed; `install.sh` only falls back to a local
  `docker build` if the pull fails (e.g. offline, or pinning an unpublished
  `DISSEDOFF_IMAGE`).

## 2. Start the server

```sh
curl -fsSL https://raw.githubusercontent.com/bushibot/dissedoff/master/scripts/install.sh -o install.sh
bash install.sh
```

All settings are environment variables with defaults; none are prompted for.
Override any of them by exporting before running:

| Variable                  | Default                       | Meaning                                  |
|----------------------------|--------------------------------|-------------------------------------------|
| `DISSEDOFF_PORT`           | `8080`                         | Host port for the HTTP API + web UI       |
| `DISSEDOFF_VOICE_PORT`     | `64738`                        | Host port for Mumble voice (tcp+udp)      |
| `DISSEDOFF_DATA_VOLUME`    | `dissedoff-data`               | Named Docker volume for persistent data   |
| `DISSEDOFF_CONTAINER_NAME` | `dissedoff`                    | Docker container name                     |
| `DISSEDOFF_SERVER_NAME`    | `My Dissedoff Server`          | Server display name                       |
| `DISSEDOFF_ENABLE_VOICE`   | `true`                         | Whether to boot embedded Murmur voice     |
| `DISSEDOFF_IMAGE`          | `ghcr.io/bushibot/dissedoff:latest` | Image to run (or build locally from)  |

The script is idempotent: re-running it when the named container already
exists does nothing destructive, it just reports status.

## 3. Verify it came up

```sh
curl -fsS http://localhost:8080/health
# expect: {"service":"dissedoff-server","status":"ok"}
```

If this fails immediately after the install script reported success, you are
very likely running inside a container yourself with Docker-outside-of-Docker
(the Docker daemon is not local to your container) — published ports then land
on the *Docker host's* real network interface, not your own container's
loopback. Use the host's actual IP instead of `localhost` in that case.

## 4. Create the first account (becomes the server owner automatically)

The **first** account ever registered on a fresh server is automatically
granted the `owner` role (full permissions). There is no separate admin setup
step.

```sh
curl -s -X POST http://localhost:8080/api/v1/auth/register \
  -H 'Content-Type: application/json' \
  -d '{"username":"owner","password":"REPLACE-WITH-8-PLUS-CHARS"}'
```

Expected response (capture the `token` if you intend to make further API
calls as this user):

```json
{"token":"<opaque bearer token>","public_id":"<hashed public id>","username":"owner"}
```

`username` is optional in the request body — omit it (`"username": null`) to
get an auto-generated one.

## 5. Create a channel and post a message (optional, proves the chat path works)

```sh
TOKEN="<token from step 4>"
curl -s -X POST http://localhost:8080/api/v1/channels \
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{"name":"general","kind":"text"}'

CHANNEL_ID="<id from the response above>"
curl -s -X POST "http://localhost:8080/api/v1/channels/$CHANNEL_ID/messages" \
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{"content":"hello from a script"}'
```

## 6. Use the web UI, or a native desktop client

Open `http://<host>:8080/` in a browser. It's a single static page baked
into the server binary (`server/web/app.html`) — sign in with the account
created in step 4. No separate client install is needed for basic chat/admin
use.

A native desktop app (`dissedoff-desktop`, Tauri) is also published per
release for Linux (`.AppImage`/`.deb`), macOS (`.dmg`), and Windows
(`.exe`) — same UI, as a standalone app instead of a browser tab. Grab it
from the [Releases page](https://github.com/bushibot/dissedoff/releases).
Mobile (Android/iOS) is a separate, not-yet-built phase (see `PLAN.md`).

As of v0.2.0, the client (web and desktop) supports connecting to and
switching between multiple independent Dissedoff servers from one session —
each is its own saved account, since there's no shared identity across
deployments.
