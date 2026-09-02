//! Typed SDK errors and transport-independent error introspection.

use reqwest::{Method, StatusCode};
use std::{fmt, string::FromUtf8Error, time::Duration};
use thiserror::Error;
use url::Url;

use crate::{ProxyError, ProxyScheme};

/// Stable category for an [`enum@Error`] without exposing its variant fields.
///
/// This is useful for metrics and policy decisions that should not depend on
/// the concrete error payload. Use [`Error::status`], [`Error::method`], and
/// [`Error::url`] when the associated HTTP context is needed.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Invalid client or request configuration.
    Configuration,
    /// Missing or expired authentication.
    Unauthorized,
    /// Authorization or CSRF rejection.
    Forbidden,
    /// Non-success HTTP status other than 401 or 403.
    HttpStatus,
    /// HTTP transport or response-body IO failure.
    Transport,
    /// HTTP response body exceeded its configured memory limit.
    ResponseTooLarge,
    /// Request serialization failure.
    Encode,
    /// Application-level rejection in a successful HTTP response.
    Api,
    /// HTTP response JSON decoding failure.
    Decode,
    /// Invalid UTF-8 response body.
    Utf8,
    /// WebSocket connection timeout.
    WebSocketConnectTimeout,
    /// WebSocket handshake, transport, or protocol failure.
    WebSocket,
    /// Explicit outbound proxy connection or negotiation failure.
    Proxy,
    /// WebSocket event JSON decoding failure.
    EventDecode,
    /// Unsupported WebSocket application frame.
    UnexpectedWebSocketFrame,
    /// Successful response missing its required object.
    MissingObject,
}

impl ErrorKind {
    /// Returns a stable snake-case label suitable for logs and metrics.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Configuration => "configuration",
            Self::Unauthorized => "unauthorized",
            Self::Forbidden => "forbidden",
            Self::HttpStatus => "http_status",
            Self::Transport => "transport",
            Self::ResponseTooLarge => "response_too_large",
            Self::Encode => "encode",
            Self::Api => "api",
            Self::Decode => "decode",
            Self::Utf8 => "utf8",
            Self::WebSocketConnectTimeout => "websocket_connect_timeout",
            Self::WebSocket => "websocket",
            Self::Proxy => "proxy",
            Self::EventDecode => "event_decode",
            Self::UnexpectedWebSocketFrame => "unexpected_websocket_frame",
            Self::MissingObject => "missing_object",
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Errors returned by the SDK.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// The client configuration is invalid.
    #[error("invalid client configuration: {0}")]
    Configuration(String),

    /// The panel rejected the caller's credentials or session.
    #[error("{method} {url} requires authentication")]
    Unauthorized {
        /// HTTP method used for the request.
        method: Method,
        /// Request URL.
        url: Box<Url>,
    },

    /// The panel rejected a request, commonly due to an invalid CSRF token.
    #[error("{method} {url} was forbidden")]
    Forbidden {
        /// HTTP method used for the request.
        method: Method,
        /// Request URL.
        url: Box<Url>,
    },

    /// The panel returned an unsuccessful HTTP status.
    #[error("{method} {url} returned HTTP {status}")]
    HttpStatus {
        /// HTTP method used for the request.
        method: Method,
        /// Request URL.
        url: Box<Url>,
        /// Response status.
        status: StatusCode,
    },

    /// The request could not be sent or its body could not be read.
    #[error("request {method} {url} failed: {source}")]
    Transport {
        /// HTTP method used for the request.
        method: Method,
        /// Request URL.
        url: Box<Url>,
        /// Underlying HTTP client error.
        #[source]
        source: reqwest::Error,
    },

    /// The response body exceeded the configured in-memory size limit.
    #[error("response from {method} {url} exceeded the {limit}-byte body limit")]
    ResponseTooLarge {
        /// HTTP method used for the request.
        method: Method,
        /// Request URL, redacted for public subscription routes.
        url: Box<Url>,
        /// Configured maximum response body size in bytes.
        limit: usize,
        /// Server-advertised body size, when a `Content-Length` was present.
        content_length: Option<u64>,
    },

    /// A request value could not be encoded in the endpoint's wire format.
    #[error("could not encode request for {operation}: {source}")]
    Encode {
        /// Human-readable operation name.
        operation: &'static str,
        /// Underlying JSON encoding error.
        #[source]
        source: serde_json::Error,
    },

    /// The panel returned HTTP success with `success: false`.
    #[error("panel rejected {method} {url}: {message}")]
    Api {
        /// HTTP method used for the request.
        method: Method,
        /// Request URL.
        url: Box<Url>,
        /// Human-readable message returned by 3x-ui.
        message: String,
    },

