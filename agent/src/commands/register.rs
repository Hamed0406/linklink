/// Phase 5 — Device registration with control plane.
/// Implemented in Phase 5 of the implementation plan.
pub async fn cmd_register(name: String) -> anyhow::Result<()> {
    println!("Register not yet implemented (Phase 5).");
    println!("Device name: {name}");
    println!("When implemented, this will:");
    println!("  1. Generate a WireGuard keypair locally");
    println!("  2. Discover external endpoint via STUN");
    println!("  3. Send public key + device info to the control plane");
    println!("  4. Wait for admin approval");
    Ok(())
}
