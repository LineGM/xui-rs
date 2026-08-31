# Errors and retry policy

Every fallible operation returns `xui_rs::Result<T>`, whose error type is the
non-exhaustive `xui_rs::Error` enum. Applications can inspect individual
variants when they need their complete payload, but routine policy and metrics
do not need to destructure them.

## Stable introspection

`Error::kind` returns a copyable `ErrorKind`; `as_str()` and `Display` provide
stable snake-case labels for logs and metrics. The remaining helpers expose
context consistently across HTTP and WebSocket operations:

- `status()` returns 401 and 403 for the dedicated authentication variants and
  the status carried by `HttpStatus`;
- `method()` and `url()` return request context when the error has it;
- `is_unauthorized()` and `is_forbidden()` identify authentication and CSRF or
  authorization failures;
- `is_rate_limited()` identifies HTTP 429;
- `is_response_too_large()`, `response_body_limit()`, and
  `advertised_content_length()` expose bounded-body failures without matching
  their variant;
- `is_server_error()` identifies HTTP 5xx;
- `is_timeout()` covers both `reqwest` timeouts and the WebSocket connection
  timeout.

```rust
use reqwest::StatusCode;
use xui_rs::{Error, ErrorKind};

fn classify(error: &Error) -> &'static str {
    if error.is_unauthorized() {
        "reauthenticate"
    } else if error.is_rate_limited() {
        "back off"
    } else if error.is_timeout() || error.is_server_error() {
        "transient"
    } else {
        match error.kind() {
            ErrorKind::Configuration | ErrorKind::Encode => "fix request",
            ErrorKind::Decode | ErrorKind::EventDecode => "contract mismatch",
            _ if error.status() == Some(StatusCode::NOT_FOUND) => "unsupported route",
            _ => "inspect",
        }
    }
}

fn main() {
    let error = Error::Configuration("example".to_owned());
    assert_eq!(classify(&error), "fix request");
}
```

`ErrorKind` and `Error` are non-exhaustive. Keep a fallback arm so an
application remains source-compatible when the SDK learns about a new failure
class.

## Retry deliberately

The SDK does not automatically retry API operations. A timeout, 429, or 5xx is
only evidence that a failure may be transient; it does not prove that a
mutation was not applied. Before retrying, also inspect `method()` and the
operation's idempotency:

| Operation | Suggested response |
|---|---|
| Read-only GET/HEAD | Retry with bounded exponential backoff and jitter |
| Idempotent replacement | Re-fetch state, then retry if still required |
| Create/import/delete/action | Reconcile server state before retrying |
| `Unauthorized` | Obtain new credentials/session under application policy |
| WebSocket disconnect | Refresh HTTP snapshots, then call `reconnect()` |

The one internal CSRF retry is safe because 3x-ui rejects the first unsafe
request before its handler executes.

## Sources and diagnostics

`Error` implements `std::error::Error`; transport and decoding variants retain
their original source for diagnostics. `Display` is concise and contextual,
while `Debug` includes typed error structure. API response bodies are never
attached wholesale to an error.

Request URLs can contain a custom panel base path and caller-controlled path
segments. Most contextual `Display` messages include that URL, and `url()`
provides structured access. Treat error values as diagnostic data and apply the
application's privacy policy before sending them to shared logs or telemetry.
Standalone subscription-client errors replace the secret subscription
identifier with a fixed redacted segment.

See the [transport guide](transport.md) for response-body limits and the
privacy contract of SDK-generated `tracing` events.
