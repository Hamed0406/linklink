use crate::error::{Error, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::path::Path;
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::Zeroizing;

/// A WireGuard keypair. Private key is wrapped in Zeroizing so it is
/// zeroed from memory on drop.
pub struct KeyPair {
    pub private_key: Zeroizing<[u8; 32]>,
    pub public_key: [u8; 32],
}

impl KeyPair {
    pub fn generate() -> Self {
        let private = StaticSecret::random_from_rng(rand::rngs::OsRng);
        let public = PublicKey::from(&private);
        KeyPair {
            private_key: Zeroizing::new(*private.as_bytes()),
            public_key: *public.as_bytes(),
        }
    }

    pub fn from_private_bytes(bytes: [u8; 32]) -> Self {
        let private = StaticSecret::from(bytes);
        let public = PublicKey::from(&private);
        KeyPair {
            private_key: Zeroizing::new(bytes),
            public_key: *public.as_bytes(),
        }
    }

    pub fn private_key_base64(&self) -> String {
        STANDARD.encode(*self.private_key)
    }

    pub fn public_key_base64(&self) -> String {
        STANDARD.encode(self.public_key)
    }
}

/// Saves a private key to disk as base64. Sets file permissions to 0600 on Unix.
/// The parent directory is created if it does not exist.
pub fn save_private_key(path: &Path, key_bytes: &[u8; 32]) -> Result<()> {
    let encoded = STANDARD.encode(key_bytes);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, encoded.as_bytes())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(path, perms)?;
    }

    Ok(())
}

/// Loads a private key from a base64 file. Rejects files that are not exactly 32 bytes.
pub fn load_private_key(path: &Path) -> Result<Zeroizing<[u8; 32]>> {
    let encoded = std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            Error::InvalidKey(format!("Key file not found: {}", path.display()))
        } else {
            Error::Io(e)
        }
    })?;

    let bytes = STANDARD
        .decode(encoded.trim())
        .map_err(|e| Error::InvalidKey(format!("Invalid base64 in key file: {e}")))?;

    if bytes.len() != 32 {
        return Err(Error::InvalidKey(format!(
            "Expected 32 bytes, got {}",
            bytes.len()
        )));
    }

    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(Zeroizing::new(arr))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_keypair_is_valid() {
        let kp = KeyPair::generate();
        // Re-derive public key from private, must match stored public key
        let private = StaticSecret::from(*kp.private_key);
        let derived = PublicKey::from(&private);
        assert_eq!(*derived.as_bytes(), kp.public_key);
    }

    #[test]
    fn test_private_key_not_same_as_public() {
        let kp = KeyPair::generate();
        assert_ne!(*kp.private_key, kp.public_key);
    }

    #[test]
    fn test_keypairs_are_unique() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();
        assert_ne!(kp1.public_key, kp2.public_key);
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("private.key");
        let kp = KeyPair::generate();
        save_private_key(&path, &kp.private_key).unwrap();
        let loaded = load_private_key(&path).unwrap();
        assert_eq!(*kp.private_key, *loaded);
    }

    #[cfg(unix)]
    #[test]
    fn test_file_permissions_are_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let path = dir.path().join("private.key");
        let kp = KeyPair::generate();
        save_private_key(&path, &kp.private_key).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "Expected 0600, got {:o}", mode & 0o777);
    }

    #[test]
    fn test_load_missing_file_returns_err() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent.key");
        assert!(load_private_key(&path).is_err());
    }

    #[test]
    fn test_load_invalid_base64_returns_err() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad.key");
        std::fs::write(&path, b"not-valid-base64!!!").unwrap();
        assert!(load_private_key(&path).is_err());
    }

    #[test]
    fn test_load_wrong_length_returns_err() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("short.key");
        // 16 bytes base64 — too short
        let encoded = STANDARD.encode([0u8; 16]);
        std::fs::write(&path, encoded.as_bytes()).unwrap();
        assert!(load_private_key(&path).is_err());
    }

    #[test]
    fn test_base64_encoding_length() {
        let kp = KeyPair::generate();
        // 32 bytes base64-encoded = 44 chars (with padding)
        assert_eq!(kp.private_key_base64().len(), 44);
        assert_eq!(kp.public_key_base64().len(), 44);
    }

    #[test]
    fn test_create_parent_dirs_on_save() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nested/dirs/private.key");
        let kp = KeyPair::generate();
        save_private_key(&path, &kp.private_key).unwrap();
        assert!(path.exists());
    }
}
