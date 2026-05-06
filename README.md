# linklink

Secure mesh VPN — connect any devices privately across any network, with or without a central server.

- **Zero pre-install** — WireGuard bundled via `boringtun` (no kernel module needed)
- **Works everywhere** — STUN-based NAT traversal + UDP-443 relay fallback for corporate firewalls
- **Serverless mode** — two devices pair with a QR invite code, no server required
- **Control plane mode** — Go API server + React dashboard for team/fleet management
- **Platforms** — Linux, Windows, macOS, iOS, Android

---

## Quick Start — Serverless (no server needed)

```bash
# Device A — generate invite
linklink invite create --name my-laptop
# prints a LINKLINK:v1:... code and QR

# Device B — accept invite
linklink invite accept LINKLINK:v1:<code>

# Both devices — bring up the tunnel (requires root or CAP_NET_ADMIN)
sudo linklink up
ping 10.44.0.2
```

---

## Quick Start — Control Plane (self-hosted)

```bash
# 1. Copy and fill in secrets
cp deploy/.env.example deploy/.env
$EDITOR deploy/.env        # set POSTGRES_PASSWORD and JWT_SECRET

# 2. Start the stack
cd deploy
docker compose -f docker-compose.dev.yml up -d

# 3. On each device
linklink login --server http://YOUR_SERVER:8080
linklink register --name my-device
sudo linklink up
```

The web dashboard is available at `http://YOUR_SERVER:5173`.

---

## Updating the Stack

After a new image is pushed to Docker Hub (on every merge to `main`), pull and restart:

```bash
cd deploy

# Pull latest images
docker compose -f docker-compose.prod.yml --env-file .env pull

# Restart with zero-downtime (recreates only changed containers)
docker compose -f docker-compose.prod.yml --env-file .env up -d

# Verify all containers are healthy
docker compose -f docker-compose.prod.yml ps

# Check server logs
docker compose -f docker-compose.prod.yml logs server --tail 50
```

To pin a specific release instead of always pulling `main`:

```bash
# In deploy/.env, set:
# IMAGE_TAG=v1.2.3
# Then in docker-compose.prod.yml change :main → :${IMAGE_TAG}
docker compose -f docker-compose.prod.yml --env-file .env pull
docker compose -f docker-compose.prod.yml --env-file .env up -d
```

---

## Using Podman Instead of Docker

All commands work with Podman. The only differences:

### Install podman-compose

```bash
# Fedora / RHEL / CentOS
sudo dnf install podman podman-compose

# Ubuntu / Debian
sudo apt install podman
pip3 install podman-compose

# macOS
brew install podman podman-compose
podman machine init && podman machine start
```

### Run the stack

Replace `docker` with `podman` and `docker compose` with `podman-compose`:

```bash
cd deploy

# Start
podman-compose -f docker-compose.prod.yml --env-file .env up -d

# Pull latest images
podman-compose -f docker-compose.prod.yml --env-file .env pull

# Restart after update
podman-compose -f docker-compose.prod.yml --env-file .env up -d

# View logs
podman-compose -f docker-compose.prod.yml logs server --tail 50

# Stop
podman-compose -f docker-compose.prod.yml down
```

### Rootless Podman (recommended)

Podman runs containers as your own user by default — no root daemon required:

```bash
# No sudo needed
podman-compose -f docker-compose.prod.yml --env-file .env up -d

# Check running containers
podman ps

# Persistent across reboots (generate systemd units)
podman generate systemd --new --name linklink-server > ~/.config/systemd/user/linklink-server.service
systemctl --user enable --now linklink-server
```

### Podman vs Docker — key differences

| | Docker | Podman |
|---|---|---|
| Daemon | Required (`dockerd`) | None (daemonless) |
| Default user | root | your user (rootless) |
| Compose | `docker compose` (plugin) | `podman-compose` (separate install) |
| Socket path | `/var/run/docker.sock` | `/run/user/$UID/podman/podman.sock` |
| Compatibility | — | Drop-in for most use cases |

---

## CI / CD

| Event | Action |
|---|---|
| Push to any branch / PR | Rust tests + Go unit tests + Frontend build |
| Changes to `.md` files only | Skipped — no build triggered |
| Merge to `main` | Build and push `linklink-server:main` + `linklink-frontend:main` to Docker Hub |
| Git tag `v*` | Same, plus versioned tags on Docker Hub |

Required GitHub secrets: `DOCKERHUB_USERNAME`, `DOCKERHUB_TOKEN`.

---

## Repository Layout

```
core/       Rust library — WireGuard config, STUN, invite codes, gossip, keystore
agent/      Rust CLI agent — linklink binary for Linux/macOS/Windows
mobile/     iOS (Swift + UniFFI) and Android (Kotlin + UniFFI) apps
server/     Go control plane — REST API, device management, auth, hub sync
frontend/   React + TypeScript + Tailwind web dashboard
deploy/     Docker Compose (dev + prod), systemd unit, install scripts
docs/       Architecture, security model, API reference
```

---

## Development

### Prerequisites

| Tool | Version |
|---|---|
| Rust | 1.78+ |
| Go | 1.25+ |
| Node | 20+ |
| Docker + Compose **or** Podman + podman-compose | any recent |

### Run tests

```bash
# Rust — no root, no DB needed
cargo test --workspace

# Go unit tests — no DB needed
cd server && go test ./internal/auth/... ./internal/devices/...

# Go all tests — requires running Postgres
cd server && go test ./...

# Frontend type check + build
cd frontend && npm ci && npm run build
```

### Local dev stack

```bash
cp deploy/.env.example deploy/.env && $EDITOR deploy/.env

# Docker
cd deploy && docker compose -f docker-compose.dev.yml up -d

# Podman
cd deploy && podman-compose -f docker-compose.dev.yml up -d

# API:      http://localhost:8080
# Frontend: http://localhost:5173
```

### Build static Linux agent binary

```bash
cargo build -p linklink-agent \
  --target x86_64-unknown-linux-musl \
  --release
# → target/x86_64-unknown-linux-musl/release/linklink
```

### Environment variables for testing agent locally

```bash
export LINKLINK_KEY=/tmp/ll.key
export LINKLINK_WG_CONFIG=/tmp/ll.conf
export LINKLINK_PEERS=/tmp/ll-peers.json
```

---

## Implementation Phases

| Phase | Status |
|---|---|
| 1 — Rust core library | ✅ complete |
| 2 — Serverless agent CLI | ✅ complete |
| 3 — Go server foundation | ✅ complete |
| 4 — Authentication (OAuth2 device flow) | 🔲 next |
| 5 — Device registration & management | 🔲 planned |
| 6 — Hub integration & wg syncconf | 🔲 planned |
| 7 — NAT traversal & relay fallback | 🔲 planned |
| 8 — Web UI | 🔲 planned |
| 9 — Audit logging | 🔲 planned |
| 10 — ACL system | 🔲 planned |
| 11 — Mobile (UniFFI) | 🔲 planned |
| 12 — Hardening & production | 🔲 planned |

---

## Security

- Private keys are generated on-device and **never transmitted**
- Tunnel operates independently of the control plane — network stays up if server goes down
- All secrets (DB password, JWT secret) must be set via environment variables; the stack refuses to start otherwise
- Config files and key files are written with `0600` permissions
- See [`docs/security.md`](docs/security.md) for the full security model
