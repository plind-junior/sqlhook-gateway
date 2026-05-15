//! Worker loop: claim a job, transform, deliver, report outcome.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tokio::time::sleep;

use crate::config::ResolvedRoute;
use crate::delivery::Deliverer;
use crate::metrics::Metrics;
use crate::queue::{DeliveryRecord, Job, JobOutcome, Queue};
use crate::retry::{backoff_ms, should_retry};
use crate::transform::Transform;

pub struct Worker {
    queue: Arc<dyn Queue>,
    deliverer: Arc<Deliverer>,
    transforms: Arc<HashMap<String, Transform>>,
    routes: Arc<HashMap<String, Arc<RouteSnapshot>>>,
    metrics: Arc<Metrics>,
    poll_interval: Duration,
}

/// Read-only snapshot of the bits of a route the worker needs.
pub struct RouteSnapshot {
    pub id: String,
    pub destination: crate::config::DestinationSpec,
    pub retry: crate::config::RetrySpec,
}

impl Worker {
    pub fn new(
        queue: Arc<dyn Queue>,
        deliverer: Arc<Deliverer>,
        transforms: Arc<HashMap<String, Transform>>,
        routes: Arc<HashMap<String, Arc<RouteSnapshot>>>,
        metrics: Arc<Metrics>,
        poll_interval: Duration,
    ) -> Self {
        Self { queue, deliverer, transforms, routes, metrics, poll_interval }
    }

    /// Build worker state from the loaded config. Compiles each route's transform.
    pub fn snapshots_from_config(
        routes: &HashMap<String, ResolvedRoute>,
    ) -> Result<(HashMap<String, Transform>, HashMap<String, Arc<RouteSnapshot>>), crate::error::AppError> {
        let mut transforms = HashMap::new();
        let mut snapshots = HashMap::new();
        for r in routes.values() {
            let t = Transform::compile(&r.transform)?;
            transforms.insert(r.id.clone(), t);
            snapshots.insert(
                r.id.clone(),
                Arc::new(RouteSnapshot {
                    id: r.id.clone(),
                    destination: r.destination.clone(),
                    retry: r.retry.clone(),
                }),
            );
        }
        Ok((transforms, snapshots))
    }

    pub async fn run(&self) {
        loop {
            match self.queue.claim_one().await {
                Ok(Some(job)) => self.process(job).await,
                Ok(None) => sleep(self.poll_interval).await,
                Err(e) => {
                    tracing::error!(error = %e, "queue claim failed");
                    sleep(self.poll_interval).await;
                }
            }
        }
    }

    async fn process(&self, job: Job) {
        let Some(route) = self.routes.get(&job.route_id).cloned() else {
            tracing::error!(job = %job.id, route = %job.route_id, "no route snapshot for job; marking dead");
            let _ = self.queue.report(&job.id, JobOutcome::Dead {
                last_error: Some("route no longer configured".into()),
                last_response_code: None,
            }).await;
            return;
        };

        let Some(transform) = self.transforms.get(&job.route_id) else {
            tracing::error!(job = %job.id, route = %job.route_id, "no transform for job; marking dead");
            let _ = self.queue.report(&job.id, JobOutcome::Dead {
                last_error: Some("transform missing".into()),
                last_response_code: None,
            }).await;
            return;
        };

        // Parse the raw payload back to JSON.
        let payload: serde_json::Value = match serde_json::from_str(&job.raw_payload) {
            Ok(v) => v,
            Err(e) => {
                let _ = self.queue.report(&job.id, JobOutcome::Dead {
                    last_error: Some(format!("payload re-parse: {e}")),
                    last_response_code: None,
                }).await;
                return;
            }
        };

        // Transform.
        let transformed = match transform.apply(payload) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(job = %job.id, route = %route.id, error = %e, "transform failed");
                let _ = self.queue.report(&job.id, JobOutcome::Dead {
                    last_error: Some(format!("transform: {e}")),
                    last_response_code: None,
                }).await;
                return;
            }
        };

        // Deliver.
        let result = self.deliverer.deliver(&route.destination, &transformed).await;

        let _ = self.queue.record_delivery(DeliveryRecord {
            job_id: job.id.clone(),
            route_id: route.id.clone(),
            attempt: job.attempts,
            transformed_payload: Some(transformed.to_string()),
            destination_url: route.destination.url.clone(),
            success: result.success,
            response_code: result.response_code.map(|c| c as i64),
            response_body: result.response_body.clone(),
            error: result.error.clone(),
            duration_ms: result.duration_ms as i64,
        }).await;

        if result.success {
            self.metrics.delivery_attempts.with_label_values(&[&route.id, "success"]).inc();
            let _ = self.queue.report(&job.id, JobOutcome::Done).await;
            return;
        }

        self.metrics.delivery_attempts.with_label_values(&[&route.id, "failure"]).inc();

        if should_retry(&route.retry, job.attempts) {
            let delay = backoff_ms(&route.retry, job.attempts.saturating_sub(1));
            let visible_at = Utc::now() + chrono::Duration::milliseconds(delay as i64);
            self.metrics.delivery_retries.inc();
            tracing::info!(
                job = %job.id,
                route = %route.id,
                attempts = job.attempts,
                next_delay_ms = delay,
                "scheduling retry"
            );
            let _ = self.queue.report(&job.id, JobOutcome::Retry {
                visible_at,
                last_error: result.error,
                last_response_code: result.response_code.map(|c| c as i64),
            }).await;
        } else {
            self.metrics.delivery_dead.inc();
            tracing::warn!(job = %job.id, route = %route.id, attempts = job.attempts, "moving to dead-letter");
            let _ = self.queue.report(&job.id, JobOutcome::Dead {
                last_error: result.error,
                last_response_code: result.response_code.map(|c| c as i64),
            }).await;
        }
    }
}
