#!/usr/bin/env bash
# linklink agent installer for Linux
# Usage: curl -fsSL https://get.linklink.dev | sudo bash
# Or:    sudo bash install-agent.sh
set -euo pipefail

BINARY_URL="${LINKLINK_BINARY_URL:-https://github.com/linklink/linklink/releases/latest/download/linklink-linux-amd64}"
INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/etc/linklink"
DATA_DIR="/var/lib/linklink"
LOG_DIR="/var/log/linklink"
SERVICE_USER="linklink"

need_root() {
  if [ "$(id -u)" -ne 0 ]; then
    echo "Error: this script must be run as root (use sudo)" >&2
    exit 1
  fi
}

install_binary() {
  echo "Downloading linklink agent..."
  curl -fsSL "$BINARY_URL" -o "$INSTALL_DIR/linklink"
  chmod 755 "$INSTALL_DIR/linklink"
  echo "Installed: $INSTALL_DIR/linklink"
}

create_user() {
  if ! id "$SERVICE_USER" &>/dev/null; then
    useradd --system --no-create-home --shell /usr/sbin/nologin "$SERVICE_USER"
    echo "Created system user: $SERVICE_USER"
  fi
}

create_dirs() {
  mkdir -p "$CONFIG_DIR" "$DATA_DIR" "$LOG_DIR"
  chown -R "$SERVICE_USER:$SERVICE_USER" "$DATA_DIR" "$LOG_DIR"
  chmod 750 "$CONFIG_DIR" "$DATA_DIR"
}

install_service() {
  local svc_src
  svc_src="$(dirname "$0")/../systemd/linklink-agent.service"
  if [ -f "$svc_src" ]; then
    cp "$svc_src" /etc/systemd/system/linklink-agent.service
  else
    # Inline service file if not found alongside script
    cat > /etc/systemd/system/linklink-agent.service <<'EOF'
[Unit]
Description=linklink secure mesh tunnel agent
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=linklink
Group=linklink
ExecStart=/usr/local/bin/linklink up
ExecStop=/usr/local/bin/linklink down
Restart=on-failure
RestartSec=5s
AmbientCapabilities=CAP_NET_ADMIN
CapabilityBoundingSet=CAP_NET_ADMIN
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=strict
ReadWritePaths=/etc/linklink /etc/wireguard /var/lib/linklink /var/log/linklink
StandardOutput=journal
StandardError=journal
SyslogIdentifier=linklink

[Install]
WantedBy=multi-user.target
EOF
  fi

  systemctl daemon-reload
  systemctl enable linklink-agent
  echo "Service installed and enabled."
}

main() {
  need_root
  create_user
  create_dirs
  install_binary
  install_service

  echo ""
  echo "linklink agent installed successfully."
  echo ""
  echo "Next steps:"
  echo "  1. Log in:     linklink login"
  echo "  2. Register:   linklink register --name \$(hostname)"
  echo "  3. Start:      systemctl start linklink-agent"
  echo "  4. Status:     linklink status"
}

main "$@"
