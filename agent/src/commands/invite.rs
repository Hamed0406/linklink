use linklink_core::{
    invite::{decode_invitation, encode_invitation, Invitation},
    keystore::{load_private_key, save_private_key, KeyPair},
    stun,
};

use super::{config_path, peers_cache_path, private_key_path, wg_config_path};

pub async fn cmd_invite_create(name: String) -> anyhow::Result<()> {
    // Load or generate keypair
    let kp = load_or_generate_keypair()?;
    let public_key_b64 = kp.public_key_base64();

    // Discover external endpoint via STUN
    let config = load_config_or_default();
    let endpoint = match stun::discover_with_fallback(&config.stun_servers) {
        Ok(addr) => {
            println!("Discovered external endpoint: {addr}");
            Some(addr.to_string())
        }
        Err(e) => {
            eprintln!("Warning: STUN discovery failed ({e}), invite will have no endpoint");
            None
        }
    };

    let invite = Invitation::new(name, public_key_b64, endpoint);
    let code = encode_invitation(&invite)?;

    println!("\n=== Invitation created ===\n");
    println!("Share this code with the other device:\n");
    println!("{code}\n");

    // Print QR code to terminal
    print_qr(&code);

    println!("On the other device, run:");
    println!("  linklink invite accept {code}\n");

    Ok(())
}

pub async fn cmd_invite_accept(code: String) -> anyhow::Result<()> {
    let invite = decode_invitation(&code)?;
    println!(
        "Accepting invite from '{}' (network: {})",
        invite.name, invite.nid
    );

    // Generate our own keypair
    let kp = load_or_generate_keypair()?;
    let our_public_key = kp.public_key_base64();
    let our_private_key_b64 = kp.private_key_base64();

    // Discover our own endpoint
    let config = load_config_or_default();
    let our_endpoint = stun::discover_with_fallback(&config.stun_servers)
        .ok()
        .map(|a| a.to_string());

    // Build WireGuard config with the inviter as a peer
    let wg_cfg = linklink_core::wireguard::WgConfig {
        address: "10.44.0.2/32".to_string(), // placeholder; real IP assigned by control plane
        private_key: our_private_key_b64,
        listen_port: Some(51820),
        dns: None,
        peers: vec![linklink_core::wireguard::WgPeer {
            comment: Some(invite.name.clone()),
            public_key: invite.pk.clone(),
            endpoint: invite.ep.clone(),
            allowed_ips: vec!["10.44.0.0/24".to_string()],
            persistent_keepalive: Some(25),
        }],
    };

    let wg_path = wg_config_path();
    linklink_core::wireguard::write_wg_config(&wg_path, &wg_cfg)?;

    // Save peer cache
    let peer_info = linklink_core::gossip::PeerInfo {
        public_key: invite.pk.clone(),
        tunnel_ip: "unknown".to_string(),
        external_endpoint: invite.ep.clone(),
        last_seen: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };
    save_peers_cache(&[peer_info])?;

    println!("\nPeer '{}' added.", invite.name);
    println!("Our public key: {our_public_key}");
    if let Some(ep) = our_endpoint {
        println!("Our endpoint:   {ep}");
    }
    println!("\nRun `linklink up` to start the tunnel.");
    println!(
        "Share your public key with '{}' so they can add you as a peer.",
        invite.name
    );
    println!("  Their command: linklink invite accept <code-from-you>");

    Ok(())
}

// ── helpers ─────────────────────────────────────────────────────────────────

fn load_or_generate_keypair() -> anyhow::Result<KeyPair> {
    let key_path = private_key_path();
    if key_path.exists() {
        let bytes = load_private_key(&key_path)?;
        Ok(KeyPair::from_private_bytes(*bytes))
    } else {
        let kp = KeyPair::generate();
        save_private_key(&key_path, &kp.private_key)?;
        println!("Generated new keypair. Public key: {}", kp.public_key_base64());
        Ok(kp)
    }
}

fn load_config_or_default() -> linklink_core::config::AgentConfig {
    let path = config_path();
    linklink_core::config::load_config(&path).unwrap_or_default()
}

fn save_peers_cache(peers: &[linklink_core::gossip::PeerInfo]) -> anyhow::Result<()> {
    let path = peers_cache_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(peers)?;
    std::fs::write(&path, json)?;
    Ok(())
}

fn print_qr(code: &str) {
    use qrcode::{render::unicode, QrCode};
    match QrCode::new(code.as_bytes()) {
        Ok(qr) => {
            let image = qr
                .render::<unicode::Dense1x2>()
                .dark_color(unicode::Dense1x2::Dark)
                .light_color(unicode::Dense1x2::Light)
                .build();
            println!("{image}");
        }
        Err(_) => {
            println!("(QR code generation failed — use the text code above)");
        }
    }
}
