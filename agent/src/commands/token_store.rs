use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedToken {
    pub access_token: String,
    pub refresh_token: String,
    pub server_url: String,
}

pub fn save_token(path: &Path, token: &SavedToken) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(token)?;
    std::fs::write(path, json)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(path, perms)?;
    }
    Ok(())
}

pub fn load_token(path: &Path) -> Result<SavedToken> {
    let json = std::fs::read_to_string(path)
        .with_context(|| format!("token file not found at {}\nRun `linklink login` first.", path.display()))?;
    serde_json::from_str(&json).context("token file is corrupt — run `linklink login` again")
}
