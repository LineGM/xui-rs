use xui_rs::api::XUiClient;
use xui_rs::errors::MyError;

#[tokio::main]
async fn main() -> Result<(), MyError> {
    // A trailing slash is significant.
    // Without it, the last path component is considered to be a “file” name
    // to be removed to get at the “directory” that is used as the base.
    let panel_url = "PANEL_BASE_URL";
    let mut panel_client = match XUiClient::new(panel_url) {
        Ok(panel_client) => panel_client,
        Err(e) => return Err(e),
    };

    let user = "USERNAME";
    let password = "PASSWORD";
    panel_client.login(user, password).await?;

    let inbounds = panel_client.get_inbounds().await?;
    println!("{}\n\n", serde_json::to_string_pretty(&inbounds)?);

    let inbound = panel_client.get_inbound(1_u64).await?;
    println!("{}\n\n", serde_json::to_string_pretty(&inbound)?);

    let traffic_by_email = panel_client.get_client_traffic_by_email("EMAIL").await?;
    println!("{}\n\n", serde_json::to_string_pretty(&traffic_by_email)?);

    let traffic_by_uuid = panel_client.get_client_traffic_by_uuid("UUID").await?;
    println!("{}\n\n", serde_json::to_string_pretty(&traffic_by_uuid)?);

    let backup = panel_client.get_backup().await?;
    println!("{}\n\n", serde_json::to_string_pretty(&backup)?);

    Ok(())
}