    /// The panel response did not match the documented schema.
    #[error("could not decode response from {method} {url}: {source}")]
    Decode {
        /// HTTP method used for the request.
        method: Method,
        /// Request URL.
        url: Box<Url>,
        /// JSON decoding error.
        #[source]
        source: serde_json::Error,
    },

    /// A text endpoint returned bytes that are not valid UTF-8.
    #[error("response from {method} {url} was not valid UTF-8: {source}")]
    Utf8 {
        /// HTTP method used for the request.
        method: Method,
        /// Redacted request URL.
        url: Box<Url>,
        /// Underlying UTF-8 decoding error.
        #[source]
        source: FromUtf8Error,
    },

    /// Establishing the authenticated WebSocket exceeded the configured
    /// connection timeout.
    #[error("WebSocket connection to {url} timed out after {timeout:?}")]
    WebSocketConnectTimeout {
        /// WebSocket endpoint.
        url: Box<Url>,
        /// Configured connection timeout.
        timeout: Duration,
    },

    /// A WebSocket handshake or frame operation failed.
    #[error("WebSocket operation on {url} failed: {source}")]
    WebSocket {
        /// WebSocket endpoint.
        url: Box<Url>,
        /// Underlying protocol/transport error.
        #[source]
        source: Box<tokio_tungstenite::tungstenite::Error>,
    },

    /// An explicit proxy could not establish a WebSocket tunnel.
    #[error("{scheme} proxy could not connect WebSocket to {url}: {source}")]
    Proxy {
        /// Proxy protocol in use. The proxy endpoint is deliberately omitted.
        scheme: ProxyScheme,
        /// Target WebSocket endpoint, never the proxy endpoint.
        url: Box<Url>,
        /// Underlying proxy transport or negotiation failure.
        #[source]
        source: Box<ProxyError>,
    },

    /// A WebSocket text message did not match its source-defined JSON shape.
    #[error("could not decode WebSocket {message_type:?} event: {source}")]
    EventDecode {
        /// Message name when the envelope was valid enough to identify it.
        message_type: Option<String>,
        /// Underlying JSON decoding error.
        #[source]
        source: serde_json::Error,
    },

    /// The panel sent a data-frame format not used by the v3.7.0 protocol.
    #[error("unexpected WebSocket {kind} frame from {url}")]
    UnexpectedWebSocketFrame {
        /// WebSocket endpoint.
        url: Box<Url>,
        /// Human-readable frame kind.
        kind: &'static str,
    },

    /// A successful response omitted its documented `obj` value.
    #[error("response from {method} {url} did not contain obj")]
    MissingObject {
        /// HTTP method used for the request.
        method: Method,
        /// Request URL.
        url: Box<Url>,
    },
}

impl Error {
    /// Returns the stable category of this error.
    pub const fn kind(&self) -> ErrorKind {
        match self {
            Self::Configuration(_) => ErrorKind::Configuration,
            Self::Unauthorized { .. } => ErrorKind::Unauthorized,
            Self::Forbidden { .. } => ErrorKind::Forbidden,
            Self::HttpStatus { .. } => ErrorKind::HttpStatus,
            Self::Transport { .. } => ErrorKind::Transport,
            Self::ResponseTooLarge { .. } => ErrorKind::ResponseTooLarge,
            Self::Encode { .. } => ErrorKind::Encode,
            Self::Api { .. } => ErrorKind::Api,
            Self::Decode { .. } => ErrorKind::Decode,
            Self::Utf8 { .. } => ErrorKind::Utf8,
            Self::WebSocketConnectTimeout { .. } => ErrorKind::WebSocketConnectTimeout,
            Self::WebSocket { .. } => ErrorKind::WebSocket,
            Self::Proxy { .. } => ErrorKind::Proxy,
            Self::EventDecode { .. } => ErrorKind::EventDecode,
            Self::UnexpectedWebSocketFrame { .. } => ErrorKind::UnexpectedWebSocketFrame,
            Self::MissingObject { .. } => ErrorKind::MissingObject,
        }
    }

