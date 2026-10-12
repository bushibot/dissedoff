# Dissedoff (working title)

An open-source, self-hostable, client-server chat + voice platform — a real alternative to
Discord's non-advertising features, built so anyone can run their own server on any platform.

**Status:** Phase 1 + first slice of Phase 2 are real and tested, not stubs: auth, roles,
channels, messages, a real-time gateway, and real Mumble voice bundled as a supervised process
in the single container. A terminal reference client (`dissedoff-cli`) can register, chat, and
see messages arrive live. Verified two ways: an automated test (`tests/e2e.rs`) that registers two
accounts and confirms a message sent by one is delivered live to the other over the real gateway;
and manually, by running two `dissedoff-cli` processes under two different accounts myself (no
second person involved) against the real Docker image to confirm the CLI itself works end to end.
No GUI client yet, and no multi-person usage has happened.

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

**Using it once it's running:** open `http://<host>:8080/` in a browser — a full login/register +
chat + basic server-admin web UI is baked directly into the server binary (`server/web/app.html`),
so no separate client install is required for day-to-day text use. Dedicated native desktop/mobile
clients are a separate, not-yet-built phase (see `PLAN.md`, Phase 1 GUI client and Phase 4 mobile).

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
status) served directly by the binary at `/` — verified by registering an account, creating a
channel, and posting a message through it against a real running server, both locally and inside
the actual Docker image. The one-line installer (`scripts/install.sh`) and the release pipeline
(`.github/workflows/release.yml`) are also real: the installer was run against the real Docker
daemon end-to-end (including catching and fixing a real bug — the Dockerfile hadn't been updated
when `apps/cli` was added to the workspace, so the image build was broken until this round).

Still open (tracked in the plan's roadmap, not silently dropped): proxying Murmur's Ice control
interface through Dissedoff's own permission engine and `VoiceStateUpdate` events (so a client
can actually *join* voice through Dissedoff, not just see that it's running); federation; any
native desktop/mobile GUI client (Linux/Windows/macOS/Android — see `PLAN.md` Phase 1 GUI and
Phase 4 Mobile, neither started); per-server-configurable required-registration-fields policy
(currently: password required, username optional with an auto-generated fallback); and no
release has actually been tagged/published yet, so the release pipeline is wired up but unproven
against a real GitHub Release until a `v*` tag is pushed.
