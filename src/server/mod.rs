pub mod api;
pub mod health;
pub mod ingest;
pub mod metrics;

use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use sqlx::sqlite::SqlitePool;

use crate::config::LoadedConfig;
use crate::metrics::Metrics;
use crate::queue::Queue;

pub struct AppState {
    pub config: LoadedConfig,
    pub queue: Arc<dyn Queue>,
    pub metrics: Arc<Metrics>,
    /// Read-only handle on the storage pool for dashboard queries.
    /// Optional so we can still build state in tests that don't need it.
    pub read_pool: Option<SqlitePool>,
}

impl AppState {
    pub fn pool(&self) -> Option<&SqlitePool> {
        self.read_pool.as_ref()
    }
}

pub fn build_router(state: Arc<AppState>) -> Router {
    let api = Router::new()
        .route("/stats", get(api::stats))
        .route("/jobs", get(api::list_jobs))
        .route("/jobs/{id}", get(api::get_job))
        .route("/jobs/{id}/deliveries", get(api::list_deliveries))
        .route("/jobs/{id}/replay", post(api::replay))
        .route("/routes", get(api::list_routes));

    Router::new()
        .route("/health", get(health::health))
        .route("/metrics", get(metrics::metrics))
        .route("/ingest/{*path}", post(ingest::ingest))
        .nest("/api", api)
        .with_state(state)
}
