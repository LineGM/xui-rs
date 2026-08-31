use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use reqwest::{Method, RequestBuilder, Response};
use tracing::debug;
use url::Url;

use crate::{Error, Result};

static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy)]
pub(crate) enum ErrorUrlPolicy {
    Preserve,
    Redact,
}

pub(crate) async fn send_request(
    request: RequestBuilder,
    method: &Method,
    diagnostic_url: &Url,
    error_url_policy: ErrorUrlPolicy,
) -> Result<Response> {
    let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
    let started = Instant::now();
    debug!(
        target: "xui_rs::transport",
        request_id,
        method = %method,
        "sending 3x-ui HTTP request"
    );
    let response = match request.send().await {
        Ok(response) => response,
        Err(source) => {
            debug!(
                target: "xui_rs::transport",
                request_id,
                method = %method,
                elapsed_ms = elapsed_millis(started.elapsed()),
                outcome = "transport_error",
                "3x-ui HTTP request failed"
            );
            return Err(Error::Transport {
                method: method.clone(),
                url: Box::new(diagnostic_url.clone()),
                source: sanitize_reqwest_error(source, error_url_policy),
            });
        }
    };
    debug!(
        target: "xui_rs::transport",
        request_id,
        method = %method,
        status = response.status().as_u16(),
        elapsed_ms = elapsed_millis(started.elapsed()),
        outcome = "response",
        "received 3x-ui HTTP response headers"
    );
    Ok(response)
}

pub(crate) async fn read_response_body(
    mut response: Response,
    method: &Method,
    diagnostic_url: &Url,
    limit: usize,
    error_url_policy: ErrorUrlPolicy,
) -> Result<Vec<u8>> {
    if *method == Method::HEAD {
        return Ok(Vec::new());
    }

    let content_length = response.content_length();
    if content_length.is_some_and(|length| length > usize_as_u64(limit)) {
        return Err(response_too_large(
            method,
            diagnostic_url,
            limit,
            content_length,
        ));
    }
    let capacity = content_length
        .and_then(|length| usize::try_from(length).ok())
        .unwrap_or_default()
        .min(limit);
    let mut bytes = Vec::with_capacity(capacity);
    while let Some(chunk) = response.chunk().await.map_err(|source| Error::Transport {
        method: method.clone(),
        url: Box::new(diagnostic_url.clone()),
        source: sanitize_reqwest_error(source, error_url_policy),
    })? {
        if chunk.len() > limit.saturating_sub(bytes.len()) {
            return Err(response_too_large(
                method,
                diagnostic_url,
                limit,
                content_length,
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn response_too_large(
    method: &Method,
    diagnostic_url: &Url,
    limit: usize,
    content_length: Option<u64>,
) -> Error {
    Error::ResponseTooLarge {
        method: method.clone(),
        url: Box::new(diagnostic_url.clone()),
        limit,
        content_length,
    }
}

fn sanitize_reqwest_error(source: reqwest::Error, policy: ErrorUrlPolicy) -> reqwest::Error {
    match policy {
        ErrorUrlPolicy::Preserve => source,
        ErrorUrlPolicy::Redact => source.without_url(),
    }
}

fn elapsed_millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn usize_as_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
