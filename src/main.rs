use xui_rs::api::XUiClient;
use xui_rs::errors::MyError;

#[tokio::main]
async fn main() -> Result<(), MyError> {
    let panel_url = "PANEL_URL";
    let mut client = XUiClient::new(&panel_url);

    let user = "USERNAME";
    let password = "PASSWORD";
    client.login(user, password).await?;

    let inbounds = client.get_inbounds().await?;
    let inbound = client.get_inbound(1).await?;
    let traffic_by_email = client.get_client_traffic_by_email("EMAIL").await?;
    let traffic_by_uuid = client.get_client_traffic_by_uuid("UUID").await?;
    client.get_backup().await?;

    println!("{}\n\n", serde_json::to_string_pretty(&inbounds)?);
    println!("{}\n\n", serde_json::to_string_pretty(&inbound)?);
    println!("{}\n\n", serde_json::to_string_pretty(&traffic_by_email)?);
    println!("{}\n\n", serde_json::to_string_pretty(&traffic_by_uuid)?);

    Ok(())
}
