# Outbound proxies

`Client` and `SubscriptionClient` accept the same explicit [`ProxyConfig`]. A
panel client applies it to ordinary HTTP requests and authenticated WebSocket
connections, so enabling a proxy cannot accidentally leave the real-time
stream on a direct route.

```rust,no_run
use xui_rs::{Client, ProxyConfig, SubscriptionClient};

# fn example() -> xui_rs::Result<()> {
let proxy = ProxyConfig::new("socks5h://proxy.example.com:1080")?
    .with_basic_auth("service", "proxy-password")?;

let panel = Client::builder("https://panel.example.com/secret/")?
    .proxy(proxy.clone())
    .build()?;
let subscriptions = SubscriptionClient::builder("https://sub.example.com:2096")?
    .proxy(proxy)
    .build()?;
# let _ = (panel, subscriptions);
# Ok(())
# }
```

## Protocol and DNS behavior

| URL scheme | Transport | Target hostname resolution |
| --- | --- | --- |
| `http://` | HTTP proxy; WebSocket uses CONNECT | Proxy/HTTP stack |
| `https://` | TLS to the HTTP proxy; WebSocket uses CONNECT | Proxy/HTTP stack |
| `socks5://` | SOCKS5 | Local machine |
| `socks5h://` | SOCKS5 | SOCKS5 proxy |

Use `socks5h` when local DNS must not observe or resolve the panel hostname.
The distinction is enforced for both reqwest HTTP requests and the SDK's
WebSocket transport.

The SDK deliberately ignores ambient `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY`,
and `NO_PROXY` environment variables. This makes routing reproducible across
services, developer shells, and containers. Configure a proxy explicitly with
`proxy`, use the convenience `proxy_url` method for an unauthenticated URL, or
remove an earlier builder setting with `no_proxy`.

## Authentication and redaction

Proxy URLs containing `user:password@` are rejected. Credentials must be added
with `ProxyConfig::with_basic_auth`; they are stored as secret strings and are
never included in `ProxyConfig`, client, or proxy-error `Debug` output.
`ProxyConfig::url` intentionally returns only the credential-free endpoint.

HTTP Basic and SOCKS5 username/password authentication do not by themselves
encrypt credentials. Prefer an HTTPS proxy, or a SOCKS5 proxy reached over a
separately trusted private transport, when the network between the process and
proxy is not trusted.

WebSocket proxy failures use `Error::Proxy` and `ErrorKind::Proxy`. They expose
the target WebSocket URL and proxy scheme for diagnostics, but deliberately
omit the proxy endpoint and credentials. An HTTP proxy's CONNECT status is not
reported by `Error::status`, because that accessor is reserved for responses
from the target panel.

## Cookies, TLS, and timeouts

Proxying does not require a separate cookie implementation. Cookies remain in
the panel client's single standards-compliant jar and are matched against the
target panel URL, never the proxy URL. The WebSocket handshake reads the same
jar, so session replacement, domain/path scoping, `Secure`, and expiry behavior
remain identical between HTTP and WebSocket requests.

`connect_timeout` bounds the complete WebSocket setup: proxy TCP/TLS,
HTTP CONNECT or SOCKS5 negotiation, target TLS, and the WebSocket handshake.
`danger_accept_invalid_certs(true)` also applies to an HTTPS proxy certificate;
use it only when both the target and proxy are explicitly trusted.

