#![allow(missing_docs)]

use xui_rs::{Client, LoginRequest};

#[tokio::main]
async fn main() -> xui_rs::Result<()> {
    let panel_url = std::env::var("XUI_PANEL_URL").expect("XUI_PANEL_URL must be set");
    let username = std::env::var("XUI_USERNAME").expect("XUI_USERNAME must be set");
    let password = std::env::var("XUI_PASSWORD").expect("XUI_PASSWORD must be set");

    let client = Client::new(panel_url)?;
    client
        .auth()
        .login(LoginRequest::new(username, password))
        .await?;
    println!("Authenticated successfully");
    client.auth().logout().await?;
    Ok(())
}
