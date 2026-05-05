use linklink_core::gossip::PeerInfo;
use std::time::{SystemTime, UNIX_EPOCH};

use super::peers_cache_path;

pub async fn cmd_peers() -> anyhow::Result<()> {
    let path = peers_cache_path();
    if !path.exists() {
        println!("No peers known yet. Use `linklink invite accept` to add peers.");
        return Ok(());
    }

    let json = std::fs::read_to_string(&path)?;
    let peers: Vec<PeerInfo> = serde_json::from_str(&json)?;

    if peers.is_empty() {
        println!("No peers in cache.");
        return Ok(());
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    println!("{:<48}  {:<18}  {:<22}  {}", "Public Key", "Tunnel IP", "Endpoint", "Status");
    println!("{}", "-".repeat(110));

    for peer in &peers {
        let status = match now.saturating_sub(peer.last_seen) {
            s if s < 180 => "online",
            s if s < 600 => "idle",
            _ => "offline",
        };
        let ep = peer.external_endpoint.as_deref().unwrap_or("-");
        println!(
            "{:<48}  {:<18}  {:<22}  {}",
            &peer.public_key, peer.tunnel_ip, ep, status
        );
    }

    Ok(())
}
