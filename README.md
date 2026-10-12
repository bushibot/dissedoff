# Dissedoff (working title)

An open-source, self-hostable, client-server chat + voice platform — a real alternative to
Discord's non-advertising features, built so anyone can run their own server on any platform.

**Status:** Phase 1 + first slice of Phase 2 are real and tested, not stubs: auth, roles,
channels, messages, a real-time gateway, and real Mumble voice bundled as a supervised process
in the single container. Three clients exist: a terminal reference client (`dissedoff-cli`), a
browser web UI served by the server itself, and a native desktop app (`dissedoff-desktop`, Tauri).
The desktop app was verified for real on Linux — launched, driven with simulated keyboard/mouse
input against a real running server, and screenshotted at each step: login failure on a bad
password, registration, channel creation, and a sent/received chat message all genuinely worked.
macOS and Windows desktop builds are wired into CI but not yet run/tested by a human on those
OSes. No multi-person usage has happened yet (all verification so far is one operator against a
real server, plus one fully automated two-account test).

A tagged release, **v0.1.0**, is published with a real pulled-and-run GHCR image and
Linux/macOS/Windows server+CLI binaries: https://github.com/bushibot/dissedoff/releases/tag/v0.1.0

Full architecture, licensing rationale, voice/federation design, and the phased roadmap (with
per-phase status) live in [`PLAN.md`](./PLAN.md).

## Getting a server running

**Easiest — downloaded release:** grab a prebuilt image or binary from the
[Releases page](https://github.com/bushibot/dissedoff/releases) once one exists, or run the
one-line, fully non-interactive installer (reviews the script before piping it into a shell you
don't control, as with any installer):

```sh
curl -fsSL https://raw.githubusercontent.com/bushibot/dissedoff/master/scripts/install.sh | bash
```

See [`docs/AI_SETUP.md`](./docs/AI_SETUP.md) for the exact non-interactive command sequence
(server setup through first-account creation through posting a message), written so a script or
an AI agent can follow it without a human in the loop.

**Using it:** open `http://<host>:8080/` in a browser — a full login/register + chat + basic
server-admin web UI is baked directly into the server binary, no separate client install needed.
Or grab the `dissedoff-desktop` app for your OS from the
[Releases page](https://github.com/bushibot/dissedoff/releases) — same UI, as a standalone native
app instead of a browser tab. It's unsigned for now (no Apple notarization or Windows code-signing
cert yet — real recurring costs, see `PLAN.md`), so expect a Gatekeeper/SmartScreen warning on
first launch. Android/iOS clients don't exist yet.

## Important: clean-room constraint

Feature parity research for this project is done via Discord's own public developer
documentation, ordinary UX observation of one's own account, and secondary written sources —
**never** via decompiling or disassembling Discord's client or service. Discord's Terms of
Service explicitly prohibit reverse engineering/decompiling their software, and doing so risks
DMCA §1201 and copyright exposure. Do not add decompiled code, extracted strings, or copied
assets (icons, sounds, exact copy text) from Discord to this repository.

## Repo layout

```
crates/
  protocol/        # wire types, gateway event enum, and REST DTOs shared by server + every client (Apache-2.0)
  permissions/      # role/permission bitfield model + resolution algorithm (Apache-2.0)
  client-core/       # REST client + gateway connection state machine shared by every client (Apache-2.0)
server/              # the single-binary dissedoff-server: REST + WS gateway, auth, identity, voice supervisor (AGPL-3.0-or-later)
  web/app.html       # the browser login/chat/admin UI, embedded into the binary at compile time
apps/
  cli/               # terminal reference client built on client-core (Apache-2.0)
  desktop/           # native desktop app (Tauri v2) — same UI as server/web/app.html, as a standalone app (Apache-2.0)
scripts/install.sh   # one-line, non-interactive installer (docker-based)
docs/AI_SETUP.md     # exact non-interactive setup steps, for scripts/AI agents
.github/workflows/   # CI (build+test on every push) and the tag-triggered release pipeline
```

## License

Dual-licensed by design (see the plan doc, "Licensing split"):
- `server/` — **AGPL-3.0-or-later** ([LICENSE-AGPL](./LICENSE-AGPL)). Closes the closed-source
  SaaS-fork loophole: anyone running a modified server as a hosted service must publish their
  changes.
- `crates/protocol`, `crates/permissions`, `crates/client-core`, and all client apps (`apps/*`) —
  **Apache-2.0** ([LICENSE-APACHE](./LICENSE-APACHE)). Required to keep an eventual iOS App
  Store release legally viable (strong copyleft has unresolved conflict with Apple's DRM-wrapped
  distribution — see the VLC 2011 App Store removal).

Each crate's `Cargo.toml` states its own `license` field; trust that over this README if they
ever disagree.

## Building from source

Requires a Rust toolchain (stable, 2021 edition workspace). From the repo root:

```sh
cargo build --workspace
cargo test --workspace            # 11 tests, including a full register→chat→receive e2e test
cargo run -p dissedoff-server     # self-contained: creates /data, bootstraps roles, generates
                                   # its own salt + (if voice enabled) Murmur cert, on first run
cargo run -p dissedoff-cli -- http://127.0.0.1:8080   # terminal client
cargo run -p dissedoff-desktop                         # native desktop client (needs webkit2gtk-dev
                                                        # + gtk3-dev on Linux; see release.yml for the
                                                        # exact apt package list)
```

Or build and run the real single-container image (bundles Murmur too):

```sh
docker build -t dissedoff-server .
docker run -d -p 8080:8080 -p 64738:64738 -v dissedoff-data:/data dissedoff-server
```

## What's real vs. still a stub

Real, tested, and verified against the actual running binary/container (not mocked):
auth (argon2 + opaque sessions), first-registrant-becomes-owner bootstrapping, the permission
bitfield model, channel/message CRUD, the WebSocket gateway's real-time fan-out, the hashed
per-server user-id scheme, and Murmur embedded as an auto-configured, crash-restarted child
process with its voice port actually reachable.

Also real: a browser-based web UI (login/register, channel list + creation, live chat, voice
status) served directly by the binary at `/`, and the same UI wrapped as a native desktop app
(`apps/desktop`, Tauri v2) — both verified by actually registering an account, creating a channel,
and posting/receiving a message through them against a real running server. The desktop app was
driven with simulated input and screenshotted at each step on Linux (see commit history); it isn't
just "it compiled." The one-line installer (`scripts/install.sh`) and the release pipeline are
real too: `v0.1.0` is a published release with a GHCR image that was pulled fresh from a different
process and confirmed to boot, plus Linux/macOS/Windows server+CLI binaries actually present on
the release page. Along the way this caught and fixed a real bug — the Dockerfile hadn't been
updated when `apps/cli` joined the workspace, so the image build was silently broken until this
round of work.

Still open (tracked in the plan's roadmap, not silently dropped): proxying Murmur's Ice control
interface through Dissedoff's own permission engine and `VoiceStateUpdate` events (so a client can
actually *join* voice through Dissedoff, not just see that it's running); federation; Android/iOS
clients (see `PLAN.md` Phase 4 Mobile, not started — needs a separate mobile SDK/NDK toolchain);
push-to-talk/overlay (needs the native helper process described in `PLAN.md`, not built — the
desktop app today is a plain webview, same limitations as the browser); macOS/Windows desktop
builds are wired into CI but not yet confirmed working by an actual run on those OSes, since this
was all built and verified from a headless Linux container; code-signing/notarization for the
desktop app (unsigned builds will warn on first launch); and per-server-configurable
required-registration-fields policy (currently: password required, username optional with an
auto-generated fallback).
