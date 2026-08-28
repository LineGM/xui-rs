//! Reads public subscription metadata without downloading its secret body.

use xui_rs::SubscriptionClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = SubscriptionClient::new(std::env::var("XUI_SUB_URL")?)?;
    let subscription_id = std::env::var("XUI_SUB_ID")?;

    let metadata = client.raw_metadata(&subscription_id).await?;
    println!("profile: {:?}", metadata.profile_title);
    println!("traffic: {:?}", metadata.traffic);

    let info = client.info(&subscription_id).await?;
    println!("enabled: {}; online: {}", info.enabled, info.is_online);
    Ok(())
}
