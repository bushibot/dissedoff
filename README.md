# Dissedoff (working title)

An open-source, self-hostable, client-server chat + voice platform — a real alternative to
Discord's non-advertising features, built so anyone can run their own server on any platform.

**Status:** Phase 0/1 scaffolding. Nothing production-ready yet.

Full architecture, licensing rationale, voice/federation design, and the phased roadmap live in
the planning document this repo was scaffolded from:
`/config/.claude/plans/mellow-inventing-crayon.md` (copy it into this repo as `PLAN.md` before
sharing outside the machine it was written on).

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
  protocol/       # wire types + gateway event enum — shared by server and every client (Apache-2.0)
  permissions/     # role/permission bitfield model + resolution algorithm (Apache-2.0)
  client-core/     # gateway connection state machine for clients (Apache-2.0) — placeholder until Phase 1
server/            # the single-binary dissedoff-server: REST + WS gateway, auth, identity, voice supervisor (AGPL-3.0-or-later)
```

## License

Dual-licensed by design (see the plan doc, "Licensing split"):
- `server/` — **AGPL-3.0-or-later** ([LICENSE-AGPL](./LICENSE-AGPL)). Closes the closed-source
  SaaS-fork loophole: anyone running a modified server as a hosted service must publish their
  changes.
- `crates/protocol`, `crates/permissions`, `crates/client-core`, and all future client apps —
  **Apache-2.0** ([LICENSE-APACHE](./LICENSE-APACHE)). Required to keep an eventual iOS App
  Store release legally viable (strong copyleft has unresolved conflict with Apple's DRM-wrapped
  distribution — see the VLC 2011 App Store removal).

Each crate's `Cargo.toml` states its own `license` field; trust that over this README if they
ever disagree.

## Building

Requires a Rust toolchain (stable, 2021 edition workspace). From the repo root:

```sh
cargo build --workspace
cargo test --workspace
cargo run -p dissedoff-server   # serves a /health endpoint on :8080
```
