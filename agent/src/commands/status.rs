use linklink_core::{config::load_config, wireguard::parse_wg_dump};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::config_path;

pub async fn cmd_status() -> anyhow::Result<()> {
    let cfg = load_config(&config_path()).unwrap_or_default();
    let iface = &cfg.interface_name;

    // Check if interface exists via `ip link show`
    let link_output = Command::new("ip")
        .args(["link", "show", iface])
        .output();

    match link_output {
        Ok(o) if o.status.success() => {}
        _ => {
            println!("Tunnel: DOWN (interface '{}' not found)", iface);
            return Ok(());
        }
    }

    // Get WireGuard status via `wg show <iface> dump`
    let wg_output = Command::new("wg")
        .args(["show", iface, "dump"])
        .output();

    let dump = match wg_output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).into_owned(),
        _ => {
            println!("Tunnel: UP (but `wg show` failed — are you root?)");
            return Ok(());
        }
    };

    let status = parse_wg_dump(iface, &dump);

    println!("Tunnel:     UP");
    println!("Interface:  {iface}");
    if let Some(pk) = &status.public_key {
        println!("Public key: {pk}");
    }
    if let Some(port) = status.listen_port {
        println!("Port:       {port}");
    }
    if let Some(ip) = &cfg.tunnel_ip {
        println!("Tunnel IP:  {ip}");
    }

    if status.peers.is_empty() {
        println!("\nNo peers.");
    } else {
        println!("\nPeers ({}):", status.peers.len());
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        for peer in &status.peers {
            let conn_state = match peer.latest_handshake_secs {
                Some(ts) if now_secs.saturating_sub(ts) < 180 => "online",
                Some(_) => "idle",
                None => "never",
            };
            let handshake_str = match peer.latest_handshake_secs {
                Some(ts) => {
                    let ago = now_secs.saturating_sub(ts);
                    format_duration(Duration::from_secs(ago))
                }
                None => "never".to_string(),
            };

            println!(
                "  {} [{}]  last handshake: {}  rx: {}  tx: {}",
                &peer.public_key[..16],
                conn_state,
                handshake_str,
                format_bytes(peer.bytes_received),
                format_bytes(peer.bytes_sent),
            );
        }
    }

    Ok(())
}

fn format_duration(d: Duration) -> String {
    let s = d.as_secs();
    if s < 60 {
        format!("{s}s ago")
    } else if s < 3600 {
        format!("{}m ago", s / 60)
    } else {
        format!("{}h ago", s / 3600)
    }
}

fn format_bytes(n: u64) -> String {
    if n < 1024 {
        format!("{n}B")
    } else if n < 1024 * 1024 {
        format!("{:.1}KB", n as f64 / 1024.0)
    } else {
        format!("{:.1}MB", n as f64 / (1024.0 * 1024.0))
    }
}
