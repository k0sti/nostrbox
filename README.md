# Nostrbox

Nostrbox is a Rust + TypeScript admin surface for a sovereign Nostr community server.

Current shape:
- **Rust core** for domain logic and current-state handling
- **Embedded Nostr relay** via `nostr-relay-builder`
- **ContextVM app interface** over Nostr and HTTP fallback
- **SQLite runtime store** for resolved state and indexes
- **Standalone React web UI** served by the Rust server

This repo is the **current working implementation**, not just a design sketch.

## Status

First working version.

What works today:
- registration submit / list / approve / deny
- actor list / get / detail
- group list / get / create / add member / remove member
- dashboard summaries
- Nostr event validation and replaceable-event helpers
- event publishing for key state changes into local event store
- embedded relay startup with write policy
- ContextVM transport over Nostr for the web client
- HTTP fallback for app operations
- web login via NIP-07 / Amber / Nostr Connect
- role-aware navigation and admin/member gating in UI
- public registration flow
- responsive web UI
- 48 tests passing across Rust crates

What is still rough / incomplete:
- relay query/read policy is not fully fleshed out yet
- event ingestion/projection flow exists but still needs hardening
- some docs in Obsidian still describe older planned architecture
- no production auth/session polish beyond Nostr identity + role checks
- profile metadata fetches are pragmatic, not deeply cached
- no moderation, payments, files, or Nomen integration yet

---

## Docker image

Nostrbox includes a production-oriented multi-stage Docker build.

### Build

Because the Rust workspace uses local path dependencies outside this repo,
build from the parent workspace directory so Docker can see:

- `nostrbox/`
- `nostr-1/`
- `rust-contextvm-sdk/`

Example layout:

```text
~/work/
├── nostrbox/
├── nostr-1/
└── rust-contextvm-sdk/
```

Build the image from `~/work`:

```bash
docker build -f nostrbox/Dockerfile -t nostrbox ~/work
```

A `.dockerignore` at `~/work/.dockerignore` keeps the build context small.

### Run

```bash
docker run --rm \
  -p 3000:3000 \
  -p 7777:7777 \
  -v nostrbox-data:/data \
  nostrbox
```

The image ships with a minimal config at `/etc/nostrbox/nostrbox.toml`.
To override it, mount your own config file there.

## Repository layout

```text
nostrbox/
├── crates/
│   ├── core/        # Domain types and access semantics
│   ├── nostr/       # Nostr event kinds, validation, replaceable helpers
│   ├── store/       # SQLite runtime store + connection pool
│   ├── contextvm/   # Operation handlers + event builders
│   ├── relay/       # Embedded relay setup + write policy
│   └── server/      # Axum server, relay bootstrap, transport, ingestion
├── web/             # React + Vite admin UI
├── justfile         # Dev/build/deploy commands
└── nostrbox.toml    # Local runtime config
```

---

## Architecture

### 1. Canonical vs runtime state

Nostrbox aims to keep **Nostr events** as canonical state where practical.

Current runtime model:
- **canonical-ish events** are built and signed for state changes like:
  - actor role assignment
  - group definition
  - group membership
- **SQLite** stores the current working state for fast reads and UI queries
- **events table** stores published events locally

So right now the system is already event-aware and event-producing, while still leaning on SQLite as the operational source for the running app.

### 2. Relay

The relay is built with `nostr-relay-builder`.

Current relay behavior:
- starts on a dedicated port
- accepts writes through a custom write policy
- allows NIP-59 gift wraps so encrypted ContextVM messages work
- rejects writes from unknown actors / guests

### 3. App interface

The app interface is operation-oriented.

Two transports exist:
- **ContextVM over Nostr** via `@contextvm/sdk`
- **HTTP fallback** via `POST /api/op`

The web client can switch between them in settings.

### 4. Web UI

The web app is React + Vite and is served by the Rust server.

Current UI includes:
- dashboard
- registration page
- registrations admin view
- actors list + detail panel
- groups list + detail panel
- relay/settings panel
- login/profile flows

---

## Domain model

### Actor
An actor is any pubkey-based identity.

Fields currently used:
- `pubkey`
- `npub`
- `kind = human | agent | service | system`
- `global_role = guest | member | admin | owner`
- `status = active | disabled | banned | restricted`
- `display_name`
- `groups`
- `created_at`
- `updated_at`

### Registration
A global request to join the box.

States:
- `pending`
- `approved`
- `denied`

### Group
The main scoped unit.

Fields currently used:
- `group_id`
- `name`
- `description`
- `visibility = public | group | internal`
- `slug`
- `join_policy = open | request | invite_only | closed`
- `status = active | frozen | archived`
- `members`
- `created_at`
- `updated_at`

---

## Custom Nostr event kinds

Current Nostrbox event kinds:
- `0` → metadata
- `30078` → actor role assignment
- `30079` → registration request
- `30080` → group definition
- `30081` → group membership

These use parameterized replaceable event patterns where appropriate via `d` tags.

