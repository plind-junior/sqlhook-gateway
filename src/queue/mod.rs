pub mod sqlite;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::error::AppResult;

/// One row of the `jobs` table when claimed by a worker.
#[derive(Debug, Clone)]
pub struct Job {
    pub id: String,
    pub route_id: String,
    pub source_path: String,
    pub raw_payload: String,
    pub attempts: u32,
}

/// Outcome a worker reports back to the queue after processing a job.
pub enum JobOutcome {
    /// Delivered successfully — mark done.
    Done,
    /// Failed, but should be retried. Worker supplies the next visibility time.
    Retry {
        visible_at: DateTime<Utc>,
        last_error: Option<String>,
        last_response_code: Option<i64>,
    },
    /// Failed and out of budget — move to dead-letter.
    Dead {
        last_error: Option<String>,
        last_response_code: Option<i64>,
    },
}

#[async_trait]
pub trait Queue: Send + Sync + 'static {
    async fn enqueue(&self, route_id: &str, source_path: &str, raw_payload: &str) -> AppResult<String>;
    async fn claim_one(&self) -> AppResult<Option<Job>>;
    async fn report(&self, job_id: &str, outcome: JobOutcome) -> AppResult<()>;
    async fn record_delivery(&self, delivery: DeliveryRecord) -> AppResult<()>;
}

pub struct DeliveryRecord {
    pub job_id: String,
    pub route_id: String,
    pub attempt: u32,
    pub transformed_payload: Option<String>,
    pub destination_url: String,
    pub success: bool,
    pub response_code: Option<i64>,
    pub response_body: Option<String>,
    pub error: Option<String>,
    pub duration_ms: i64,
}
