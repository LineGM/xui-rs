//! Panel and Xray settings APIs.

mod api;
mod models;

#[cfg(test)]
mod contract_tests;

pub use api::{SettingsApi, XraySettingsApi};
pub use models::*;
