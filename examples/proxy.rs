#![allow(missing_docs)]

use xui_rs::{Client, ProxyConfig};

fn main() -> xui_rs::Result<()> {
    let panel_url = std::env::var("XUI_PANEL_URL").expect("XUI_PANEL_URL must be set");
    let proxy_url = std::env::var("XUI_PROXY_URL").expect("XUI_PROXY_URL must be set");

    let mut proxy = ProxyConfig::new(proxy_url)?;
    if let Ok(username) = std::env::var("XUI_PROXY_USERNAME") {
        let password = std::env::var("XUI_PROXY_PASSWORD").expect("XUI_PROXY_PASSWORD must be set");
        proxy = proxy.with_basic_auth(username, password)?;
    }

    let client = Client::builder(panel_url)?.proxy(proxy).build()?;
    println!("Configured proxied client for {}", client.base_url());
    Ok(())
}
