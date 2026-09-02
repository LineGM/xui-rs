use serde::{Deserialize, Serialize};

/// Strategy used by a generated JSON-subscription balancer.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum SubscriptionBalancerStrategy {
    /// Chooses the least-loaded eligible outbound.
    LeastLoad,
    /// Chooses the lowest-latency eligible outbound.
    LeastPing,
    /// Chooses an eligible outbound randomly.
    #[default]
    Random,
    /// Cycles through eligible outbounds.
    RoundRobin,
    /// A newer strategy not understood by this SDK version.
    #[serde(other)]
    Unknown,
}

impl SubscriptionBalancerStrategy {
    pub(crate) const fn as_str(self) -> Option<&'static str> {
        match self {
            Self::LeastLoad => Some("leastLoad"),
            Self::LeastPing => Some("leastPing"),
            Self::Random => Some("random"),
            Self::RoundRobin => Some("roundRobin"),
            Self::Unknown => None,
        }
    }
}

/// Persisted subscription balancer returned by 3x-ui.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SubscriptionBalancer {
    /// Database identifier.
    pub id: i64,
    /// User-visible name.
    pub remark: String,
    /// Selection strategy.
    pub strategy: SubscriptionBalancerStrategy,
    /// Inbounds whose generated outbounds participate.
    pub inbound_ids: Vec<i64>,
    /// One-based order in generated subscription documents.
    pub sort_order: i32,
    /// Whether the balancer is emitted.
    pub enabled: bool,
    /// Creation timestamp in Unix milliseconds.
    pub created_at: i64,
    /// Update timestamp in Unix milliseconds.
    pub updated_at: i64,
}

/// Writable subscription-balancer fields.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscriptionBalancerInput {
    /// User-visible name.
    pub remark: String,
    /// Selection strategy.
    pub strategy: SubscriptionBalancerStrategy,
    /// Inbounds whose generated outbounds participate.
    pub inbound_ids: Vec<i64>,
    /// One-based order in generated subscription documents.
    pub sort_order: i32,
    /// Explicit enabled state. `None` uses the create default or preserves it on update.
    pub enabled: Option<bool>,
}

impl SubscriptionBalancerInput {
    /// Creates an enabled random balancer at sort position one.
    pub fn new(remark: impl Into<String>, inbound_ids: Vec<i64>) -> Self {
        Self {
            remark: remark.into(),
            strategy: SubscriptionBalancerStrategy::Random,
            inbound_ids,
            sort_order: 1,
            enabled: Some(true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strategies_and_constructor_match_wire_defaults() {
        assert_eq!(
            SubscriptionBalancerStrategy::LeastLoad.as_str(),
            Some("leastLoad")
        );
        assert_eq!(
            SubscriptionBalancerStrategy::LeastPing.as_str(),
            Some("leastPing")
        );
        assert_eq!(
            SubscriptionBalancerStrategy::Random.as_str(),
            Some("random")
        );
        assert_eq!(
            SubscriptionBalancerStrategy::RoundRobin.as_str(),
            Some("roundRobin")
        );
        assert_eq!(SubscriptionBalancerStrategy::Unknown.as_str(), None);

        let input = SubscriptionBalancerInput::new("EU pool", vec![7, 9]);
        assert_eq!(input.remark, "EU pool");
        assert_eq!(input.strategy, SubscriptionBalancerStrategy::Random);
        assert_eq!(input.inbound_ids, [7, 9]);
        assert_eq!(input.sort_order, 1);
        assert_eq!(input.enabled, Some(true));
    }
}
