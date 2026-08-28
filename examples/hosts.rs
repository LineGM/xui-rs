//! Lists host overrides and builds a typed create request.

use xui_rs::{Client, HostGroup, HostSecurity};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder(std::env::var("XUI_URL")?)?
        .bearer_token(std::env::var("XUI_API_TOKEN")?)
        .build()?;

    let existing = client.hosts().list().await?;
    println!("{} host override groups", existing.len());

    let mut proposed = HostGroup::new(vec![7], "production CDN");
    proposed.hosts = vec!["cdn.example.com".into()];
    proposed.options.port = 443;
    proposed.options.security = HostSecurity::Tls;
    proposed.options.sni = "origin.example.com".into();

    // Creation is explicit because it expands into persistent database rows.
    // let created = client.hosts().create(&proposed).await?;
    println!("proposed group: {proposed:?}");
    Ok(())
}
