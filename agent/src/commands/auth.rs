/// Phase 4 — OAuth2 device authorization flow.
/// Implemented in Phase 4 of the implementation plan.
pub async fn cmd_login(server: Option<String>) -> anyhow::Result<()> {
    let server = server.unwrap_or_else(|| {
        std::env::var("LINKLINK_SERVER").unwrap_or_else(|_| "https://ctrl.linklink.dev".into())
    });
    println!("Login not yet implemented (Phase 4).");
    println!("Target server: {server}");
    println!("When implemented, this will start the device authorization flow.");
    println!("You will visit a URL and enter a short code to authenticate.");
    Ok(())
}
