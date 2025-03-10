use xui_rs::api::XUiClient;
use xui_rs::errors::MyError;

#[tokio::main]
async fn main() -> Result<(), MyError> {
    let panel_url = "http://HOST:PORT/PANEL_BASE_PATH/";
    let mut panel_client = match XUiClient::new(panel_url) {
        Ok(panel_client) => panel_client,
        Err(e) => return Err(e),
    };

    let user = "USERNAME";
    let password = "PASSWORD";
    panel_client.login(user, password).await?;

    let inbounds = panel_client.get_inbounds().await?;
    println!("{}\n\n", serde_json::to_string_pretty(&inbounds)?);

    let inbound = panel_client.get_inbound(1).await?;
    println!("{}\n\n", serde_json::to_string_pretty(&inbound)?);

    let traffic_by_email = panel_client
        .get_client_traffic_by_email("USER_EMAIL")
        .await?;
    println!("{}\n\n", serde_json::to_string_pretty(&traffic_by_email)?);

    let traffic_by_uuid = panel_client.get_client_traffic_by_uuid("USER_UUID").await?;
    println!("{}\n\n", serde_json::to_string_pretty(&traffic_by_uuid)?);

    let backup = panel_client.get_backup().await?;
    println!("{}\n\n", serde_json::to_string_pretty(&backup)?);

    Ok(())
}
