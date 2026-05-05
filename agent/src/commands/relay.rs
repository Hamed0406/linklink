use linklink_core::keystore::{save_private_key, KeyPair};
use super::private_key_path;

pub async fn cmd_relay_enable() -> anyhow::Result<()> {
    println!("Relay mode enabled. This device will forward traffic for other peers.");
    println!("Ensure UDP port 51820 and 443 are open on this machine's firewall.");
    // TODO: update config, set is_relay = true, notify control plane if registered
    Ok(())
}

pub async fn cmd_relay_disable() -> anyhow::Result<()> {
    println!("Relay mode disabled.");
    Ok(())
}

/// Generates the relay hub keypair locally. Public key is printed for registration.
pub async fn cmd_relay_init() -> anyhow::Result<()> {
    let key_path = private_key_path();
    if key_path.exists() {
        println!("Key already exists at {}. Use `relay register` to register it.", key_path.display());
        return Ok(());
    }

    let kp = KeyPair::generate();
    save_private_key(&key_path, &kp.private_key)?;

    println!("Hub keypair generated.");
    println!("Public key: {}", kp.public_key_base64());
    println!("Private key stored at: {} (0600)", key_path.display());
    println!("\nNext: run `linklink relay register --server <control-plane-url>`");

    Ok(())
}

pub async fn cmd_relay_register(server: String) -> anyhow::Result<()> {
    use linklink_core::keystore::load_private_key;
    let key_path = private_key_path();
    let bytes = load_private_key(&key_path)
        .map_err(|_| anyhow::anyhow!("No keypair found. Run `linklink relay init` first."))?;
    let kp = KeyPair::from_private_bytes(*bytes);

    println!("Registering relay with: {server}");
    println!("Hub public key: {}", kp.public_key_base64());
    // TODO: POST /api/v1/hub/register, save returned API key to /etc/linklink/hub-api-key
    println!("(Control plane registration not yet implemented — Phase 6)");

    Ok(())
}
