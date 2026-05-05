use crate::error::{Error, Result};
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

const MAGIC_COOKIE: u32 = 0x2112_A442;
const BINDING_REQUEST: u16 = 0x0001;
const BINDING_RESPONSE: u16 = 0x0101;
const ATTR_MAPPED_ADDRESS: u16 = 0x0001;
const ATTR_XOR_MAPPED_ADDRESS: u16 = 0x0020;

/// Sends a STUN Binding Request to `server` and returns our external `SocketAddr`.
pub fn discover_external_endpoint(server: &str) -> Result<SocketAddr> {
    let sock = UdpSocket::bind("0.0.0.0:0")?;
    sock.set_read_timeout(Some(Duration::from_secs(3)))?;

    let server_addr: SocketAddr = server
        .parse()
        .or_else(|_| resolve_stun_addr(server))
        .map_err(|_| Error::Stun(format!("Cannot resolve STUN server: {server}")))?;

    let tx_id = random_transaction_id();
    let request = build_binding_request(&tx_id);
    sock.send_to(&request, server_addr)?;

    let mut buf = [0u8; 512];
    let (n, _) = sock
        .recv_from(&mut buf)
        .map_err(|_| Error::Timeout)?;

    parse_binding_response(&buf[..n], &tx_id)
}

/// Tries each server in order, returns first success.
pub fn discover_with_fallback(servers: &[String]) -> Result<SocketAddr> {
    let mut last_err = Error::Stun("No STUN servers provided".into());
    for server in servers {
        match discover_external_endpoint(server) {
            Ok(addr) => return Ok(addr),
            Err(e) => last_err = e,
        }
    }
    Err(last_err)
}

fn resolve_stun_addr(host_port: &str) -> std::result::Result<SocketAddr, std::io::Error> {
    use std::net::ToSocketAddrs;
    host_port
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no addr"))
}

fn random_transaction_id() -> [u8; 12] {
    use rand::RngCore;
    let mut id = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut id);
    id
}

fn build_binding_request(tx_id: &[u8; 12]) -> [u8; 20] {
    let mut msg = [0u8; 20];
    msg[0..2].copy_from_slice(&BINDING_REQUEST.to_be_bytes());
    msg[2..4].copy_from_slice(&0u16.to_be_bytes()); // message length = 0
    msg[4..8].copy_from_slice(&MAGIC_COOKIE.to_be_bytes());
    msg[8..20].copy_from_slice(tx_id);
    msg
}

fn parse_binding_response(data: &[u8], tx_id: &[u8; 12]) -> Result<SocketAddr> {
    if data.len() < 20 {
        return Err(Error::Stun("Response too short".into()));
    }
    let msg_type = u16::from_be_bytes([data[0], data[1]]);
    if msg_type != BINDING_RESPONSE {
        return Err(Error::Stun(format!("Unexpected message type: {msg_type:#06x}")));
    }
    let cookie = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    if cookie != MAGIC_COOKIE {
        return Err(Error::Stun("Invalid magic cookie".into()));
    }
    if &data[8..20] != tx_id {
        return Err(Error::Stun("Transaction ID mismatch".into()));
    }

    let msg_len = u16::from_be_bytes([data[2], data[3]]) as usize;
    let attrs = &data[20..20 + msg_len.min(data.len().saturating_sub(20))];
    parse_attributes(attrs, tx_id)
}

fn parse_attributes(attrs: &[u8], tx_id: &[u8; 12]) -> Result<SocketAddr> {
    let mut i = 0;
    while i + 4 <= attrs.len() {
        let attr_type = u16::from_be_bytes([attrs[i], attrs[i + 1]]);
        let attr_len = u16::from_be_bytes([attrs[i + 2], attrs[i + 3]]) as usize;
        let val_start = i + 4;
        let val_end = val_start + attr_len;
        if val_end > attrs.len() {
            break;
        }
        let val = &attrs[val_start..val_end];

        match attr_type {
            ATTR_XOR_MAPPED_ADDRESS => {
                return parse_xor_mapped_address(val, tx_id);
            }
            ATTR_MAPPED_ADDRESS => {
                return parse_mapped_address(val);
            }
            _ => {}
        }

        // Attributes are 4-byte aligned
        i = val_end + ((4 - (attr_len % 4)) % 4);
    }
    Err(Error::Stun("No mapped address attribute found".into()))
}

