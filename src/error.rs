use reqwest::{Method, StatusCode};
use std::string::FromUtf8Error;
use thiserror::Error;
use url::Url;

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
    /// Returns `true` when authentication must be supplied or refreshed.
    pub fn is_unauthorized(&self) -> bool {
        matches!(self, Self::Unauthorized { .. })
    }
}

/// Result type used throughout the SDK.
pub type Result<T> = std::result::Result<T, Error>;
