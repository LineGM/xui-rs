#![allow(missing_docs)]

use xui_rs::{Client, ClientPageRequest, ClientSort, ClientStatusFilter, SortOrder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder(std::env::var("XUI_URL")?)?
        .bearer_token(std::env::var("XUI_API_TOKEN")?)
        .build()?;

    let page = client
        .clients()
        .list_paged(&ClientPageRequest {
            statuses: vec![ClientStatusFilter::Online],
            sort: Some(ClientSort::LastOnline),
            order: SortOrder::Descending,
            ..ClientPageRequest::default()
        })
        .await?;

    println!(
        "showing {} of {} clients; {} currently online",
        page.items.len(),
        page.filtered,
        page.summary.online_count
    );
    for row in page.items {
        println!(
            "{}: {} bytes used",
            row.email,
            row.traffic.map_or(0, |t| t.up + t.down)
        );
    }

    Ok(())
}
