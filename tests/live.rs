//! Opt-in integration tests against a real disposable 3x-ui v3.8.5 panel.

use std::{collections::BTreeSet, env, error::Error as StdError, io, time::Duration};

use serde_json::json;
use xui_rs::{
    ApiTokenCreateRequest, ApiTokenScope, Client, ErrorKind, InboundConfig, InboundProtocol,
    LoginRequest, OpenApiDocument, ServerStatus, SubscriptionBalancerInput,
    SubscriptionBalancerStrategy,
};

type TestResult<T = ()> = Result<T, Box<dyn StdError + Send + Sync>>;

struct LiveConfig {
    base_url: String,
    username: String,
    password: String,
    expected_version: String,
}

impl LiveConfig {
    fn from_env() -> TestResult<Self> {
        Ok(Self {
            base_url: env::var("XUI_LIVE_BASE_URL")?,
            username: env::var("XUI_LIVE_USERNAME")?,
            password: env::var("XUI_LIVE_PASSWORD")?,
            expected_version: env::var("XUI_LIVE_EXPECTED_VERSION")?,
        })
    }

    fn client(&self) -> TestResult<Client> {
        Ok(Client::new(&self.base_url)?)
    }

    async fn authenticated_client(&self) -> TestResult<Client> {
        let client = self.client()?;
        client
            .auth()
            .login(LoginRequest::new(&self.username, &self.password))
            .await?;
        Ok(client)
    }
}

fn require(condition: bool, message: impl Into<String>) -> TestResult {
    if condition {
        Ok(())
    } else {
        Err(io::Error::other(message.into()).into())
    }
}

fn normalized_version(version: &str) -> &str {
    version.strip_prefix('v').unwrap_or(version)
}

