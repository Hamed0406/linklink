use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PeerInfo {
    pub public_key: String,
    pub tunnel_ip: String,
    pub external_endpoint: Option<String>,
    /// Unix timestamp (seconds) when this endpoint was last confirmed
    pub last_seen: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipMessage {
    pub sender_public_key: String,
    pub peers: Vec<PeerInfo>,
    pub timestamp: u64,
}

impl GossipMessage {
    pub fn new(sender_public_key: String, peers: Vec<PeerInfo>) -> Self {
        GossipMessage {
            sender_public_key,
            peers,
            timestamp: now_secs(),
        }
    }
}

pub fn encode_gossip(msg: &GossipMessage) -> crate::Result<Vec<u8>> {
    serde_json::to_vec(msg)
        .map_err(|e| crate::Error::Serialization(e.to_string()))
}

pub fn decode_gossip(bytes: &[u8]) -> crate::Result<GossipMessage> {
    serde_json::from_slice(bytes)
        .map_err(|e| crate::Error::Serialization(e.to_string()))
}

/// Merges an incoming peer list into the local list.
///
/// Rules:
/// - Incoming peers with a **newer** `last_seen` update the stored entry.
/// - Peers present in local but absent from incoming are **kept** (never removed via gossip).
/// - New peers not in local are added.
pub fn merge_peer_list(local: &mut Vec<PeerInfo>, incoming: &[PeerInfo]) {
    for incoming_peer in incoming {
        if let Some(local_peer) = local
            .iter_mut()
            .find(|p| p.public_key == incoming_peer.public_key)
        {
            if incoming_peer.last_seen > local_peer.last_seen {
                local_peer.external_endpoint = incoming_peer.external_endpoint.clone();
                local_peer.tunnel_ip = incoming_peer.tunnel_ip.clone();
                local_peer.last_seen = incoming_peer.last_seen;
            }
        } else {
            local.push(incoming_peer.clone());
        }
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer(key: &str, ep: &str, ts: u64) -> PeerInfo {
        PeerInfo {
            public_key: key.into(),
            tunnel_ip: "10.44.0.2".into(),
            external_endpoint: Some(ep.into()),
            last_seen: ts,
        }
    }

    #[test]
    fn test_gossip_roundtrip() {
        let msg = GossipMessage::new(
            "sender_key==".into(),
            vec![peer("peer_a", "1.2.3.4:51820", 100)],
        );
        let bytes = encode_gossip(&msg).unwrap();
        let decoded = decode_gossip(&bytes).unwrap();
        assert_eq!(decoded.sender_public_key, msg.sender_public_key);
        assert_eq!(decoded.peers.len(), 1);
    }

    #[test]
    fn test_merge_newer_endpoint_wins() {
        let mut local = vec![peer("key_a", "old_ep:1000", 50)];
        let incoming = vec![peer("key_a", "new_ep:2000", 100)];
        merge_peer_list(&mut local, &incoming);
        assert_eq!(local.len(), 1);
        assert_eq!(local[0].external_endpoint.as_deref(), Some("new_ep:2000"));
        assert_eq!(local[0].last_seen, 100);
    }

    #[test]
    fn test_merge_older_endpoint_ignored() {
        let mut local = vec![peer("key_a", "current_ep:9000", 200)];
        let incoming = vec![peer("key_a", "stale_ep:1000", 50)];
        merge_peer_list(&mut local, &incoming);
        assert_eq!(local[0].external_endpoint.as_deref(), Some("current_ep:9000"));
        assert_eq!(local[0].last_seen, 200);
    }

    #[test]
    fn test_merge_new_peer_added() {
        let mut local = vec![peer("key_a", "ep_a", 100)];
        let incoming = vec![peer("key_b", "ep_b", 100)];
        merge_peer_list(&mut local, &incoming);
        assert_eq!(local.len(), 2);
        assert!(local.iter().any(|p| p.public_key == "key_b"));
    }

    #[test]
    fn test_merge_does_not_remove_existing_peers() {
        let mut local = vec![peer("key_a", "ep_a", 100), peer("key_b", "ep_b", 100)];
        let incoming = vec![peer("key_a", "ep_a_new", 200)];
        merge_peer_list(&mut local, &incoming);
        // key_b must still be present even though it wasn't in incoming
        assert_eq!(local.len(), 2);
        assert!(local.iter().any(|p| p.public_key == "key_b"));
    }

    #[test]
    fn test_merge_empty_incoming_no_change() {
        let mut local = vec![peer("key_a", "ep_a", 100)];
        merge_peer_list(&mut local, &[]);
        assert_eq!(local.len(), 1);
    }

    #[test]
    fn test_decode_invalid_json_returns_err() {
        assert!(decode_gossip(b"not json").is_err());
    }
}
