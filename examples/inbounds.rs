#![allow(missing_docs)]

use xui_rs::{Client, InboundConfig, InboundProtocol};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder(std::env::var("XUI_URL")?)?
        .bearer_token(std::env::var("XUI_API_TOKEN")?)
        .build()?;

    for inbound in client.inbounds().list_slim().await? {
        println!(
            "{}: {} on port {}",
            inbound.id, inbound.config.remark, inbound.config.port
        );
    }

    let mut config = InboundConfig::new(InboundProtocol::Vless, 443);
    "managed-by-xui-rs".clone_into(&mut config.remark);
    config.settings = serde_json::json!({ "clients": [] });
    config.stream_settings = serde_json::json!({ "network": "tcp" });

    // Uncomment only when you intend to create the inbound.
    // let created = client.inbounds().create(&config).await?;
    // println!("created inbound {}", created.id);
    println!("prepared inbound config: {}", config.remark);

    Ok(())
}