async fn wait_for_server_status(client: &Client) -> TestResult<ServerStatus> {
    for _ in 0..30 {
        match client.server().status().await {
            Ok(status) => return Ok(status),
            Err(error) if error.kind() == ErrorKind::MissingObject => {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Err(error) => return Err(error.into()),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        "3x-ui server status cache did not become ready within 30 seconds",
    )
    .into())
}

/// Checks cookie login, typed HTTP responses, the live API contract, WebSocket status, and logout.
/// Requires an explicitly configured real panel and remains ignored in ordinary test runs.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires XUI_LIVE_* credentials for a real 3x-ui v3.8.5 panel"]
async fn live_cookie_http_and_websocket_smoke() -> TestResult {
    let config = LiveConfig::from_env()?;
    let client = config.client()?;

    let csrf = client.auth().csrf_token().await?;
    require(
        !csrf.expose().is_empty(),
        "panel returned an empty CSRF token",
    )?;
    require(
        !client.auth().is_two_factor_enabled().await?,
        "the isolated live-test account unexpectedly requires two-factor authentication",
    )?;

    client
        .auth()
        .login(LoginRequest::new(&config.username, &config.password))
        .await?;

    let settings = client.settings().all().await?;
    require(
        settings.settings.web.web_base_path == client.base_url().path(),
        "runtime web base path differs from the SDK base URL",
    )?;
    let xray_settings = client.xray_settings().settings().await?;
    require(
        !xray_settings.geodata_sources.is_empty(),
        "tagged panel did not return its standard geodata presets",
    )?;

    let status = wait_for_server_status(&client).await?;
    require(
        normalized_version(&status.panel_version) == normalized_version(&config.expected_version),
        format!(
            "expected panel {}, got {}",
            config.expected_version, status.panel_version
        ),
    )?;

    let runtime_openapi = client.panel().openapi().await?;
    let vendored_value: serde_json::Value = serde_json::from_str(json_include())?;
    let vendored_openapi = OpenApiDocument::from(vendored_value);
    require(
        runtime_openapi.version() == vendored_openapi.version(),
        "runtime and vendored OpenAPI versions differ",
    )?;
    let runtime_operations = http_operations(&runtime_openapi);
    let mut expected_operations = http_operations(&vendored_openapi);
    expected_operations.extend(source_only_panel_operations()?);
    require(
        runtime_operations == expected_operations,
        format!(
            "runtime OpenAPI operation set differs from the tagged contract; missing: {:?}; \
             unexpected: {:?}",
            expected_operations
                .difference(&runtime_operations)
                .collect::<Vec<_>>(),
            runtime_operations
                .difference(&expected_operations)
                .collect::<Vec<_>>()
        ),
    )?;

    let inbounds = client.inbounds().list().await?;
    let slim = client.inbounds().list_slim().await?;
    require(
        inbounds.len() == slim.len(),
        "full and slim inbound lists have different lengths",
    )?;

    check_websocket_status(&client, &config.expected_version).await?;

    client.auth().logout().await?;
    let error = client
        .server()
        .status()
        .await
        .expect_err("logout must invalidate the protected cookie session");
    require(
        error.kind() == ErrorKind::Unauthorized,
        format!("protected request after logout failed as {}", error.kind()),
    )
}

/// Exercises token revocation and inbound/balancer mutations on an explicitly disposable panel.
/// Attempts cleanup of created resources even when the operation checks fail.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires XUI_LIVE_ALLOW_MUTATION=1 and a disposable 3x-ui v3.8.5 panel"]
async fn live_token_and_inbound_round_trip_with_cleanup() -> TestResult {
    require(
        env::var("XUI_LIVE_ALLOW_MUTATION").as_deref() == Ok("1"),
        "refusing to mutate a panel without XUI_LIVE_ALLOW_MUTATION=1",
    )?;
    let config = LiveConfig::from_env()?;
    let client = config.authenticated_client().await?;

    let token_name = format!("xui-rs-live-{}", std::process::id());
    let created_token = client
        .settings()
        .create_api_token(&ApiTokenCreateRequest::new(&token_name))
        .await?;
    let token_id = created_token.id;
    let mut inbound_id = None;
    let mut balancer_id = None;

    let operation_result: TestResult = async {
        require(
            created_token.name == token_name && created_token.enabled,
            "created API token metadata is inconsistent",
        )?;
        let tokens = client.settings().api_tokens().await?;
        require(
            tokens.iter().any(|token| token.id == token_id),
            "created API token was not returned by the list endpoint",
        )?;

        let bearer = Client::builder(&config.base_url)?
            .bearer_token(created_token.token)
            .build()?;
        bearer.settings().all().await?;
        wait_for_server_status(&bearer).await?;
        check_disabled_token(&client, &bearer, token_id).await?;

        let inbound = live_inbound_config();

        let created = client.inbounds().create(&inbound).await?;
        inbound_id = Some(created.id);
        require(
            created.config.remark == inbound.remark && !created.config.enable,
            "created inbound differs from the requested disabled configuration",
        )?;

        let fetched = client.inbounds().get(created.id).await?;
        require(fetched.id == created.id, "inbound get returned another row")?;
        require(
            client
                .inbounds()
                .list()
                .await?
                .iter()
                .any(|item| item.id == created.id),
            "created inbound is absent from the full list",
        )?;
        require(
            client
                .inbounds()
                .list_slim()
                .await?
                .iter()
                .any(|item| item.id == created.id),
            "created inbound is absent from the slim list",
        )?;
        check_inbound_option(&client, created.id).await?;
        client.inbounds().all_links().await?;

        check_weighted_balancer_and_sort(&client, created.id, &mut balancer_id).await?;

        let mut update = fetched.to_config();
        update.remark.push_str("-updated");
        let updated = client.inbounds().update(created.id, &update).await?;
        require(
            updated.config.remark == update.remark,
            "updated inbound remark was not persisted",
        )?;
        client.inbounds().set_enabled(created.id, false).await?;
        client.inbounds().reset_traffic(created.id).await?;
        Ok(())
    }
    .await;

    let balancer_cleanup = match balancer_id {
        Some(id) => client
            .subscription_balancers()
            .delete(id)
            .await
            .map_err(Into::into),
        None => Ok(()),
    };
    let inbound_cleanup = match inbound_id {
        Some(id) => client
            .inbounds()
            .delete(id)
            .await
            .map(|_| ())
            .map_err(Into::into),
        None => Ok(()),
    };
    let token_cleanup = client
        .settings()
        .delete_api_token(token_id, ApiTokenScope::Admin)
        .await
        .map_err(Into::into);

    operation_result
        .and(balancer_cleanup)
        .and(inbound_cleanup)
        .and(token_cleanup)
}

/// Verifies that a newly created inbound appears in the panel option list.
async fn check_inbound_option(client: &Client, inbound_id: i64) -> TestResult {
    require(
        client
            .inbounds()
            .options()
            .await?
            .iter()
            .any(|item| item.id == inbound_id),
        "created inbound is absent from options",
    )?;
    Ok(())
}

/// Waits at most 30 seconds for a timestamped status event with the expected panel version.
/// Closes the stream after validation and verifies its local closed state.
async fn check_websocket_status(client: &Client, expected_version: &str) -> TestResult {
    let mut events = client.events().connect().await?;
    tokio::time::timeout(Duration::from_secs(30), async {
        while let Some(event) = events.next_event().await? {
            if let xui_rs::PanelEventKind::Status(status) = event.kind {
                require(
                    event.timestamp_ms > 0,
                    "WebSocket envelope has no timestamp",
                )?;
                return require(
                    normalized_version(&status.panel_version)
                        == normalized_version(expected_version),
                    "WebSocket status did not decode the tagged panel version",
                );
            }
        }
        Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "WebSocket closed before status",
        )
        .into())
    })
    .await??;
    events.close().await?;
    require(events.is_closed(), "WebSocket did not close locally")
}

/// Disables an API token, checks for HTTP 401, then re-enables it and verifies access.
async fn check_disabled_token(client: &Client, bearer: &Client, token_id: i64) -> TestResult {
    client
        .settings()
        .set_api_token_enabled(token_id, ApiTokenScope::Admin, false)
        .await?;
    let error = bearer
        .settings()
        .all()
        .await
        .expect_err("disabled token must be rejected");
    require(
        error.kind() == ErrorKind::Unauthorized,
        "disabled token did not return HTTP 401",
    )?;
    client
        .settings()
        .set_api_token_enabled(token_id, ApiTokenScope::Admin, true)
        .await?;
    bearer.settings().all().await?;
    Ok(())
}

