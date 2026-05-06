use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::time::Duration;

use linklink_core::{
    keystore::{load_private_key, save_private_key, KeyPair},
    stun,
    config::{AgentConfig, save_config},
};

use super::{config_path, private_key_path, token_path, token_store::load_token};

#[derive(Deserialize)]
struct RegisteredDevice {
    id: String,
    tunnel_ip: String,
    status: String,
}

pub async fn cmd_register(name: String) -> Result<()> {
    // Load saved login token
    let token = load_token(&token_path())?;

    // Load or generate keypair
    let kp = load_or_generate_keypair()?;
    let public_key = kp.public_key_base64();

    // Discover external endpoint via STUN
    let default_stun = vec!["stun.l.google.com:19302".to_string()];
    let endpoint = stun::discover_with_fallback(&default_stun)
        .ok()
        .map(|a| a.to_string());
    if let Some(ref ep) = endpoint {
        println!("Discovered external endpoint: {ep}");
    }

    // Register with server
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()?;

    let body = serde_json::json!({
        "name": name,
        "public_key": public_key,
        "os": std::env::consts::OS,
        "hostname": hostname(),
        "external_endpoint": endpoint,
    });

    let resp = client
        .post(format!("{}/api/v1/devices/register", token.server_url))
        .bearer_auth(&token.access_token)
        .json(&body)
        .send()
        .await
        .context("could not reach server")?;

    if resp.status() == reqwest::StatusCode::CONFLICT {
        bail!("A device with this public key is already registered. Use `linklink reset` to generate a new keypair.");
    }
    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        bail!("Token expired. Run `linklink login` again.");
    }
    resp.error_for_status_ref().context("registration failed")?;

    let device: RegisteredDevice = resp.json().await?;

    // Save agent config
    let cfg = AgentConfig {
        server_url: Some(token.server_url.clone()),
        device_id: Some(device.id.clone()),
        interface_name: "linklink".to_string(),
        tunnel_ip: Some(device.tunnel_ip.clone()),
        stun_servers: default_stun,
        ..Default::default()
    };
    save_config(&config_path(), &cfg).context("failed to save config")?;

    println!("\nDevice '{}' registered.", name);
    println!("  Device ID:  {}", device.id);
    println!("  Tunnel IP:  {}", device.tunnel_ip);
    println!("  Status:     {}", device.status);
    println!("\nWaiting for admin approval in the dashboard.");
    println!("Once approved, run: linklink up");

    Ok(())
}

fn load_or_generate_keypair() -> Result<KeyPair> {
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

fn hostname() -> Option<String> {
    std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}
