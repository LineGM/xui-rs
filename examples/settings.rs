//! Reads panel and Xray settings without printing secret-bearing documents.

use xui_rs::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder(std::env::var("XUI_URL")?)?
        .bearer_token(std::env::var("XUI_API_TOKEN")?)
        .build()?;

    let panel = client.settings().all().await?;
    let xray = client.xray_settings().settings().await?;

    println!(
        "panel port {}; {} inbound tags; API tokens present: {}",
        panel.settings.web.web_port,
        xray.inbound_tags.len(),
        panel.has_api_token
    );
    Ok(())
}