    /// Returns the HTTP status that caused this error, when available.
    ///
    /// Authentication errors report 401 or 403 even though they have dedicated
    /// variants. Application-level [`Error::Api`] responses used HTTP success
    /// and therefore return `None`.
    pub const fn status(&self) -> Option<StatusCode> {
        match self {
            Self::Unauthorized { .. } => Some(StatusCode::UNAUTHORIZED),
            Self::Forbidden { .. } => Some(StatusCode::FORBIDDEN),
            Self::HttpStatus { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// Returns the HTTP method associated with the failed operation.
    pub const fn method(&self) -> Option<&Method> {
        match self {
            Self::Unauthorized { method, .. }
            | Self::Forbidden { method, .. }
            | Self::HttpStatus { method, .. }
            | Self::Transport { method, .. }
            | Self::ResponseTooLarge { method, .. }
            | Self::Api { method, .. }
            | Self::Decode { method, .. }
            | Self::Utf8 { method, .. }
            | Self::MissingObject { method, .. } => Some(method),
            _ => None,
        }
    }

    /// Returns the request or WebSocket URL associated with the failure.
    pub fn url(&self) -> Option<&Url> {
        match self {
            Self::Unauthorized { url, .. }
            | Self::Forbidden { url, .. }
            | Self::HttpStatus { url, .. }
            | Self::Transport { url, .. }
            | Self::ResponseTooLarge { url, .. }
            | Self::Api { url, .. }
            | Self::Decode { url, .. }
            | Self::Utf8 { url, .. }
            | Self::WebSocketConnectTimeout { url, .. }
            | Self::WebSocket { url, .. }
            | Self::Proxy { url, .. }
            | Self::UnexpectedWebSocketFrame { url, .. }
            | Self::MissingObject { url, .. } => Some(url),
            Self::Configuration(_) | Self::Encode { .. } | Self::EventDecode { .. } => None,
        }
    }

    /// Returns `true` when authentication must be supplied or refreshed.
    pub const fn is_unauthorized(&self) -> bool {
        matches!(self, Self::Unauthorized { .. })
    }

    /// Returns `true` when the operation was rejected with HTTP 403.
    pub const fn is_forbidden(&self) -> bool {
        matches!(self, Self::Forbidden { .. })
    }

    /// Returns `true` when the panel asked the caller to slow down.
    pub const fn is_rate_limited(&self) -> bool {
        matches!(self.status(), Some(StatusCode::TOO_MANY_REQUESTS))
    }

    /// Returns `true` when the response body exceeded its configured limit.
    pub const fn is_response_too_large(&self) -> bool {
        matches!(self, Self::ResponseTooLarge { .. })
    }

    /// Returns the configured response body limit that was exceeded.
    pub const fn response_body_limit(&self) -> Option<usize> {
        match self {
            Self::ResponseTooLarge { limit, .. } => Some(*limit),
            _ => None,
        }
    }

    /// Returns the server-advertised `Content-Length` for an oversized body.
    pub const fn advertised_content_length(&self) -> Option<u64> {
        match self {
            Self::ResponseTooLarge { content_length, .. } => *content_length,
            _ => None,
        }
    }

    /// Returns `true` for an HTTP 5xx response.
    pub fn is_server_error(&self) -> bool {
        self.status().is_some_and(|status| status.is_server_error())
    }

    /// Returns `true` when an HTTP or WebSocket transport timed out.
    pub fn is_timeout(&self) -> bool {
        match self {
            Self::Transport { source, .. } => source.is_timeout(),
            Self::WebSocketConnectTimeout { .. } => true,
            Self::WebSocket { source, .. } => matches!(
                source.as_ref(),
                tokio_tungstenite::tungstenite::Error::Io(error)
                    if error.kind() == std::io::ErrorKind::TimedOut
            ),
            Self::Proxy { source, .. } => source.is_timeout(),
            _ => false,
        }
    }
}

/// Result type used throughout the SDK.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    fn endpoint() -> Url {
        Url::parse("https://panel.example.com/secret/api").unwrap()
    }

    #[test]
    fn every_error_kind_has_a_stable_display_label() {
        let values = [
            (ErrorKind::Configuration, "configuration"),
            (ErrorKind::Unauthorized, "unauthorized"),
            (ErrorKind::Forbidden, "forbidden"),
            (ErrorKind::HttpStatus, "http_status"),
            (ErrorKind::Transport, "transport"),
            (ErrorKind::ResponseTooLarge, "response_too_large"),
            (ErrorKind::Encode, "encode"),
            (ErrorKind::Api, "api"),
            (ErrorKind::Decode, "decode"),
            (ErrorKind::Utf8, "utf8"),
            (
                ErrorKind::WebSocketConnectTimeout,
                "websocket_connect_timeout",
            ),
            (ErrorKind::WebSocket, "websocket"),
            (ErrorKind::Proxy, "proxy"),
            (ErrorKind::EventDecode, "event_decode"),
            (
                ErrorKind::UnexpectedWebSocketFrame,
                "unexpected_websocket_frame",
            ),
            (ErrorKind::MissingObject, "missing_object"),
        ];
        for (kind, label) in values {
            assert_eq!(kind.as_str(), label);
            assert_eq!(kind.to_string(), label);
        }
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn variants_expose_consistent_http_and_socket_context() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        let transport_source = reqwest::get(format!("http://{address}")).await.unwrap_err();
        let json_source = || serde_json::from_str::<serde_json::Value>("{").unwrap_err();
        let utf8_source = String::from_utf8(vec![0xff]).unwrap_err();
        let errors = vec![
            (
                Error::Configuration("bad configuration".into()),
                ErrorKind::Configuration,
                false,
            ),
            (
                Error::Unauthorized {
                    method: Method::GET,
                    url: Box::new(endpoint()),
                },
                ErrorKind::Unauthorized,
                true,
            ),
            (
                Error::Forbidden {
                    method: Method::POST,
                    url: Box::new(endpoint()),
                },
                ErrorKind::Forbidden,
                true,
            ),
            (
                Error::HttpStatus {
                    method: Method::PUT,
                    url: Box::new(endpoint()),
                    status: StatusCode::BAD_GATEWAY,
                },
                ErrorKind::HttpStatus,
                true,
            ),
            (
                Error::Transport {
                    method: Method::GET,
                    url: Box::new(endpoint()),
                    source: transport_source,
                },
                ErrorKind::Transport,
                true,
            ),
            (
                Error::ResponseTooLarge {
                    method: Method::GET,
                    url: Box::new(endpoint()),
                    limit: 1024,
                    content_length: None,
                },
                ErrorKind::ResponseTooLarge,
                true,
            ),
            (
                Error::Encode {
                    operation: "test",
                    source: json_source(),
                },
                ErrorKind::Encode,
                false,
            ),
            (
                Error::Api {
                    method: Method::PATCH,
                    url: Box::new(endpoint()),
                    message: "rejected".into(),
                },
                ErrorKind::Api,
                true,
            ),
            (
                Error::Decode {
                    method: Method::GET,
                    url: Box::new(endpoint()),
                    source: json_source(),
                },
                ErrorKind::Decode,
                true,
            ),
            (
                Error::Utf8 {
                    method: Method::GET,
                    url: Box::new(endpoint()),
                    source: utf8_source,
                },
                ErrorKind::Utf8,
                true,
            ),
            (
                Error::WebSocketConnectTimeout {
                    url: Box::new(endpoint()),
                    timeout: Duration::from_millis(10),
                },
                ErrorKind::WebSocketConnectTimeout,
                true,
            ),
            (
                Error::WebSocket {
                    url: Box::new(endpoint()),
                    source: Box::new(tokio_tungstenite::tungstenite::Error::Io(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "timed out",
                    ))),
                },
                ErrorKind::WebSocket,
                true,
            ),
            (
                Error::Proxy {
                    scheme: ProxyScheme::Http,
                    url: Box::new(endpoint()),
                    source: Box::new(ProxyError::Io(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "timed out",
                    ))),
                },
                ErrorKind::Proxy,
                true,
            ),
            (
                Error::EventDecode {
                    message_type: Some("status".into()),
                    source: json_source(),
                },
                ErrorKind::EventDecode,
                false,
            ),
            (
                Error::UnexpectedWebSocketFrame {
                    url: Box::new(endpoint()),
                    kind: "binary data",
                },
                ErrorKind::UnexpectedWebSocketFrame,
                true,
            ),
            (
                Error::MissingObject {
                    method: Method::GET,
                    url: Box::new(endpoint()),
                },
                ErrorKind::MissingObject,
                true,
            ),
        ];

