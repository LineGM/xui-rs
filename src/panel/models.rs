use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Runtime `OpenAPI` document embedded in the connected panel.
///
/// The model stays open-ended so extensions and schemas from future panel
/// versions are preserved exactly.
#[derive(Clone, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
pub struct OpenApiDocument(Value);

impl OpenApiDocument {
    /// Borrows the complete `OpenAPI` JSON document.
    pub const fn as_value(&self) -> &Value {
        &self.0
    }

    /// Consumes the wrapper and returns the complete JSON document.
    pub fn into_value(self) -> Value {
        self.0
    }

    /// Returns the declared `OpenAPI` version string.
    pub fn version(&self) -> Option<&str> {
        self.0.get("openapi").and_then(Value::as_str)
    }

    /// Counts real HTTP operations, excluding documentation-only WebSocket
    /// message pseudo-operations.
    pub fn http_operation_count(&self) -> usize {
        const METHODS: &[&str] = &[
            "get", "head", "post", "put", "patch", "delete", "options", "trace",
        ];
        self.0
            .get("paths")
            .and_then(Value::as_object)
            .into_iter()
            .flat_map(|paths| paths.values())
            .filter_map(Value::as_object)
            .map(|item| {
                METHODS
                    .iter()
                    .filter(|method| item.contains_key(**method))
                    .count()
            })
            .sum()
    }
}

impl From<Value> for OpenApiDocument {
    fn from(value: Value) -> Self {
        Self(value)
    }
}

impl fmt::Debug for OpenApiDocument {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenApiDocument")
            .field("version", &self.version())
            .field("http_operation_count", &self.http_operation_count())
            .finish_non_exhaustive()
    }
}
