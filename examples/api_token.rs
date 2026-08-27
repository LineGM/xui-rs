#![allow(missing_docs)]

use xui_rs::Client;

fn main() -> xui_rs::Result<()> {
    let panel_url = std::env::var("XUI_PANEL_URL").expect("XUI_PANEL_URL must be set");
    let api_token = std::env::var("XUI_API_TOKEN").expect("XUI_API_TOKEN must be set");

    let client = Client::builder(panel_url)?
        .bearer_token(api_token)
        .build()?;
    println!("Configured client for {}", client.base_url());
    Ok(())
}
