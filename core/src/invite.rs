use crate::error::{Error, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const PREFIX: &str = "LINKLINK";
const VERSION: u8 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Invitation {
    /// Schema version
    pub v: u8,
    /// Network ID (UUID)
    pub nid: String,
    /// Human-readable name of the inviting device
    pub name: String,
    /// WireGuard public key of the inviting device (base64)
    pub pk: String,
    /// Last-known external endpoint of the inviting device (IP:port), optional
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ep: Option<String>,
    /// One-time pre-shared authorization token (hex, 32 chars)
    pub token: String,
}

impl Invitation {
    pub fn new(name: String, public_key: String, endpoint: Option<String>) -> Self {
        Invitation {
            v: VERSION,
            nid: Uuid::new_v4().to_string(),
            name,
            pk: public_key,
            ep: endpoint,
            token: random_token(),
        }
    }
}

/// Encodes an invitation to `LINKLINK:v1:<base64url_json>`.
pub fn encode_invitation(inv: &Invitation) -> Result<String> {
    let json =
        serde_json::to_string(inv).map_err(|e| Error::Serialization(e.to_string()))?;
    let b64 = URL_SAFE_NO_PAD.encode(json.as_bytes());
    Ok(format!("{}:v{}:{}", PREFIX, inv.v, b64))
}

/// Decodes an invitation from `LINKLINK:v1:<base64url_json>`.
pub fn decode_invitation(code: &str) -> Result<Invitation> {
    let parts: Vec<&str> = code.splitn(3, ':').collect();
    if parts.len() != 3 {
        return Err(Error::Invite("Invalid invite format".into()));
    }
    if parts[0] != PREFIX {
        return Err(Error::Invite(format!(
            "Unknown invite prefix '{}', expected '{}'",
            parts[0], PREFIX
        )));
    }
    let version_str = parts[1];
    if version_str != format!("v{}", VERSION) {
        return Err(Error::Invite(format!(
            "Unsupported invite version '{}', expected 'v{}'",
            version_str, VERSION
        )));
    }
    let json_bytes = URL_SAFE_NO_PAD
        .decode(parts[2])
        .map_err(|e| Error::Invite(format!("Invalid base64: {e}")))?;
    let inv: Invitation = serde_json::from_slice(&json_bytes)
        .map_err(|e| Error::Invite(format!("Invalid invite JSON: {e}")))?;
    Ok(inv)
}

fn random_token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_invite() -> Invitation {
        Invitation {
            v: 1,
            nid: "550e8400-e29b-41d4-a716-446655440000".into(),
            name: "hamed-laptop".into(),
            pk: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".into(),
            ep: Some("1.2.3.4:51820".into()),
            token: "deadbeefdeadbeef0123456789abcdef".into(),
        }
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let inv = sample_invite();
        let code = encode_invitation(&inv).unwrap();
        let decoded = decode_invitation(&code).unwrap();
        assert_eq!(inv, decoded);
    }

    #[test]
    fn test_encode_starts_with_prefix() {
        let code = encode_invitation(&sample_invite()).unwrap();
        assert!(code.starts_with("LINKLINK:v1:"));
    }

    #[test]
    fn test_decode_wrong_prefix_rejected() {
        assert!(decode_invitation("BADPREFIX:v1:abc").is_err());
    }

    #[test]
    fn test_decode_wrong_version_rejected() {
        let inv = sample_invite();
        let code = encode_invitation(&inv).unwrap();
        // Replace version
        let mangled = code.replacen("LINKLINK:v1:", "LINKLINK:v99:", 1);
        let err = decode_invitation(&mangled).unwrap_err();
        assert!(err.to_string().contains("version"));
    }

    #[test]
    fn test_decode_truncated_base64_rejected() {
        assert!(decode_invitation("LINKLINK:v1:!!!!!").is_err());
    }

    #[test]
    fn test_decode_missing_colon_rejected() {
        assert!(decode_invitation("LINKLINK_v1_abc").is_err());
    }

    #[test]
    fn test_tokens_are_random() {
        let inv1 = Invitation::new("dev1".into(), "pk==".into(), None);
        let inv2 = Invitation::new("dev2".into(), "pk==".into(), None);
        assert_ne!(inv1.token, inv2.token);
    }

    #[test]
    fn test_network_ids_are_unique() {
        let inv1 = Invitation::new("dev1".into(), "pk==".into(), None);
        let inv2 = Invitation::new("dev2".into(), "pk==".into(), None);
        assert_ne!(inv1.nid, inv2.nid);
    }

    #[test]
    fn test_optional_endpoint_omitted_in_json() {
        let inv = Invitation::new("dev".into(), "pk==".into(), None);
        let code = encode_invitation(&inv).unwrap();
        let b64 = code.splitn(3, ':').nth(2).unwrap();
        let json_bytes = URL_SAFE_NO_PAD.decode(b64).unwrap();
        let json = String::from_utf8(json_bytes).unwrap();
        assert!(!json.contains("\"ep\""));
    }
}
