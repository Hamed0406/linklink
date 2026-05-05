# Architecture

## Overview

linklink is a three-layer system:

```
┌─────────────────────────────────────────────────────┐
│  Data plane  —  WireGuard (boringtun, encrypted UDP) │
└─────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────┐
│  Control plane  —  Go API server + PostgreSQL        │
│  (device approval, IP allocation, config versioning) │
└─────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────┐
│  Management  —  React dashboard / CLI agent          │
└─────────────────────────────────────────────────────┘
```

The data plane operates **independently** of the control plane. If the server goes down, existing tunnels stay up.

---

## Codebase Structure

### `core/` — Rust library

Shared across all platforms (CLI, mobile via UniFFI). Contains:

| Module | Purpose |
|---|---|
| `keystore` | x25519 keypair generation, `0600` key file read/write, `Zeroizing` wrapper |
| `wireguard` | WireGuard config rendering, `wg syncconf`, `wg-quick` wrappers, `wg show dump` parser |
| `stun` | Raw UDP STUN client (RFC 5389), `XOR_MAPPED_ADDRESS` + `MAPPED_ADDRESS` parsing |
| `invite` | `LINKLINK:v1:<base64url(json)>` invite code encode/decode |
| `gossip` | Peer list merge (last-seen timestamp wins, peers never removed via gossip) |
| `config` | `AgentConfig` TOML read/write with env-var path overrides |
| `error` | Unified `Error` enum + `Result<T>` alias |

### `agent/` — Rust CLI binary

Built as a static musl binary for Linux. Uses `clap` v4 with derive macros.

Commands: `invite create`, `invite accept`, `up`, `down`, `status`, `peers`, `login`, `register`, `relay`, `reset`.

Path resolution (all overridable via env vars):

| Env var | Default (Linux) |
|---|---|
| `LINKLINK_CONFIG` | `/etc/linklink/config.toml` |
| `LINKLINK_KEY` | `/etc/linklink/private.key` |
| `LINKLINK_WG_CONFIG` | `/etc/wireguard/linklink.conf` |
| `LINKLINK_PEERS` | `/var/lib/linklink/peers.json` |
| `LINKLINK_TOKEN` | `/var/lib/linklink/token` |

### `server/` — Go control plane

chi v5 router. Migrations embedded via `embed.FS` and run on startup.

| Package | Purpose |
|---|---|
| `internal/config` | Env-var config; rejects startup if `ENV=production` and `JWT_SECRET` is default |
| `internal/db` | pgxpool connection + embedded golang-migrate runner |
| `internal/auth` | JWT (HS256), OAuth2 device flow (in-memory store + reaper), refresh token hashing |
| `internal/devices` | Register, approve, revoke, heartbeat, IP allocation (`SELECT ... FOR UPDATE`) |
| `internal/audit` | Append-only audit log table |
| `internal/api` | chi router wiring all handlers |

Database schema highlights:
- `device_status`, `acl_action`, `acl_protocol` — PostgreSQL enums for type safety
- Refresh tokens stored as SHA-256 hex hash only
- Hub API keys stored as bcrypt hash only
- `config_version` on hubs and devices — agents poll for changes

### `frontend/` — React + TypeScript + Tailwind

SPA served by nginx, proxies `/api/` to the Go server.

| File | Purpose |
|---|---|
| `src/api/client.ts` | Typed fetch wrapper; validates JWT + token format before writing to localStorage; redirects to `/login` on 401 |
| `src/pages/Login.tsx` | OAuth2 device flow UI (polls until activated) |
| `src/pages/Devices.tsx` | Device table with approve/revoke |

### `deploy/`

| File | Purpose |
|---|---|
| `docker-compose.dev.yml` | Local dev stack (Postgres + server build + frontend build); all secrets via `.env` |
| `docker-compose.prod.yml` | Production stack pulling images from Docker Hub |
| `.env.example` | Template — copy to `.env` and fill in |
| `scripts/setup-hub.sh` | iptables PREROUTING for UDP-443 → 51820 relay, UFW rules |
| `systemd/linklink-agent.service` | `AmbientCapabilities=CAP_NET_ADMIN` so agent runs without root after install |

---

## Operation Modes

### Serverless (P2P only)
Two devices exchange a `LINKLINK:v1:` invite code (or QR). Each generates its own keypair, discovers its external endpoint via STUN, and writes a WireGuard config with the peer's public key + endpoint. No server involved.

### Self-hosted control plane
Admin runs the Docker stack. Devices register via `linklink login` + `linklink register`. Server assigns tunnel IPs, approves devices, and distributes peer lists. Tunnel data still flows P2P (or via hub relay) — the server is never in the data path.

### Hub relay fallback
When P2P fails (symmetric NAT, strict firewall), traffic routes through a hub VPS. The hub runs WireGuard and is registered with the control plane. UDP port 443 is redirected to 51820 via iptables for corporate firewalls.

---

## CI / CD Pipeline

```
Push to branch  →  CI (Rust tests + Go tests + Frontend build)
Merge to main   →  Docker build → push linklink-server:main + linklink-frontend:main to Docker Hub
Git tag v*      →  same + versioned Docker Hub tags
```

Required GitHub secrets: `DOCKERHUB_USERNAME`, `DOCKERHUB_TOKEN`.
