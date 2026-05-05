# Security Model

## Core Principles

1. **Private keys never leave the device.** Keys are generated locally using `x25519-dalek` with `OsRng`. The control plane server never sees a private key.
2. **Tunnel independence.** WireGuard sessions continue to function even if the control plane server is unreachable. The server is a management plane, not a data plane.
3. **Zeroized key memory.** Private key bytes are wrapped in `Zeroizing<[u8; 32]>` — the memory is zeroed on drop.
4. **Restricted file permissions.** All key files and WireGuard config files are written with `0600` (owner read/write only).

## Authentication

- **Device flow (OAuth2 RFC 8628)** — headless CLI login; user activates on any browser without typing a password into the CLI.
- **Access tokens** — HS256 JWT, 1-hour TTL, signed with `JWT_SECRET`.
- **Refresh tokens** — 30-day TTL; only the SHA-256 hash is stored in the database (plaintext never persisted).
- **Hub API keys** — bcrypt-hashed; used for hub→server authentication only.

## Secrets Management

| Secret | Where set | How stored |
|---|---|---|
| `JWT_SECRET` | Environment variable | Never persisted |
| `POSTGRES_PASSWORD` | Environment variable | Never in source |
| Hub API key | Generated at registration | bcrypt hash in DB |
| Refresh tokens | Issued by server | SHA-256 hash in DB |
| Device private key | Generated on-device | `0600` file, never sent |

The Docker Compose stack uses `${VAR:?error}` syntax — it will refuse to start if `POSTGRES_PASSWORD` or `JWT_SECRET` are not set.

## Network

- **WireGuard** — all tunnel traffic is encrypted end-to-end using WireGuard's Noise protocol.
- **NAT traversal** — STUN (RFC 5389) is used for external endpoint discovery; UDP hole punching for direct P2P connections.
- **Relay fallback** — a hub VPS relays traffic when direct P2P fails. Port 443 UDP is supported via `iptables PREROUTING` redirect for environments that block non-standard UDP ports.
- **`wg syncconf`** — peer list updates are applied non-disruptively; existing sessions are not dropped.

## Input Validation

- WireGuard interface names: validated against `^[a-zA-Z0-9_-]{1,15}$` before passing to system commands (prevents shell injection).
- System commands use `Command::new()` with explicit argument arrays — no shell interpolation.
- Device names: validated server-side before DB insertion.
- Public keys: validated as 32-byte base64 before use.
- JWT tokens and refresh tokens: format-validated before writing to browser localStorage.

## What Is Not in Scope (Yet)

- mTLS between agent and server (planned for Phase 12)
- Certificate pinning on mobile (planned for Phase 11)
- ACL enforcement on the hub (planned for Phase 10)
