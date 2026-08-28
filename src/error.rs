//! Typed SDK errors and transport-independent error introspection.

use reqwest::{Method, StatusCode};
use std::{fmt, string::FromUtf8Error, time::Duration};
use thiserror::Error;
use url::Url;

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
            Self::Encode => "encode",
            Self::Api => "api",
            Self::Decode => "decode",
            Self::Utf8 => "utf8",
            Self::WebSocketConnectTimeout => "websocket_connect_timeout",
            Self::WebSocket => "websocket",
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

    /// A WebSocket text message did not match its source-defined JSON shape.
    #[error("could not decode WebSocket {message_type:?} event: {source}")]
    EventDecode {
        /// Message name when the envelope was valid enough to identify it.
        message_type: Option<String>,
        /// Underlying JSON decoding error.
        #[source]
        source: serde_json::Error,
    },

    /// The panel sent a data-frame format not used by the v3.6.0 protocol.
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
            Self::Encode { .. } => ErrorKind::Encode,
            Self::Api { .. } => ErrorKind::Api,
            Self::Decode { .. } => ErrorKind::Decode,
            Self::Utf8 { .. } => ErrorKind::Utf8,
            Self::WebSocketConnectTimeout { .. } => ErrorKind::WebSocketConnectTimeout,
            Self::WebSocket { .. } => ErrorKind::WebSocket,
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
            | Self::Api { url, .. }
            | Self::Decode { url, .. }
            | Self::Utf8 { url, .. }
            | Self::WebSocketConnectTimeout { url, .. }
            | Self::WebSocket { url, .. }
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
            _ => false,
        }
    }
}

/// Result type used throughout the SDK.
pub type Result<T> = std::result::Result<T, Error>;
