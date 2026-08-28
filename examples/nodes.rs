//! Lists remote nodes and builds a non-persistent typed registration request.

use xui_rs::{Client, NodeRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder(std::env::var("XUI_URL")?)?
        .bearer_token(std::env::var("XUI_API_TOKEN")?)
        .build()?;

    let nodes = client.nodes().list().await?;
    let online = nodes
        .iter()
        .filter(|node| node.status.as_str() == "online")
        .count();
    println!("{} nodes, {online} online", nodes.len());

    let proposed = NodeRequest::new("edge-de", "node.example.com", 2053)
        .with_api_token(std::env::var("NODE_API_TOKEN")?);

    // Registration probes and persists the node, so it remains explicit.
    // let created = client.nodes().create(&proposed).await?;
    println!("proposed node: {proposed:?}");
    Ok(())
}
