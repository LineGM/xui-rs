use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub(crate) struct ApiResponse<T> {
    pub(crate) success: bool,
    #[serde(default)]
    pub(crate) msg: String,
    #[serde(default)]
    pub(crate) obj: Option<T>,
}
