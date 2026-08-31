# Transport safety and observability

Every HTTP response that xui-rs materializes in memory has an explicit byte
limit. The defaults separate ordinary structured responses from intentionally
large database downloads:

| Response class | Default | Builder setting |
|---|---:|---|
| Panel API, logs, runtime OpenAPI | 64 MiB | `ClientBuilder::response_body_limit` |
| Database and migration downloads | 512 MiB | `ClientBuilder::download_body_limit` |
| Public subscription documents | 64 MiB | `SubscriptionClientBuilder::response_body_limit` |

The constants `DEFAULT_API_RESPONSE_BODY_LIMIT`,
`DEFAULT_DOWNLOAD_RESPONSE_BODY_LIMIT`, and
`DEFAULT_SUBSCRIPTION_RESPONSE_BODY_LIMIT` expose these defaults. The built
clients also report their effective settings through matching accessor
methods.

```rust
use xui_rs::{Client, SubscriptionClient};

fn main() -> xui_rs::Result<()> {
    let panel = Client::builder("https://panel.example.com/secret/")?
        .response_body_limit(16 * 1024 * 1024)
        .download_body_limit(1024 * 1024 * 1024)
        .build()?;
    let subscriptions = SubscriptionClient::builder("https://panel.example.com")?
        .response_body_limit(32 * 1024 * 1024)
        .build()?;

    assert_eq!(panel.response_body_limit(), 16 * 1024 * 1024);
    assert_eq!(panel.download_body_limit(), 1024 * 1024 * 1024);
    assert_eq!(subscriptions.response_body_limit(), 32 * 1024 * 1024);
    Ok(())
}
```

## Enforcement

xui-rs rejects an oversized declared `Content-Length` before allocating or
reading the body. It also counts bytes while reading, so chunked responses,
incorrect length headers, and decompressed payloads cannot bypass the limit.
An exact-boundary body is accepted. A `HEAD` response is exempt because its
`Content-Length` describes the corresponding `GET` representation, not a body
received by the SDK.

An overflow returns `ErrorKind::ResponseTooLarge`. Use
`is_response_too_large()`, `response_body_limit()`, and
`advertised_content_length()` for policy and metrics. The advertised length is
`None` when the server omitted it or streamed a chunked response.

Database downloads are returned as `DatabaseFile` and therefore remain fully
in memory. Increase their separate limit only when the deployment's trusted
backup size requires it. xui-rs does not implicitly retry a rejected or
interrupted download.

## Privacy-safe tracing

When an application installs a `tracing` subscriber, xui-rs emits request and
response-header events at `DEBUG` under the `xui_rs::transport` target. Events
contain a process-local `request_id`, HTTP method, outcome, response status,
and response-header latency where applicable.

Transport events deliberately omit the origin, base path, complete URL,
query, headers, cookies, tokens, subscription identifiers, request body, and
response body. The latency measures sending through receipt of response
headers; consuming the response body happens afterward. Error values retain
more diagnostic context, so apply the guidance in the
[error guide](errors.md) before exporting them to shared telemetry.
