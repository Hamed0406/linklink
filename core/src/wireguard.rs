use crate::error::{Error, Result};
use std::fmt::Write as FmtWrite;
use std::path::Path;
use std::process::Command;

static INTERFACE_NAME_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();

fn interface_re() -> &'static regex::Regex {
    INTERFACE_NAME_RE.get_or_init(|| regex::Regex::new(r"^[a-zA-Z0-9_-]{1,15}$").unwrap())
}

/// Validates that an interface name is safe to pass to system commands.
pub fn validate_interface_name(name: &str) -> Result<()> {
    if interface_re().is_match(name) {
        Ok(())
    } else {
        Err(Error::InvalidInterfaceName(format!(
            "Interface name '{}' must match ^[a-zA-Z0-9_-]{{1,15}}$",
            name
        )))
    }
}

#[derive(Debug, Clone)]
pub struct WgPeer {
    pub comment: Option<String>,
    pub public_key: String,
    pub endpoint: Option<String>,
    pub allowed_ips: Vec<String>,
    pub persistent_keepalive: Option<u16>,
}

#[derive(Debug, Clone)]
pub struct WgConfig {
    /// Tunnel IP with prefix, e.g. "10.44.0.2/32"
    pub address: String,
    /// Base64 private key — only written locally; never sent to the server
    pub private_key: String,
    pub listen_port: Option<u16>,
    pub dns: Option<String>,
    pub peers: Vec<WgPeer>,
}

/// Renders a WireGuard config file (INI format).
/// The private key is included — this file is written locally by the agent only.
pub fn render_wg_config(cfg: &WgConfig) -> String {
    let mut out = String::new();
    writeln!(out, "[Interface]").unwrap();
    writeln!(out, "Address = {}", cfg.address).unwrap();
    writeln!(out, "PrivateKey = {}", cfg.private_key).unwrap();
    if let Some(port) = cfg.listen_port {
        writeln!(out, "ListenPort = {port}").unwrap();
    }
    if let Some(dns) = &cfg.dns {
        writeln!(out, "DNS = {dns}").unwrap();
    }
    for peer in &cfg.peers {
        writeln!(out, "\n[Peer]").unwrap();
        if let Some(comment) = &peer.comment {
            writeln!(out, "# {comment}").unwrap();
        }
        writeln!(out, "PublicKey = {}", peer.public_key).unwrap();
        if let Some(ep) = &peer.endpoint {
            writeln!(out, "Endpoint = {ep}").unwrap();
        }
        if !peer.allowed_ips.is_empty() {
            writeln!(out, "AllowedIPs = {}", peer.allowed_ips.join(", ")).unwrap();
        }
        if let Some(ka) = peer.persistent_keepalive {
            writeln!(out, "PersistentKeepalive = {ka}").unwrap();
        }
    }
    out
}

/// Writes the WireGuard config to disk and sets permissions to 0600.
pub fn write_wg_config(path: &Path, cfg: &WgConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = render_wg_config(cfg);
    std::fs::write(path, content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(path, perms)?;
    }
    Ok(())
}

