# Panel and Xray settings APIs

3x-ui v3.7.0 splits this surface between `SettingController` and
`XraySettingController`. `Client::settings()` covers the 14 panel-settings,
credential, notification, and API-token routes. `Client::xray_settings()`
covers all 26 Xray settings and integration routes. All 40 operations are
documented by the tagged OpenAPI and independently pinned from the Go routers.

## Full-replacement panel settings

The update endpoint replaces the complete `AllSetting` record. The SDK models
that contract as `PanelSettings`, organized into web, display, Telegram, SMTP,
security, subscription, and LDAP groups. Serde flattens the groups back to the
exact lower-camel 3x-ui body, including acronym spellings such as
`trustedProxyCIDRs`, `tgBotAPIServer`, `subURI`, and `ldapDefaultTotalGB`.

Always fetch `settings().all()` before editing an existing deployment. Starting
from `PanelSettings::default()` would deliberately send zero/blank values as a
full replacement and is only suitable when every required value is filled in.

```rust
use xui_rs::{Client, PanelSettingsUpdate};
async fn example(client: &Client) -> xui_rs::Result<()> {
let view = client.settings().all().await?;
let mut update = PanelSettingsUpdate::new(view.settings);
update.settings.display.page_size = 100;
client.settings().update(&update).await?;
Ok(())
}
```

3x-ui redacts stored Telegram, SMTP, LDAP, and 2FA secrets on reads. A blank
secret in an update means “preserve the stored value”; the three `clear_*`
flags explicitly erase Telegram, SMTP, or LDAP credentials. Disabling 2FA may
also require `two_factor_code`. Secret-bearing settings and requests have
redacted `Debug` implementations.

## API tokens and notification tests

`api_tokens()` returns `ApiTokenMetadata`, which cannot contain plaintext
tokens. `create_api_token()` accepts `ApiTokenCreateRequest` with a typed
admin, monitor, or node-sync scope and optional expiry, then returns
`CreatedApiToken`; copy its token to secure storage immediately because the
panel shows it only once. Delete and enable operations require the expected
scope, matching v3.7.0's confused-deputy protection. `Debug` never prints the
token.

SMTP testing is an intentional exception to the normal 3x-ui envelope: an
authentication or connection failure is HTTP 200 with `success: false`,
`stage`, and `msg`. `test_smtp()` therefore returns `Ok(SmtpTestResult)` for a
completed unsuccessful probe so callers retain the stage. Transport,
authentication, HTTP, and decoding failures still return `Err`.

## Xray settings and nested JSON

The `/panel/api/xray/` endpoint JSON-encodes its object into a string inside the
normal response envelope. `xray_settings().settings()` removes that historical
double encoding and returns `XraySettingsSnapshot`. Likewise, `update()` takes
an `XrayConfig` and serializes the form field automatically.

The SDK keeps Xray documents open-ended because valid fields follow the
installed xray-core rather than only the panel release. Explicit access remains
available through `XrayConfig::as_value`; `Debug` is redacted. WARP and NordVPN
actions return `SensitivePayload`, whose raw string and optional parsed JSON are
available explicitly without appearing in logs.

PIA joins the typed integration surface in v3.7.0: country/region/server
discovery, registration, persisted account state, deletion, and key creation.
Private keys and credentials remain redacted. Geodata file, category, entry,
pagination, and token-validation responses are also fully typed.

## Tests, routing, and remote outbounds

Outbound probes accept `serde_json::Value` documents and automatically encode
the nested JSON form fields. `OutboundTestMode` restricts mode selection to the
three source-supported behaviours. Balancer status, override, and route tests
have typed requests and results.

Remote outbound subscriptions use full-replacement input. `url`, cached
outbounds, parsed previews, and refresh results are redacted from `Debug`
because subscription URLs and Xray outbound documents commonly contain access
tokens, passwords, or private keys. Both the normal HTTP DELETE endpoint and
the source-defined POST compatibility alias are exposed.
