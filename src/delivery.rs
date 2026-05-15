//! Outbound HTTP delivery.

use std::time::Duration;

use reqwest::Client;
use serde_json::Value as JsonValue;

use crate::config::DestinationSpec;

pub struct DeliveryResult {
    pub success: bool,
    pub response_code: Option<u16>,
    pub response_body: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

pub struct Deliverer {
    client: Client,
}

impl Deliverer {
    pub fn new() -> Self {
        Self {
            // Per-request timeout is applied at call time; the client-level timeout
            // here is a safety net in case the per-call one is missed.
            client: Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .expect("reqwest client builds"),
        }
    }

    pub async fn deliver(&self, dest: &DestinationSpec, payload: &JsonValue) -> DeliveryResult {
        let start = std::time::Instant::now();
        let result = self
            .client
            .post(&dest.url)
            .timeout(Duration::from_millis(dest.timeout_ms))
            .json(payload)
            .send()
            .await;

        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(resp) => {
                let code = resp.status().as_u16();
                let success = resp.status().is_success();
                let body = resp.text().await.ok();
                DeliveryResult {
                    success,
                    response_code: Some(code),
                    response_body: body,
                    error: None,
                    duration_ms,
                }
            }
            Err(e) => DeliveryResult {
                success: false,
                response_code: e.status().map(|s| s.as_u16()),
                response_body: None,
                error: Some(e.to_string()),
                duration_ms,
            },
        }
    }
}

impl Default for Deliverer {
    fn default() -> Self {
        Self::new()
    }
}