---

## Operations

Current operation catalog:

### Registration
- `registration.submit`
- `registration.list`
- `registration.get`
- `registration.approve`
- `registration.deny`

### Actors
- `actor.list`
- `actor.get`
- `actor.detail`

### Groups
- `group.list`
- `group.get`
- `group.put`
- `group.add_member`
- `group.remove_member`

### Dashboard
- `dashboard.get`

Mutating admin operations require caller role `admin` or `owner`.

---

## HTTP routes

Current server routes:

- `GET /health`
- `POST /api/op`
- `GET /api/relay-info`
- `GET /ws` → websocket relay proxy / relay endpoint path
- static web app fallback from `web/dist`

Notes:
- the UI is served directly by the Rust server
- `/api/relay-info` is used by the web app for relay URL + server pubkey discovery

---

## Config

Nostrbox reads config from:
- env: `NOSTRBOX_CONFIG`
- default fallback: `./nostrbox.toml`

Current config shape:

```toml
bind_address = "127.0.0.1:3400"
db_path = "/path/to/nostrbox.db"
web_dist_path = "/path/to/web/dist"
identity_nsec = "nsec1..."
relay_port = 7777
relay_urls = []
public_url = "https://nostrbox.atlantislabs.space"
```

### Important fields
- `bind_address` — HTTP/UI server bind
- `db_path` — SQLite DB path
- `web_dist_path` — built frontend assets
- `identity_nsec` — server signing identity for relay + ContextVM
- `relay_port` — embedded relay port
- `public_url` — used to derive the public relay websocket URL (`wss://.../ws`)

---

## Development

### Prerequisites
- Rust toolchain
- `bun`
- `just`

### First-time setup

```bash
just web-install
just init
```

`just init` generates or accepts an existing Nostr secret key and writes `nostrbox.toml`.

### Build

```bash
just build
```

### Run locally

```bash
just run
```

### Run web dev server only

```bash
just web-dev
```

### Test

```bash
just test
```

### Release build

```bash
just build-release
```

---

## Deployment

This repo includes a simple deploy flow:

```bash
just deploy
```

That currently:
- runs Rust tests
- builds release binaries
- builds the web app
- restarts the `nostrbox` systemd service

Quick deploy without tests:

```bash
just deploy-quick
```

Service helpers:

```bash
just status
just logs
```

---

## Public dev instance

Current dev deployment target is intended to be:
- **UI/API:** `https://nostrbox.atlantislabs.space`
- **public relay URL:** `wss://nostrbox.atlantislabs.space/ws`

The NixOS deployment should pin a fixed local port in the 3xxx range and expose it through Cloudflare tunnel ingress.

---

## Web identity and transport behavior

### Login options
- Web extension (NIP-07)
- Amber (Android / NIP-07 style)
- Nostr Connect (NIP-46 bunker URI)

### Transport modes
In the settings panel the UI can use:
- **HTTP** — default and simpler
- **ContextVM** — JSON-RPC over Nostr events

If ContextVM transport is selected but not connected, the app falls back to HTTP.

---

## Testing

Current Rust test coverage:
- `crates/nostr/tests/integration.rs`
- `crates/contextvm/tests/integration.rs`
- `crates/store/tests/integration.rs`

Current total:
- **48 tests passing**

Coverage includes:
- event validation
- replaceable event resolution
- registration flows
- actor/group CRUD
- dashboard summaries
- auth error cases

---

## Current implementation notes

A few important realities of the current codebase:

1. **This is already more than a scaffold.**
   Relay startup, ContextVM transport, event publishing, and a usable UI are present.

2. **The runtime still leans heavily on SQLite.**
   That’s fine for v1, but docs should be honest that the current implementation is not yet a pure event-sourced runtime.

3. **Some older docs are stale.**
   Early RFC/presentation docs still talk about:
   - khatru
   - Nomen-backed memory
   - SurrealDB
   - Blossom
   - NWC wallet integration
   - AI agent runtime

   None of those are part of the current working repo.

4. **The actual product boundary is clearer now.**
   Nostrbox today is:
   - community admin server
   - embedded relay
   - actor/group/registration management UI
   - ContextVM reference implementation across Rust + web

---

## Near-term next steps

The most valuable next steps are:

1. **Revise docs to match reality**
   - add current-state architecture doc
   - separate “current implementation” from “future vision”
   - trim or mark obsolete RFC material

2. **Harden relay + ingestion**
   - finish read/query policy
   - tighten projection flow from events to store
   - clarify what is canonical vs derived in code and docs

3. **Polish auth / onboarding**
   - better first-run owner bootstrap
   - clearer member/admin UX
   - better role-aware empty states

4. **Improve operability**
   - better setup docs
   - DB inspection / debug commands
   - service health and relay health visibility

5. **Expand product scope only after the core is solid**
   - moderation
   - richer relay policy
   - community-level model
   - files/payments/AI only after current core stabilizes

---

## License

MIT
