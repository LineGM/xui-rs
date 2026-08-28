//! Public standalone subscription-server client.

mod client;
mod models;

pub use client::{SubscriptionClient, SubscriptionClientBuilder};
pub use models::{
    SubscriptionDecodeError, SubscriptionDocument, SubscriptionInfo, SubscriptionJson,
    SubscriptionMetadata, SubscriptionResponse, SubscriptionTraffic,
};
