//! Reads typed server status and CPU history using API-token authentication.

use xui_rs::{Client, HistoryBucket, SystemMetric};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder(std::env::var("XUI_URL")?)?
        .bearer_token(std::env::var("XUI_API_TOKEN")?)
        .build()?;

    let status = client.server().status().await?;
    let history = client
        .server()
        .system_history(SystemMetric::Cpu, HistoryBucket::Hour1)
        .await?;

    println!(
        "panel {} / Xray {} ({:?}); {} CPU samples",
        status.panel_version,
        status.xray.version,
        status.xray.state,
        history.len()
    );
    Ok(())
}
