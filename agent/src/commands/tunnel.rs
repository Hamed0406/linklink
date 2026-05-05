use linklink_core::{
    config::load_config,
    wireguard::{bring_down, bring_up},
};
use super::{config_path, wg_config_path};

pub async fn cmd_up() -> anyhow::Result<()> {
    let cfg_path = config_path();
    let cfg = load_config(&cfg_path)
        .map_err(|_| anyhow::anyhow!("No config found. Run `linklink invite accept` or `linklink register` first."))?;

    let wg_path = wg_config_path();
    if !wg_path.exists() {
        return Err(anyhow::anyhow!(
            "WireGuard config not found at {}. Run `linklink invite accept` first.",
            wg_path.display()
        ));
    }

    println!("Starting tunnel '{}'...", cfg.interface_name);
    bring_up(&cfg.interface_name, &wg_path)?;
    println!("Tunnel up.");
    if let Some(ip) = &cfg.tunnel_ip {
        println!("Tunnel IP: {ip}");
    }
    Ok(())
}

pub async fn cmd_down() -> anyhow::Result<()> {
    let cfg_path = config_path();
    let cfg = load_config(&cfg_path)
        .map_err(|_| anyhow::anyhow!("No config found."))?;

    let wg_path = wg_config_path();
    println!("Stopping tunnel '{}'...", cfg.interface_name);
    bring_down(&cfg.interface_name, &wg_path)?;
    println!("Tunnel down.");
    Ok(())
}

pub async fn cmd_reset() -> anyhow::Result<()> {
    use std::io::{self, Write};
    print!("This will delete all local keys and config. Are you sure? [y/N] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    if input.trim().to_lowercase() != "y" {
        println!("Aborted.");
        return Ok(());
    }

    let paths = [
        super::config_path(),
        super::private_key_path(),
        super::wg_config_path(),
        super::peers_cache_path(),
        super::token_path(),
    ];

    for path in &paths {
        if path.exists() {
            std::fs::remove_file(path)?;
            println!("Removed: {}", path.display());
        }
    }
    println!("Reset complete.");
    Ok(())
}