        for (error, kind, has_url) in errors {
            assert_eq!(error.kind(), kind);
            assert_eq!(error.url().is_some(), has_url);
            assert!(!error.to_string().is_empty());
            if error.method().is_some() {
                assert!(has_url);
            }
        }
    }

    #[test]
    fn predicates_cover_positive_and_negative_paths() {
        let forbidden = Error::Forbidden {
            method: Method::POST,
            url: Box::new(endpoint()),
        };
        assert!(forbidden.is_forbidden());
        assert_eq!(forbidden.status(), Some(StatusCode::FORBIDDEN));

        let server = Error::HttpStatus {
            method: Method::GET,
            url: Box::new(endpoint()),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        };
        assert!(server.is_server_error());
        assert!(!server.is_rate_limited());

        let plain = Error::Configuration("plain".into());
        assert_eq!(plain.status(), None);
        assert_eq!(plain.method(), None);
        assert_eq!(plain.url(), None);
        assert!(!plain.is_unauthorized());
        assert!(!plain.is_forbidden());
        assert!(!plain.is_response_too_large());
        assert_eq!(plain.response_body_limit(), None);
        assert_eq!(plain.advertised_content_length(), None);
        assert!(!plain.is_server_error());
        assert!(!plain.is_timeout());
    }
}