fn parse_xor_mapped_address(val: &[u8], _tx_id: &[u8; 12]) -> Result<SocketAddr> {
    if val.len() < 8 {
        return Err(Error::Stun("XOR-MAPPED-ADDRESS too short".into()));
    }
    let family = val[1];
    let x_port = u16::from_be_bytes([val[2], val[3]]);
    let port = x_port ^ ((MAGIC_COOKIE >> 16) as u16);

    if family == 0x01 {
        // IPv4
        let x_ip = u32::from_be_bytes([val[4], val[5], val[6], val[7]]);
        let ip = x_ip ^ MAGIC_COOKIE;
        let octets = ip.to_be_bytes();
        let addr = std::net::Ipv4Addr::new(octets[0], octets[1], octets[2], octets[3]);
        Ok(SocketAddr::from((addr, port)))
    } else {
        Err(Error::Stun("IPv6 XOR-MAPPED-ADDRESS not yet supported".into()))
    }
}

fn parse_mapped_address(val: &[u8]) -> Result<SocketAddr> {
    if val.len() < 8 {
        return Err(Error::Stun("MAPPED-ADDRESS too short".into()));
    }
    let family = val[1];
    let port = u16::from_be_bytes([val[2], val[3]]);
    if family == 0x01 {
        let addr = std::net::Ipv4Addr::new(val[4], val[5], val[6], val[7]);
        Ok(SocketAddr::from((addr, port)))
    } else {
        Err(Error::Stun("IPv6 MAPPED-ADDRESS not yet supported".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_response(ip: [u8; 4], port: u16, tx_id: &[u8; 12]) -> Vec<u8> {
        // Build XOR-MAPPED-ADDRESS attribute
        let x_port = port ^ ((MAGIC_COOKIE >> 16) as u16);
        let x_ip = u32::from_be_bytes(ip) ^ MAGIC_COOKIE;
        let x_ip_bytes = x_ip.to_be_bytes();

        let attr_body: Vec<u8> = vec![
            0x00, 0x01, // family IPv4
            x_port.to_be_bytes()[0],
            x_port.to_be_bytes()[1],
            x_ip_bytes[0],
            x_ip_bytes[1],
            x_ip_bytes[2],
            x_ip_bytes[3],
        ];
        let attr_len = attr_body.len() as u16;
        let msg_len = 4 + attr_body.len();

        let mut msg = Vec::with_capacity(20 + msg_len);
        msg.extend_from_slice(&BINDING_RESPONSE.to_be_bytes());
        msg.extend_from_slice(&(msg_len as u16).to_be_bytes());
        msg.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());
        msg.extend_from_slice(tx_id);
        msg.extend_from_slice(&ATTR_XOR_MAPPED_ADDRESS.to_be_bytes());
        msg.extend_from_slice(&attr_len.to_be_bytes());
        msg.extend_from_slice(&attr_body);
        msg
    }

    #[test]
    fn test_binding_request_magic_cookie() {
        let tx_id = [0u8; 12];
        let req = build_binding_request(&tx_id);
        let cookie = u32::from_be_bytes([req[4], req[5], req[6], req[7]]);
        assert_eq!(cookie, MAGIC_COOKIE);
    }

    #[test]
    fn test_binding_request_type() {
        let tx_id = [0u8; 12];
        let req = build_binding_request(&tx_id);
        let msg_type = u16::from_be_bytes([req[0], req[1]]);
        assert_eq!(msg_type, BINDING_REQUEST);
    }

    #[test]
    fn test_parse_valid_response() {
        let tx_id = [1u8; 12];
        let data = make_response([1, 2, 3, 4], 54321, &tx_id);
        let addr = parse_binding_response(&data, &tx_id).unwrap();
        assert_eq!(addr.ip().to_string(), "1.2.3.4");
        assert_eq!(addr.port(), 54321);
    }

    #[test]
    fn test_parse_wrong_magic_cookie() {
        let tx_id = [1u8; 12];
        let mut data = make_response([1, 2, 3, 4], 1234, &tx_id);
        data[4] = 0x00; // corrupt magic cookie
        assert!(parse_binding_response(&data, &tx_id).is_err());
    }

    #[test]
    fn test_parse_tx_id_mismatch() {
        let tx_id = [1u8; 12];
        let data = make_response([1, 2, 3, 4], 1234, &tx_id);
        let wrong_tx_id = [2u8; 12];
        assert!(parse_binding_response(&data, &wrong_tx_id).is_err());
    }

    #[test]
    fn test_parse_too_short() {
        let tx_id = [0u8; 12];
        assert!(parse_binding_response(&[0u8; 10], &tx_id).is_err());
    }

    #[test]
    #[ignore = "requires internet access"]
    fn test_discover_real_endpoint() {
        let addr = discover_external_endpoint("stun.l.google.com:19302").unwrap();
        assert!(!addr.ip().is_unspecified());
        assert!(addr.port() > 1024);
    }
}
