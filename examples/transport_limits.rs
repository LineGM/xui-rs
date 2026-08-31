#![allow(missing_docs)]

use xui_rs::{Client, SubscriptionClient};

fn main() -> xui_rs::Result<()> {
    let panel_url = std::env::var("XUI_PANEL_URL").expect("XUI_PANEL_URL must be set");

    let panel = Client::builder(&panel_url)?
        .response_body_limit(16 * 1024 * 1024)
        .download_body_limit(1024 * 1024 * 1024)
        .build()?;
    let subscriptions = SubscriptionClient::builder(panel_url)?
        .response_body_limit(32 * 1024 * 1024)
        .build()?;

    println!(
        "Limits: API={} MiB, database={} MiB, subscriptions={} MiB",
        panel.response_body_limit() / 1024 / 1024,
        panel.download_body_limit() / 1024 / 1024,
        subscriptions.response_body_limit() / 1024 / 1024,
    );
    Ok(())
}
