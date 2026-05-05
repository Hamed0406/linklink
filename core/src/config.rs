use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RelayEndpoint {
    pub public_key: String,
    pub endpoint_primary: String,
    pub endpoint_fallback: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PeerConfig {
    pub name: String,
    pub public_key: String,
    pub tunnel_ip: String,
    pub external_endpoint: Option<String>,
    pub allowed_ips: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentConfig {
    pub server_url: Option<String>,
    pub device_id: Option<String>,
    pub tunnel_ip: Option<String>,
    #[serde(default = "default_interface_name")]
    pub interface_name: String,
    #[serde(default = "default_stun_servers")]
    pub stun_servers: Vec<String>,
    #[serde(default)]
    pub relay_endpoints: Vec<RelayEndpoint>,
    #[serde(default)]
    pub peers: Vec<PeerConfig>,
    #[serde(default)]
    pub config_version: u64,
}

fn default_interface_name() -> String {
    "linklink".to_string()
}

fn default_stun_servers() -> Vec<String> {
    vec![
        "stun.l.google.com:19302".to_string(),
        "stun.cloudflare.com:3478".to_string(),
    ]
}

impl Default for AgentConfig {
    fn default() -> Self {
        AgentConfig {
            server_url: None,
            device_id: None,
            tunnel_ip: None,
            interface_name: default_interface_name(),
            stun_servers: default_stun_servers(),
            relay_endpoints: vec![],
            peers: vec![],
            config_version: 0,
        }
    }
}

pub fn save_config(path: &Path, config: &AgentConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content =
        toml::to_string_pretty(config).map_err(|e| Error::Config(e.to_string()))?;
    std::fs::write(path, content)?;
    Ok(())
}

pub fn load_config(path: &Path) -> Result<AgentConfig> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            Error::Config(format!("Config file not found: {}", path.display()))
        } else {
            Error::Io(e)
        }
    })?;
    toml::from_str(&content).map_err(|e| Error::Config(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_config() -> AgentConfig {
        AgentConfig {
            server_url: Some("https://ctrl.example.com".into()),
            device_id: Some("abc-123".into()),
            tunnel_ip: Some("10.44.0.2".into()),
            interface_name: "linklink".into(),
            stun_servers: vec!["stun.l.google.com:19302".into()],
            relay_endpoints: vec![RelayEndpoint {
                public_key: "key==".into(),
                endpoint_primary: "1.2.3.4:51820".into(),
                endpoint_fallback: "1.2.3.4:443".into(),
            }],
            peers: vec![],
            config_version: 7,
        }
    }

    #[test]
    fn test_config_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let cfg = sample_config();
        save_config(&path, &cfg).unwrap();
        let loaded = load_config(&path).unwrap();
        assert_eq!(cfg, loaded);
    }

    #[test]
    fn test_missing_file_returns_err() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent.toml");
        assert!(load_config(&path).is_err());
    }

    #[test]
    fn test_malformed_toml_returns_err() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad.toml");
        std::fs::write(&path, b"[[[invalid toml").unwrap();
        assert!(load_config(&path).is_err());
    }

    #[test]
    fn test_defaults_applied_on_minimal_config() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("minimal.toml");
        // Only required fields — omit interface_name and stun_servers
        std::fs::write(&path, b"").unwrap();
        let cfg = load_config(&path).unwrap();
        assert_eq!(cfg.interface_name, "linklink");
        assert!(!cfg.stun_servers.is_empty());
    }

    #[test]
    fn test_create_parent_dirs_on_save() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nested/config.toml");
        save_config(&path, &AgentConfig::default()).unwrap();
        assert!(path.exists());
    }
}
