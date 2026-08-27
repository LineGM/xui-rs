//! A typed, asynchronous SDK for the 3x-ui panel API.
//!
//! API tokens are the preferred authentication mechanism for automation.
//! Cookie sessions remain available for username/password and 2FA login flows.

mod auth;
mod client;
mod error;
mod response;

pub use auth::{AuthApi, CsrfToken, LoginRequest};
pub use client::{AuthenticationKind, Client, ClientBuilder};
pub use error::{Error, Result};
