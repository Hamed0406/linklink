# linklink

Secure mesh tunnel — connect any devices privately across any network.

- **No pre-installed WireGuard required** — bundled via `boringtun`
- **Works behind NAT, CGNAT, corporate firewalls** — STUN + relay fallback on port 443
- **Serverless mode** — two devices connect with a QR invite, no server needed
- **Platforms** — Linux, Windows, macOS, iOS, Android

---

## Quick Start — Serverless (no server needed)

```bash
# Device A
linklink invite create --name my-laptop
# → prints a code and QR

# Device B
linklink invite accept LINKLINK:v1:<code>
linklink up

# Device A
linklink up
ping 10.44.0.2   # or whatever tunnel IP was assigned
```

---

## Quick Start — Control Plane

```bash
# 1. Start the stack locally
cd deploy
docker compose -f docker-compose.dev.yml up -d

# 2. On each device
linklink login --server http://localhost:8080
linklink register --name my-device
linklink up
```

---

## Repository Layout

```
core/       Rust: WireGuard engine, STUN, invite, gossip (compiles to all platforms)
agent/      Rust: CLI agent for Linux/Windows/macOS
mobile/     iOS (Swift + UniFFI) and Android (Kotlin + UniFFI) apps
server/     Go: control plane API, device management, hub sync
frontend/   React + TypeScript + Tailwind: web dashboard
deploy/     Docker Compose, systemd, install scripts
docs/       Architecture, security, API reference
```

---

## Development

### Prerequisites

| Tool | Version |
|------|---------|
| Rust | 1.78+  |
| Go   | 1.22+  |
| Node | 20+    |
| Docker + Compose | any recent |

### Run locally

```bash
# Start postgres + server + frontend
cd deploy && docker compose -f docker-compose.dev.yml up -d

# Rust tests (no root needed)
cargo test --workspace

# Go tests (requires postgres)
cd server && go test ./...

# Frontend dev server
cd frontend && npm install && npm run dev
```

### Build the static Linux agent binary

```bash
cargo build -p linklink-agent \
  --target x86_64-unknown-linux-musl \
  --release
# → target/x86_64-unknown-linux-musl/release/linklink
```

---

## Implementation Phases

See [`implementation-plan.md`](implementation-plan.md) for the full step-by-step build plan with tests.

| Phase | Status |
|-------|--------|
| 1 — Rust core library | ✅ complete |
| 2 — Serverless agent CLI | ✅ complete |
| 3 — Go server foundation | ✅ complete |
| 4 — Authentication (device flow) | 🔲 next |
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

Private keys are generated on-device and never transmitted.
See [`docs/security.md`](docs/security.md) for the full security model.
