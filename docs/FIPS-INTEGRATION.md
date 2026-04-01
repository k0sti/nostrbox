# FIPS Integration — Implementation Tracker

Status: **In Progress**
Started: 2026-04-01

## Phase 1: FIPS on the box

### 1.1 Flake: Add FIPS as input ✅
- [x] Add FIPS flake as input (github:k0sti/fips, pinned at d294d38; `--override-input` for local dev)
- [x] Expose `fips` and `fips-ble` packages through nostrbox flake
- [x] Add FIPS binaries (`fips`, `fipsctl`, `fipstop`) to dev shell — auto-selects BLE on Linux
- [x] Verify `nix develop` works on Linux (BLE enabled, fips 0.3.0-dev confirmed)
- [ ] Verify `nix develop` works on macOS (no BLE variant)

### 1.2 NixOS module: FIPS service ✅ (initial)
- [x] Created `nixos/modules/fips.nix` — systemd service with CAP_NET_ADMIN, socket group perms
- [x] Wired into flake as `nixosModules.fips`
- [x] `services.nostrbox.fips` options: package, socketPath, transports, peers, listenAddress
- [ ] Test on actual mac-mini hardware
- [ ] Create `nixos/modules/fips.nix` — standalone systemd service module
  - `AmbientCapabilities = CAP_NET_ADMIN` for TUN
  - Reads nsec from nostrbox's identity (shared keypair)
  - Configurable transports, listen address, static peers
  - Socket path configurable (default `/run/fips/fips.sock`)
  - Socket group = `nostrbox` so unprivileged nostrbox user can query via fipsctl
- [ ] Wire into `nixosModules.nostrbox` — `services.nostrbox.fips.enable` starts the FIPS service
- [ ] Add to mac-mini configuration

### 1.3 Shared identity ☐
- [ ] NostrBox writes FIPS key files on startup from its own nsec
  - FIPS reads nsec from key file (bech32 format, already supported)
  - Key file: `/var/lib/nostrbox/fips.key` (mode 0600)
  - Pub file: `/var/lib/nostrbox/fips.pub`
- [ ] FIPS config references these paths via `node.identity.persistent: true` + key file location
- [ ] Verify: FIPS node npub == NostrBox relay npub

### 1.4 FIPS config generation ☐
- [ ] NostrBox generates `/etc/fips/fips.yaml` from `nostrbox.yaml` FIPS section
- [ ] Config maps:
  - `fips.listen` → FIPS transport listen addresses
  - `fips.transports` → enabled transports (udp, tcp, ble)
  - `fips.peers` → static peer list
  - `fips.dns.enable` → FIPS DNS responder
- [ ] Template lives in `crates/core/` or new `crates/fips/`

### 1.5 Rust crate: `crates/fips/` ☐
- [ ] `config.rs` — generate FIPS YAML config from NostrBox config
- [ ] `status.rs` — parse fipsctl output (JSON over Unix socket)
  - Connect to FIPS control socket
  - `show status`, `show peers`, `show links`, `show tree`
  - Types: `FipsStatus`, `FipsPeer`, `FipsLink`
- [ ] `identity.rs` — write nsec/npub to FIPS key files
- [ ] Tests for config generation and status parsing

### 1.6 Management API routes ☐
- [ ] `GET /api/fips/status` — node status, mesh info
- [ ] `GET /api/fips/peers` — connected peers list
- [ ] `POST /api/fips/peers/connect` — trigger peer connection
- [ ] `POST /api/fips/peers/disconnect` — disconnect peer
- [ ] Wire into `ext-management` router

### 1.7 Web UI: Mesh view ☐
- [ ] New "Mesh" page in web UI
- [ ] Shows: node identity, mesh status (up/down), uptime
- [ ] Peer list: npub, transport, link state, latency
- [ ] Add peer form: npub + address + transport
- [ ] Auto-refresh on interval

### 1.8 NixOS profiles ✅
- [x] `nixos/profiles/appliance.nix` — production defaults (FIPS enabled, firewall, hardening)
- [x] `nixos/profiles/dev.nix` — dev tools, debug logging, relaxed settings
- [x] `nixosConfigurations.mac-mini` uses appliance profile
- [x] `nixosConfigurations.mac-mini-dev` uses dev profile

### 1.9 Two-node connectivity test ☐
- [ ] Mac Mini running NostrBox + FIPS
- [ ] Second node (laptop/VM) running FIPS
- [ ] Verify: `ping6 <fd00-address>` works both directions
- [ ] Verify: `fipsctl show peers` shows both nodes
- [ ] Verify: `fipstop` TUI shows mesh topology
- [ ] Document test procedure in this file

## Phase 2: Federation (future)
- Relay-to-relay sync over FIPS mesh
- Group event propagation
- Blossom file sync
- Design after Phase 1 proves out

---

## Open Questions (tracked)
- [x] Key format conversion — FIPS already reads nsec bech32
- [x] npub→fd00 address — solved in FIPS codebase
- [ ] fipsctl socket permissions — can non-root user connect? Test on mac-mini
- [ ] BLE on Mac Mini 2012 — Bluetooth 4.0, need to verify BlueZ driver support under NixOS

## Decisions
- **FIPS as managed process** (not library) — privilege separation, crash isolation
- **Shared keypair** — box npub = FIPS node npub
- **FIPS main branch** — pin to HEAD, update as needed
- **BLE included** — build `fips-ble` variant, test on mac-mini hardware
- **NixOS config in nostrbox repo** — profiles for appliance vs dev, split later if needed