/// Applies a new peer list non-disruptively using `wg syncconf`.
/// Does not drop existing sessions.
pub fn apply_syncconf(interface: &str, config_path: &Path) -> Result<()> {
    validate_interface_name(interface)?;
    let output = Command::new("wg")
        .arg("syncconf")
        .arg(interface)
        .arg(config_path)
        .output()?;
    if !output.status.success() {
        return Err(Error::CommandFailed {
            code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(())
}

/// Brings up a WireGuard interface via `wg-quick up`.
pub fn bring_up(interface: &str, config_path: &Path) -> Result<()> {
    validate_interface_name(interface)?;
    let output = Command::new("wg-quick")
        .arg("up")
        .arg(config_path)
        .output()?;
    if !output.status.success() {
        return Err(Error::CommandFailed {
            code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(())
}

/// Brings down a WireGuard interface via `wg-quick down`.
pub fn bring_down(interface: &str, config_path: &Path) -> Result<()> {
    validate_interface_name(interface)?;
    let output = Command::new("wg-quick")
        .arg("down")
        .arg(config_path)
        .output()?;
    if !output.status.success() {
        return Err(Error::CommandFailed {
            code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct PeerStatus {
    pub public_key: String,
    pub endpoint: Option<String>,
    pub latest_handshake_secs: Option<u64>,
    pub bytes_received: u64,
    pub bytes_sent: u64,
}

#[derive(Debug, Clone)]
pub struct TunnelStatus {
    pub interface: String,
    pub public_key: Option<String>,
    pub listen_port: Option<u16>,
    pub peers: Vec<PeerStatus>,
}

/// Parses the output of `wg show <interface> dump` (tab-separated).
pub fn parse_wg_dump(interface: &str, output: &str) -> TunnelStatus {
    let mut status = TunnelStatus {
        interface: interface.to_string(),
        public_key: None,
        listen_port: None,
        peers: vec![],
    };
    for (i, line) in output.lines().enumerate() {
        let cols: Vec<&str> = line.split('\t').collect();
        if i == 0 && cols.len() >= 3 {
            // interface line: private_key  public_key  listen_port  fwmark
            status.public_key = Some(cols[1].to_string());
            status.listen_port = cols[2].parse().ok();
        } else if cols.len() >= 7 {
            // peer line: pub_key endpoint allowed_ips latest_handshake rx tx persistent_ka
            let handshake = cols[3].parse::<u64>().ok().filter(|&v| v > 0);
            let rx = cols[4].parse().unwrap_or(0);
            let tx = cols[5].parse().unwrap_or(0);
            let endpoint = if cols[1] == "(none)" {
                None
            } else {
                Some(cols[1].to_string())
            };
            status.peers.push(PeerStatus {
                public_key: cols[0].to_string(),
                endpoint,
                latest_handshake_secs: handshake,
                bytes_received: rx,
                bytes_sent: tx,
            });
        }
    }
    status
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_interface_name_valid() {
        for name in &["linklink", "wg0", "my-vpn1", "test_iface", "a", "123456789012345"] {
            assert!(validate_interface_name(name).is_ok(), "Expected valid: {name}");
        }
    }

    #[test]
    fn test_interface_name_invalid() {
        for name in &[
            "",
            "wg 0",
            "../../etc",
            "a/b",
            "1234567890123456", // 16 chars
            "has;semicolon",
            "has$dollar",
            "has`backtick",
        ] {
            assert!(
                validate_interface_name(name).is_err(),
                "Expected invalid: {name}"
            );
        }
    }

    #[test]
    fn test_config_generation_interface_section() {
        let cfg = WgConfig {
            address: "10.44.0.2/32".into(),
            private_key: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".into(),
            listen_port: None,
            dns: Some("10.44.0.1".into()),
            peers: vec![],
        };
        let rendered = render_wg_config(&cfg);
        assert!(rendered.contains("[Interface]"));
        assert!(rendered.contains("Address = 10.44.0.2/32"));
        assert!(rendered.contains("PrivateKey = AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="));
        assert!(rendered.contains("DNS = 10.44.0.1"));
    }

    #[test]
    fn test_config_generation_peer_section() {
        let cfg = WgConfig {
            address: "10.44.0.2/32".into(),
            private_key: "pk==".into(),
            listen_port: None,
            dns: None,
            peers: vec![WgPeer {
                comment: Some("hub-relay".into()),
                public_key: "hubpub==".into(),
                endpoint: Some("5.6.7.8:51820".into()),
                allowed_ips: vec!["10.44.0.0/24".into()],
                persistent_keepalive: Some(25),
            }],
        };
        let rendered = render_wg_config(&cfg);
        assert!(rendered.contains("[Peer]"));
        assert!(rendered.contains("# hub-relay"));
        assert!(rendered.contains("PublicKey = hubpub=="));
        assert!(rendered.contains("Endpoint = 5.6.7.8:51820"));
        assert!(rendered.contains("AllowedIPs = 10.44.0.0/24"));
        assert!(rendered.contains("PersistentKeepalive = 25"));
    }

    #[test]
    fn test_write_config_creates_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("linklink.conf");
        let cfg = WgConfig {
            address: "10.44.0.2/32".into(),
            private_key: "pk==".into(),
            listen_port: None,
            dns: None,
            peers: vec![],
        };
        write_wg_config(&path, &cfg).unwrap();
        assert!(path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn test_write_config_permissions_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let path = dir.path().join("linklink.conf");
        let cfg = WgConfig {
            address: "10.44.0.2/32".into(),
            private_key: "pk==".into(),
            listen_port: None,
            dns: None,
            peers: vec![],
        };
        write_wg_config(&path, &cfg).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn test_parse_wg_dump_peers() {
        let dump = "\
privkey\tpubkey_iface\t51820\toff\n\
peer_pub1\t1.2.3.4:51820\t10.44.0.3/32\t1714000000\t102400\t204800\t25\n";
        let status = parse_wg_dump("linklink", dump);
        assert_eq!(status.public_key.as_deref(), Some("pubkey_iface"));
        assert_eq!(status.listen_port, Some(51820));
        assert_eq!(status.peers.len(), 1);
        let peer = &status.peers[0];
        assert_eq!(peer.public_key, "peer_pub1");
        assert_eq!(peer.endpoint.as_deref(), Some("1.2.3.4:51820"));
        assert_eq!(peer.latest_handshake_secs, Some(1714000000));
        assert_eq!(peer.bytes_received, 102400);
        assert_eq!(peer.bytes_sent, 204800);
    }

    #[test]
    fn test_parse_wg_dump_no_peers() {
        let dump = "privkey\tpubkey_iface\t51820\toff\n";
        let status = parse_wg_dump("linklink", dump);
        assert!(status.peers.is_empty());
    }

    #[test]
    fn test_parse_wg_dump_peer_no_handshake() {
        let dump = "\
privkey\tpubkey_iface\t51820\toff\n\
peer_pub1\t(none)\t10.44.0.3/32\t0\t0\t0\t0\n";
        let status = parse_wg_dump("linklink", dump);
        assert_eq!(status.peers[0].latest_handshake_secs, None);
        assert_eq!(status.peers[0].endpoint, None);
    }
}
