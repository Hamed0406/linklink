pub mod auth;
pub mod invite;
pub mod peers;
pub mod register;
pub mod relay;
pub mod status;
pub mod tunnel;

use std::path::PathBuf;

/// Returns the path to the agent config file.
pub fn config_path() -> PathBuf {
    if let Ok(p) = std::env::var("LINKLINK_CONFIG") {
        return PathBuf::from(p);
    }
    #[cfg(unix)]
    return PathBuf::from("/etc/linklink/config.toml");
    #[cfg(windows)]
    return dirs::data_local_dir()
        .unwrap_or_default()
        .join("linklink\\config.toml");
}

/// Returns the path where the private key is stored.
pub fn private_key_path() -> PathBuf {
    if let Ok(p) = std::env::var("LINKLINK_KEY") {
        return PathBuf::from(p);
    }
    #[cfg(unix)]
    return PathBuf::from("/etc/linklink/private.key");
    #[cfg(windows)]
    return dirs::data_local_dir()
        .unwrap_or_default()
        .join("linklink\\private.key");
}

/// Returns the path to the WireGuard config file.
pub fn wg_config_path() -> PathBuf {
    if let Ok(p) = std::env::var("LINKLINK_WG_CONFIG") {
        return PathBuf::from(p);
    }
    #[cfg(unix)]
    return PathBuf::from("/etc/wireguard/linklink.conf");
    #[cfg(windows)]
    return dirs::data_local_dir()
        .unwrap_or_default()
        .join("linklink\\wireguard\\linklink.conf");
}

/// Returns the path to the peer cache file.
pub fn peers_cache_path() -> PathBuf {
    if let Ok(p) = std::env::var("LINKLINK_PEERS") {
        return PathBuf::from(p);
    }
    #[cfg(unix)]
    return PathBuf::from("/var/lib/linklink/peers.json");
    #[cfg(windows)]
    return dirs::data_local_dir()
        .unwrap_or_default()
        .join("linklink\\peers.json");
}

/// Returns the path to the auth token file.
pub fn token_path() -> PathBuf {
    if let Ok(p) = std::env::var("LINKLINK_TOKEN") {
        return PathBuf::from(p);
    }
    #[cfg(unix)]
    return PathBuf::from("/var/lib/linklink/token.json");
    #[cfg(windows)]
    return dirs::data_local_dir()
        .unwrap_or_default()
        .join("linklink\\token.json");
}
