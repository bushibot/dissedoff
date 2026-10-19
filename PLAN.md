# Dissedoff — Open-Source Discord Replacement: Architecture & Roadmap Plan

## Context

Griffon wants a credible, open-source, self-hostable Discord replacement ("Dissedoff" — working
title) with a real client-server architecture, written primarily in Rust, that merges chat/media
and voice into one coherent system. The motivation: no serious open competitor to Discord exists
today, and Discord is leaning harder into ads/monetization. Inspiration: the gap left by
Revolt/**Stoat** (`github.com/stoatchat` — Rust, open-source, explicitly *not* federated, thin
maintainer bandwidth) and Spacebar/Fosscord (TypeScript, AGPL-3.0, aims for literal Discord API
parity, actively maintained but wrong language and higher IP exposure from mimicking Discord's
actual wire protocol).

This plan is a **research/architecture document**, produced before any code existed. Two
interpretive calls were made on ambiguous parts of the original request, stated here rather than
left implicit:

- **"storytold"** does not resolve to any matching GitHub project. The description given (open
  source, Discord alternative, "the failure of any meaningful competition to Discord") matches
  **Stoat** (`stoatchat`, formerly Revolt) almost exactly. This plan treats "storytold" as Stoat.
- **"Decompile Discord's... features and data set"** is treated as *clean-room feature-parity
  research* — studying what features exist and how they behave via Discord's own public
  developer docs, UX observation of one's own account, and secondary written sources — **not**
  literal binary/client decompilation. Discord's ToS explicitly bars reverse-engineering/
  decompiling their software, and doing so risks DMCA §1201 anti-circumvention exposure and
  copyright claims on anything extracted. Clean-room feature study (build an equivalent feature
  from a written description of its behavior, never from disassembled code or extracted assets)
  is the standard, defensible path every comparable project (Spacebar, Revolt/Stoat) has taken.
  This is a hard constraint on the project, not a preference. **Never add decompiled code,
  extracted strings, or copied assets (icons, sounds, exact copy text) from Discord to this
  repository.**

## Recommended Architecture

### System decomposition — single container (+ optional DB container)

- **`dissedoff-server`** (Rust, axum/tokio): REST API + WebSocket gateway, auth, permissions,
  messaging/media, federation module — all in-process, one binary.
- **Voice**: Murmur (Mumble's server, BSD-3-Clause C++) bundled as a **supervised child process**
  inside the same container — not a separate container. The core server auto-generates certs/
  config on first boot and proxies Murmur's control interface through its own REST API, so no
  admin or user ever touches Murmur's config files or certs directly. This is the direct fix for
  the "Mumble always sucked to set up" complaint.
- **Database**: embedded SQLite by default (true single-container, zero-config deploy); optional
  Postgres as the one allowed second container for larger servers.
- No message queue, no separate media/gateway/cache services — this is what "Rust + one process"
  buys you that a Matrix Synapse-style stack can't.

### Voice: phased approach

1. **MVP: Murmur-embedded** (as above) — lowest risk, solves the stated setup-pain problem,
   excellent latency/CPU, BSD license (no copyleft entanglement). Ceiling: audio-only, no video.
2. **Later: layer in LiveKit** (Apache-2.0) as a second managed child process, used only for
   channels explicitly flagged video-capable (calls/screenshare/stage). Voice channels stay on
   Murmur.
3. **Rejected for now**: hand-rolling a custom Rust WebRTC SFU (str0m/webrtc-rs) — a multi-month-
   to-multi-year project in itself; treat as a long-term stretch goal only if LiveKit-bundling
   proves too heavy in practice.

### Data model & identity

Standard server → channel (text/voice/category) → role (bitfield permissions + per-channel
overwrites) → message/attachment model, same *shape* as Discord's documented permission model
(fair game — it's Discord's own public bot-API docs) but independently implemented bit layout,
snowflake algorithm, and error schema.

**Hashed user ID** (the "no central registration, only a hash leaves the server" requirement):
`public_user_id = base58(HMAC-SHA256(server_secret_salt, internal_user_id))[0..16]`. The salt is
generated once per server at first boot, stored only in the data volume, never transmitted —
including during federation. A server's internal UUID primary key never appears in any API
response or federated payload. Rotating the salt re-keys every public ID on that server
(deliberate privacy feature, not a bug — document it as such).

### Protocol

Clean-room REST + WebSocket gateway, same *pattern* as Discord's (proven UX, not proprietary) but
independently named events/fields/opcodes — no mirroring of Discord's actual wire format, to keep
IP exposure low.

### Federation — deliberately lightweight, opt-in

Explicitly **not** Matrix's full room-DAG state-sync model (that's the ops-complexity the user
wants to avoid). What it federates: server discovery (self-registered to any directory, no single
blessed authority), federated invites (direct HTTPS fetch, no sync), opt-in per-relationship
cross-server presence, and store-and-forward signed delivery for cross-server mentions/DMs.
Trust model closer to ActivityPub's signed-delivery pattern than Matrix's PKI/federation.

### Clients

- **Tauri v2** as the primary client framework — only credible way to hit desktop (Linux/Mac/
  Windows) + mobile (Android/iOS) + VR-headset (Android-based, e.g. Quest) reach from one Rust-
  backed codebase. Shared `protocol`, `client-core`, and `permissions` crates reused across server
  and all clients.
- **Known tradeoff, hybrid mitigation**: webview-based UI is weaker for low-latency global-hotkey
  push-to-talk and in-game overlay (a real need for the gaming-distro audience). Plan: a small
  separate native Rust helper process alongside the Tauri app, specifically for hotkey capture and
  overlay rendering, talking to the main app over local IPC.
- **Linux packaging**: Flatpak (primary — correct citizen on Bazzite/Fedora Atomic) + AppImage
  (universal fallback, best for Arch) + AUR package. Not AppImage-only.
- macOS: notarized `.app`/`.dmg`. Windows: signed `.exe`/`.msi` (code-signing cert = real recurring
  cost). Android: APK (Play Store + sideload), explicitly tested on Quest's Android shell for VR
  usability. iOS: later phase, App Store, gated on the license split below.

### Licensing split

**AGPL-3.0 for `dissedoff-server` + federation code. Apache-2.0 for `protocol`, `client-core`,
`permissions`, and all client app code.** AGPL on the server closes the "closed-source SaaS fork"
loophole. Permissive license on the client is what makes an **iOS App Store listing legally
viable at all** — strong copyleft distributed through Apple's DRM-wrapped App Store has
unresolved legal friction (VLC was pulled from the App Store in 2011 over exactly this GPL-vs-DRM
conflict, and only returned in 2013 after relicensing more permissively). This split is the
standard workaround and directly unblocks "charge a few bucks for the iOS client."

## Phased Roadmap

0. **Legal/licensing groundwork** — lock license split, get an actual trademark opinion on
   "Dissedoff" vs "Discord" (sound/spelling/logo confusion risk — self-assessment is not enough
   given real legal exposure), scaffold repo/crate structure with license headers from day one.
   **Status: license split locked and implemented (AGPL server / Apache-2.0 everything else);
   trademark opinion still outstanding — not a legal clearance, just an engineering choice so far.**
1. **MVP server + desktop client (text only)** — auth, servers/channels/roles/permissions,
   REST+WS gateway, SQLite mode, Tauri desktop client with messaging+images. Exit: two people on
   two independently-run servers can register and chat with zero manual config.
   **Status: server side done and tested (auth, roles/permissions, channels, messages, real-time
   gateway, SQLite). A browser-based web UI (login/register, channels, live chat, voice status)
   is baked into the server binary and served at `/`. A native desktop app (`apps/desktop`, Tauri
   v2) now also exists, wrapping that same UI — verified for real on Linux (launched under a
   virtual display, driven with simulated keyboard/mouse input, screenshotted at each step:
   failed login, registration, channel creation, sending/receiving a chat message). macOS/Windows
   desktop builds are wired into the release pipeline (`tauri-apps/tauri-action` in
   `.github/workflows/release.yml`) but not yet confirmed by an actual run on those OSes. No
   mobile client exists. The native helper process for push-to-talk/overlay (see "Clients" above)
   is not built — today's desktop app is a plain webview with the same input limitations as the
   browser. The "two people" exit criterion has only been exercised as one operator running
   multiple accounts/processes, plus one fully automated two-account test — not by two actual
   separate people yet.**

   **v0.2.0 additions:** channel rename/delete, message edit/delete (with an "(edited)" marker),
   and a typing indicator are now real REST endpoints + gateway events (`CHANNEL_UPDATE`,
   `CHANNEL_DELETE`, `MESSAGE_UPDATE`, `MESSAGE_DELETE`, `TYPING_START`), wired into both UI
   copies with right-click context menus on channels and messages. The client also supports
   multiple independent server connections (a left-hand server rail, Discord-style) — each one
   its own account, since there's no shared identity across deployments yet; switching between
   them re-points the REST/WS connection without a page reload. All six new endpoints were
   smoke-tested against the live running server (create/rename/delete channel, edit/delete
   message, typing), not just compiled. Right-click delete/rename always shows in the UI
   regardless of the viewer's actual permissions — the server still enforces `MANAGE_CHANNELS`/
   `MANAGE_MESSAGES`/message-ownership and a non-owner gets a 403 surfaced as a banner; a
   permission-aware UI that hides actions you can't take isn't built yet.
2. **Voice integration** — Murmur supervisor, auto-cert/config, control-API proxy, voice UI. Exit:
   join voice with zero Mumble-specific setup, ever.
   **Status: partial. Murmur runs embedded, auto-configures certs/config, and crash-restarts —
   zero manual setup to get the voice server itself running. Still missing: the control-API proxy
   and `VoiceStateUpdate` bridging that would let a Dissedoff client actually join a voice channel
   through Dissedoff itself, rather than needing a separate native Mumble client pointed at the
   port directly.**
3. **Lightweight federation** — keypairs, signed requests, discovery, federated invites, opt-in
   presence. Exit: two independent servers peer and show cross-server presence with no shared
   infra. **Status: not started.**
4. **Mobile (Android first)** — Tauri Android build, VR-headset usability pass, push notifications
   (prefer UnifiedPush over a hard FCM dependency). Exit: full text+voice on phone/tablet/Quest.
   **Status: not started.**
5. **Video/screenshare** — bundle LiveKit as a second managed child process for video-capable
   channels. Exit: a video call works end-to-end. **Status: not started.**
6. **iOS + polish** — resolve App Store distribution via the license split, iOS Tauri build,
   native PTT/overlay helper module, Flatpak/AUR polish, AppImage auto-update.
   **Status: not started.**

## Open items / assumption debrief (not yet confirmed or resolved)

- License split, Murmur-now/LiveKit-later voice phasing, Tauri + native-helper client approach,
  and lightweight (non-Matrix-style) federation scope were all judgment calls made to keep moving
  rather than stalling on questions — flagged for confirmation, not necessarily locked forever.
- No actual trademark/legal opinion has been obtained on the "Dissedoff" name.
- Recurring costs with no engineering answer: Windows code-signing cert, Apple Developer Program
  fee, possible push-notification infra.

## Verification

Phase 1 exit criterion (two independent self-hosted servers, two users, real chat, zero manual
config) is **partially** demonstrated: real chat with zero manual config is proven (automated
test + manual two-process CLI run by one operator), but it has not yet been run by two actually
separate people on two actually separate machines/servers. Treat that distinction as still open.
