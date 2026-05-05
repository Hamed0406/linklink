#!/usr/bin/env bash
# linklink relay/hub VPS setup script
# Run once on a fresh Ubuntu/Debian VPS with a public IP.
# Usage: sudo bash setup-hub.sh --server https://ctrl.example.com
set -euo pipefail

CONTROL_SERVER=""

for arg in "$@"; do
  case $arg in
    --server=*) CONTROL_SERVER="${arg#*=}" ;;
    --server)   shift; CONTROL_SERVER="$1" ;;
    *) echo "Unknown argument: $arg" >&2; exit 1 ;;
  esac
done

need_root() {
  [ "$(id -u)" -eq 0 ] || { echo "Run as root"; exit 1; }
}

enable_ip_forwarding() {
  echo "Enabling IP forwarding..."
  cat >> /etc/sysctl.d/99-linklink.conf <<EOF
net.ipv4.ip_forward=1
net.ipv6.conf.all.forwarding=1
EOF
  sysctl -p /etc/sysctl.d/99-linklink.conf
}

configure_firewall() {
  echo "Configuring UFW firewall..."
  ufw allow 22/tcp    comment "SSH"
  ufw allow 51820/udp comment "WireGuard primary"
  ufw allow 443/udp   comment "WireGuard fallback (corporate NAT)"
  ufw allow 8080/tcp  comment "linklink control plane (if colocated)"
  ufw --force enable
}

configure_port_redirect() {
  echo "Setting up UDP 443 → 51820 redirect..."
  # Incoming UDP on 443 is redirected to WireGuard on 51820
  iptables  -t nat -A PREROUTING -p udp --dport 443 -j REDIRECT --to-port 51820
  ip6tables -t nat -A PREROUTING -p udp --dport 443 -j REDIRECT --to-port 51820

  # Persist across reboots
  apt-get install -y -q iptables-persistent
  netfilter-persistent save
  echo "iptables redirect saved."
}

install_wireguard() {
  echo "Installing WireGuard..."
  apt-get update -q
  apt-get install -y -q wireguard
}

install_linklink() {
  echo "Installing linklink agent..."
  curl -fsSL https://github.com/linklink/linklink/releases/latest/download/linklink-linux-amd64 \
    -o /usr/local/bin/linklink
  chmod 755 /usr/local/bin/linklink
}

init_hub() {
  echo "Initializing hub keypair..."
  linklink relay init

  if [ -n "$CONTROL_SERVER" ]; then
    echo "Registering hub with control plane: $CONTROL_SERVER"
    linklink relay register --server "$CONTROL_SERVER"
  else
    echo "Skipping registration (no --server provided)."
    echo "Run manually: linklink relay register --server <url>"
  fi
}

install_hub_service() {
  cat > /etc/systemd/system/linklink-hub.service <<'EOF'
[Unit]
Description=linklink hub relay agent
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/linklink relay enable
Restart=on-failure
RestartSec=10s
StandardOutput=journal
StandardError=journal
SyslogIdentifier=linklink-hub

[Install]
WantedBy=multi-user.target
EOF
  systemctl daemon-reload
  systemctl enable --now linklink-hub
}

main() {
  need_root
  enable_ip_forwarding
  configure_firewall
  configure_port_redirect
  install_wireguard
  install_linklink
  init_hub
  install_hub_service

  echo ""
  echo "Hub setup complete."
  echo "  WireGuard port:    51820/udp"
  echo "  Fallback port:     443/udp  (redirected to 51820)"
  echo "  Hub service:       systemctl status linklink-hub"
}

main "$@"
