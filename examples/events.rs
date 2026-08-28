//! Receives a bounded number of typed real-time events from a cookie session.

use xui_rs::{Client, LoginRequest, PanelEventKind};

#[tokio::main]
async fn main() -> xui_rs::Result<()> {
    let client = Client::new(std::env::var("XUI_PANEL_URL").expect("XUI_PANEL_URL must be set"))?;
    client
        .auth()
        .login(LoginRequest::new(
            std::env::var("XUI_USERNAME").expect("XUI_USERNAME must be set"),
            std::env::var("XUI_PASSWORD").expect("XUI_PASSWORD must be set"),
        ))
        .await?;

    let limit = std::env::var("XUI_EVENT_LIMIT")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(10);
    let mut events = client.events().connect().await?;
    for _ in 0..limit {
        let Some(event) = events.next_event().await? else {
            break;
        };
        match &event.kind {
            PanelEventKind::Status(status) => {
                println!("CPU {:.1}%; Xray {:?}", status.cpu, status.xray.state);
            }
            kind => println!("{} event", kind.message_type().as_str()),
        }
    }

    events.close().await?;
    client.auth().logout().await?;
    Ok(())
}
