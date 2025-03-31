# xui-rs: 3X-UI Panel API Client

[![Windows](https://github.com/LineGM/xui-rs/actions/workflows/windows.yml/badge.svg)](https://github.com/LineGM/xui-rs/actions/workflows/windows.yml)
[![Linux](https://github.com/LineGM/xui-rs/actions/workflows/linux.yml/badge.svg)](https://github.com/LineGM/xui-rs/actions/workflows/linux.yml)
[![macOS](https://github.com/LineGM/xui-rs/actions/workflows/macos.yml/badge.svg)](https://github.com/LineGM/xui-rs/actions/workflows/macos.yml)

[![Coverage Status](https://coveralls.io/repos/github/LineGM/xui-rs/badge.svg?branch=main)](https://coveralls.io/github/LineGM/xui-rs?branch=main)
[![Dependencies Status](https://deps.rs/repo/github/LineGM/xui-rs/status.svg)](https://deps.rs/repo/github/LineGM/xui-rs)

[![Clippy](https://github.com/LineGM/xui-rs/actions/workflows/clippy.yml/badge.svg)](https://github.com/LineGM/xui-rs/actions/workflows/clippy.yml)
[![Rustfmt](https://github.com/LineGM/xui-rs/actions/workflows/rustfmt.yml/badge.svg)](https://github.com/LineGM/xui-rs/actions/workflows/rustfmt.yml)

## Features

* Login to the **3X-UI** panel and manage session cookies automatically.
* Automatic re-login when the session cookie expires (if initial credentials are provided).
* Fetch a list of all inbound configurations.
* Fetch details for a specific inbound configuration by **ID**.
* Fetch client traffic statistics by **email**.
* Fetch client traffic statistics by **UUID**.
* Trigger a panel configuration backup.
* Async API calls using `reqwest` and `tokio`.
* Custom error type (`MyError`) for easier error handling.

## Installation

Add this library to your `Cargo.toml`:

```toml
[dependencies]
xui-rs = { git = "[https://github.com/LineGM/xui-rs.git](https://github.com/LineGM/xui-rs.git)" } # Or path = "path/to/xui-rs" if local, or version if published
tokio = { version = "1", features = ["full"] }
serde_json = "1.0"
```

## Usage

You need an async runtime like `tokio`.

```rust
use xui_rs::XUiClient;
use xui_rs::errors::MyError;

#[tokio::main]
async fn main() -> Result<(), MyError> {
    // IMPORTANT: The panel URL MUST end with a trailing slash `/`.
    let panel_url = "[https://your-xui-panel.com/](https://your-xui-panel.com/)";
    let username = "your_username";
    let password = "your_password";

    // Create a new client instance
    let mut client = XUiClient::new(panel_url)?;

    // Login to the panel
    client.login(username, password).await?;
    println!("Successfully logged in!");

    // Get all inbounds
    match client.get_inbounds().await {
        Ok(inbounds) => {
            println!("Inbounds:\n{}", serde_json::to_string_pretty(&inbounds)?);
        }
        Err(e) => eprintln!("Error getting inbounds: {}", e),
    }

    // Get a specific inbound (e.g., ID 1)
    let inbound_id = 1_u64;
    match client.get_inbound(inbound_id).await {
        Ok(inbound) => {
             println!("\nInbound {}:\n{}", inbound_id, serde_json::to_string_pretty(&inbound)?);
        }
        Err(e) => eprintln!("Error getting inbound {}: {}", inbound_id, e),
    }

    // Get client traffic by email
    let client_email = "user@example.com";
    match client.get_client_traffic_by_email(client_email).await {
        Ok(traffic) => {
             println!("\nTraffic for {}:\n{}", client_email, serde_json::to_string_pretty(&traffic)?);
        }
        Err(e) => eprintln!("Error getting traffic for {}: {}", client_email, e),
    }

    // Get client traffic by UUID
    let client_uuid = "d7c06399-a3e3-4007-9109-19012597dd01"; // Replace with a valid UUID
    match client.get_client_traffic_by_uuid(client_uuid).await {
        Ok(traffic) => {
             println!("\nTraffic for UUID {}:\n{}", client_uuid, serde_json::to_string_pretty(&traffic)?);
        }
        Err(e) => eprintln!("Error getting traffic for UUID {}: {}", client_uuid, e),
    }

    // Trigger a backup
    match client.get_backup().await {
        Ok(status_code) => {
            println!("\nBackup request sent. Response status: {}", status_code);
        }
        Err(e) => eprintln!("Error triggering backup: {}", e),
    }

    Ok(())
}
```

## API Methods
* ``XUiClient::new(panel_url: impl IntoUrl) -> Result<Self, MyError>``: Creates a new client. panel_url must end with /.
* ``client.login(username: impl Into<String>, password: impl Into<String>) -> Result<(), MyError>``: Logs in and stores the session cookie.
* ``client.get_inbounds() -> Result<serde_json::Value, MyError>``: Gets all inbounds.
* ``client.get_inbound(inbound_id: impl Into<u64>) -> Result<serde_json::Value, MyError>``: Gets a specific inbound by ID.
* ``client.get_client_traffic_by_email(client_email: impl Into<String>) -> Result<serde_json::Value, MyError>``: Gets client traffic by email.
* ``client.get_client_traffic_by_uuid(uuid: impl Into<String>) -> Result<serde_json::Value, MyError>``: Gets client traffic by UUID.
* ``client.get_backup() -> Result<u16, MyError>``: Triggers a panel backup and returns the HTTP status code.

## Error Handling

Most methods return `Result<T, MyError>`. The `MyError` enum encapsulates various potential errors, including network errors (`reqwest::Error`), JSON parsing errors (`serde_json::Error`), URL parsing errors, and API-specific issues.

## Contributing

Contributions are welcome! Please see [**CONTRIBUTING**](CONTRIBUTING.md) for details on how to contribute.

## License

This project is licensed under the **Unlicense** - see the [**LICENSE**](LICENSE) file for details. 