/// Checks fractional balancer weights across create, update, and list, plus negative inbound sorting.
/// Records the created balancer ID immediately so the caller can clean up after a failure.
async fn check_weighted_balancer_and_sort(
    client: &Client,
    inbound_id: i64,
    balancer_id: &mut Option<i64>,
) -> TestResult {
    let mut balancer = SubscriptionBalancerInput::new("xui-rs-live-weights", vec![inbound_id]);
    balancer.strategy = SubscriptionBalancerStrategy::LeastLoad;
    balancer.member_weights.insert(inbound_id, 0.5);
    let weighted = client.subscription_balancers().create(&balancer).await?;
    *balancer_id = Some(weighted.id);
    require(
        weighted.member_weights == balancer.member_weights,
        "created balancer lost fractional member weights",
    )?;
    balancer.member_weights.insert(inbound_id, 1.25);
    let weighted = client
        .subscription_balancers()
        .update(weighted.id, &balancer)
        .await?;
    require(
        weighted.member_weights == balancer.member_weights,
        "updated balancer lost fractional member weights",
    )?;
    require(
        client
            .subscription_balancers()
            .list()
            .await?
            .iter()
            .any(|item| item.id == weighted.id && item.member_weights == balancer.member_weights),
        "listed balancer differs from the stored weighted configuration",
    )?;
    client
        .inbounds()
        .set_subscription_sort_index(inbound_id, -2)
        .await?;
    require(
        client
            .inbounds()
            .get(inbound_id)
            .await?
            .config
            .sub_sort_index
            == -2,
        "negative subscription sort index was not preserved",
    )?;

    Ok(())
}

/// Returns the embedded `OpenAPI` snapshot used to compare the live panel with the supported release.
fn json_include() -> &'static str {
    include_str!("../spec/3x-ui-v3.8.5.openapi.json")
}

fn live_inbound_config() -> InboundConfig {
    let mut inbound = InboundConfig::new(InboundProtocol::Vless, 24_080);
    inbound.enable = false;
    inbound.remark = format!("xui-rs-live-{}", std::process::id());
    inbound.settings = json!({
        "clients": [],
        "decryption": "none",
        "fallbacks": []
    });
    inbound.stream_settings = json!({
        "network": "tcp",
        "security": "none",
        "tcpSettings": {
            "acceptProxyProtocol": false,
            "header": { "type": "none" }
        }
    });
    inbound.sniffing = json!({
        "enabled": false,
        "destOverride": [],
        "metadataOnly": false,
        "routeOnly": false
    });
    inbound
}

fn http_operations(document: &OpenApiDocument) -> BTreeSet<String> {
    const METHODS: &[&str] = &[
        "get", "head", "post", "put", "patch", "delete", "options", "trace",
    ];
    document
        .as_value()
        .get("paths")
        .and_then(serde_json::Value::as_object)
        .into_iter()
        .flat_map(|paths| paths.iter())
        .flat_map(|(path, item)| {
            METHODS
                .iter()
                .filter(|method| item.get(**method).is_some())
                .map(move |method| format!("{method} {}", normalized_path_template(path)))
        })
        .collect()
}

/// Collects panel operations present in Go-router snapshots but absent from `OpenAPI`.
/// Normalizes route parameters so they can be compared with the live API document.
fn source_only_panel_operations() -> TestResult<BTreeSet<String>> {
    const ROUTE_SNAPSHOTS: &[&str] = &[
        include_str!("../spec/3x-ui-v3.8.5.clients-routes.json"),
        include_str!("../spec/3x-ui-v3.8.5.hosts-routes.json"),
        include_str!("../spec/3x-ui-v3.8.5.inbounds-routes.json"),
        include_str!("../spec/3x-ui-v3.8.5.remaining-http-routes.json"),
        include_str!("../spec/3x-ui-v3.8.5.server-routes.json"),
        include_str!("../spec/3x-ui-v3.8.5.settings-routes.json"),
        include_str!("../spec/3x-ui-v3.8.5.subscription-balancers-routes.json"),
    ];
    let mut operations = BTreeSet::new();
    for snapshot in ROUTE_SNAPSHOTS {
        let document: serde_json::Value = serde_json::from_str(snapshot)?;
        for route in document["routes"].as_array().into_iter().flatten() {
            if route["openapi"] == false {
                let path = route["path"].as_str().unwrap_or_default();
                let method = route["method"].as_str().unwrap_or_default();
                if path.starts_with("/panel/") {
                    operations.insert(format!("{method} {}", normalized_path_template(path)));
                }
            }
        }
    }
    Ok(operations)
}

fn normalized_path_template(path: &str) -> String {
    path.split('/')
        .map(|segment| {
            if segment.starts_with('{') && segment.ends_with('}') {
                "{}"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}
