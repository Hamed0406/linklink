use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::time::Duration;
use tokio::time::sleep;

use super::{token_path, token_store::{save_token, SavedToken}};

#[derive(Deserialize)]
struct DeviceFlowStart {
    device_code: String,
    user_code: String,
    verification_uri: String,
    interval: u64,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    error: Option<String>,
}

pub async fn cmd_login(server: Option<String>) -> Result<()> {
    let server = server
        .or_else(|| std::env::var("LINKLINK_SERVER").ok())
        .unwrap_or_else(|| "http://localhost:8080".into());
    let server = server.trim_end_matches('/').to_string();

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    // Step 1 — start device flow
    let flow: DeviceFlowStart = client
        .post(format!("{server}/api/v1/auth/device"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .context("could not reach server")?
        .error_for_status()
        .context("server rejected device flow start")?
        .json()
        .await?;

    // Step 2 — show user what to do
    println!("\n=== Login to linklink ===\n");
    println!("Visit:  {server}{}", flow.verification_uri);
    println!("Code:   {}\n", flow.user_code);
    println!("Waiting for you to approve in the browser...\n");

    // Step 3 — poll until approved or expired
    let interval = Duration::from_secs(flow.interval.max(3));
    loop {
        sleep(interval).await;

        let resp = client
            .post(format!("{server}/api/v1/auth/token"))
            .json(&serde_json::json!({
                "grant_type": "urn:ietf:params:oauth:grant-type:device_code",
                "device_code": flow.device_code,
            }))
            .send()
            .await
            .context("poll request failed")?
            .json::<TokenResponse>()
            .await?;

        match resp.error.as_deref() {
            Some("authorization_pending") => {
                print!(".");
                std::io::Write::flush(&mut std::io::stdout()).ok();
                continue;
            }
            Some("expired_token") => bail!("Login code expired. Run `linklink login` again."),
            Some(other) => bail!("Server error: {other}"),
            None => {}
        }

        let access_token = resp.access_token.context("server returned no access_token")?;
        let refresh_token = resp.refresh_token.context("server returned no refresh_token")?;

        let token = SavedToken { access_token, refresh_token, server_url: server.clone() };
        save_token(&token_path(), &token).context("failed to save token")?;

        println!("\nLogged in. Token saved to {}", token_path().display());
        println!("Run `linklink register --name <your-device-name>` next.");
        return Ok(());
    }
}
