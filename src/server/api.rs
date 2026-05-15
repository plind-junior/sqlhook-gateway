//! Read-only operations API for the dashboard.
//!
//! These endpoints are observation-only with one operational action (replay).
//! Route configuration remains in YAML; no mutation endpoints here.

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::AppState;

#[derive(Debug, Deserialize)]
pub struct JobListQuery {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub route: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 { 50 }

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct JobRow {
    pub id: String,
    pub route_id: String,
    pub source_path: String,
    pub status: String,
    pub attempts: i64,
    pub last_error: Option<String>,
    pub last_response_code: Option<i64>,
    pub visible_at: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DeliveryRow {
    pub id: String,
    pub job_id: String,
    pub route_id: String,
    pub attempt: i64,
    pub transformed_payload: Option<String>,
    pub destination_url: String,
    pub success: i64,
    pub response_code: Option<i64>,
    pub response_body: Option<String>,
    pub error: Option<String>,
    pub duration_ms: i64,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct RouteView {
    pub id: String,
    pub source_path: String,
    pub destination_url: String,
    pub timeout_ms: u64,
    pub max_attempts: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub signature_header: String,
}

#[derive(Debug, Serialize, Default)]
pub struct Stats {
    pub jobs_pending: i64,
    pub jobs_processing: i64,
    pub jobs_done: i64,
    pub jobs_dead: i64,
    pub deliveries_total: i64,
    pub deliveries_success: i64,
}

// ---- handlers ----

pub async fn list_jobs(
    State(state): State<Arc<AppState>>,
    Query(q): Query<JobListQuery>,
) -> impl IntoResponse {
    let pool = match state.pool() {
        Some(p) => p,
        None => return internal("no pool"),
    };

    let mut sql = String::from(
        "SELECT id, route_id, source_path, status, attempts, last_error, \
         last_response_code, visible_at, created_at, updated_at FROM jobs",
    );
    let mut filters: Vec<&str> = Vec::new();
    if q.status.is_some() {
        filters.push("status = ?");
    }
    if q.route.is_some() {
        filters.push("route_id = ?");
    }
    if !filters.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&filters.join(" AND "));
    }
    sql.push_str(" ORDER BY updated_at DESC LIMIT ? OFFSET ?");

    let mut query = sqlx::query_as::<_, JobRow>(&sql);
    if let Some(s) = &q.status {
        query = query.bind(s);
    }
    if let Some(r) = &q.route {
        query = query.bind(r);
    }
    query = query.bind(q.limit).bind(q.offset);

    match query.fetch_all(pool).await {
        Ok(rows) => Json(json!({"jobs": rows})).into_response(),
        Err(e) => internal(&e.to_string()),
    }
}

pub async fn get_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let pool = match state.pool() {
        Some(p) => p,
        None => return internal("no pool"),
    };

    let row: Option<JobRow> = match sqlx::query_as(
        "SELECT id, route_id, source_path, status, attempts, last_error, \
         last_response_code, visible_at, created_at, updated_at \
         FROM jobs WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(pool)
    .await
    {
        Ok(r) => r,
        Err(e) => return internal(&e.to_string()),
    };

    match row {
        Some(job) => Json(json!({"job": job})).into_response(),
        None => (StatusCode::NOT_FOUND, Json(json!({"error": "job not found"}))).into_response(),
    }
}

pub async fn list_deliveries(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let pool = match state.pool() {
        Some(p) => p,
        None => return internal("no pool"),
    };

    let rows: Vec<DeliveryRow> = match sqlx::query_as(
        "SELECT id, job_id, route_id, attempt, transformed_payload, destination_url, \
         success, response_code, response_body, error, duration_ms, created_at \
         FROM deliveries WHERE job_id = ? ORDER BY created_at DESC",
    )
    .bind(&id)
    .fetch_all(pool)
    .await
    {
        Ok(r) => r,
        Err(e) => return internal(&e.to_string()),
    };

    Json(json!({"deliveries": rows})).into_response()
}

pub async fn list_routes(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut views: Vec<RouteView> = state
        .config
        .routes_by_path
        .values()
        .map(|r| RouteView {
            id: r.id.clone(),
            source_path: r.source_path.clone(),
            destination_url: r.destination.url.clone(),
            timeout_ms: r.destination.timeout_ms,
            max_attempts: r.retry.max_attempts,
            initial_backoff_ms: r.retry.initial_backoff_ms,
            max_backoff_ms: r.retry.max_backoff_ms,
            signature_header: r.signature.header.clone(),
        })
        .collect();
    views.sort_by(|a, b| a.id.cmp(&b.id));
    Json(json!({"routes": views})).into_response()
}

pub async fn stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let pool = match state.pool() {
        Some(p) => p,
        None => return internal("no pool"),
    };

    let counts: Result<Vec<(String, i64)>, _> = sqlx::query_as(
        "SELECT status, COUNT(*) FROM jobs GROUP BY status",
    )
    .fetch_all(pool)
    .await;

    let counts = match counts {
        Ok(c) => c,
        Err(e) => return internal(&e.to_string()),
    };

    let mut stats = Stats::default();
    for (status, n) in counts {
        match status.as_str() {
            "pending" => stats.jobs_pending = n,
            "processing" => stats.jobs_processing = n,
            "done" => stats.jobs_done = n,
            "dead" => stats.jobs_dead = n,
            _ => {}
        }
    }

    let deliveries: Option<(i64, i64)> = match sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(success), 0) FROM deliveries",
    )
    .fetch_optional(pool)
    .await
    {
        Ok(r) => r,
        Err(e) => return internal(&e.to_string()),
    };
    if let Some((total, success)) = deliveries {
        stats.deliveries_total = total;
        stats.deliveries_success = success;
    }

    Json(json!({"stats": stats})).into_response()
}

pub async fn replay(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let pool = match state.pool() {
        Some(p) => p,
        None => return internal("no pool"),
    };

    let now = chrono::Utc::now().to_rfc3339();
    let res = sqlx::query(
        "UPDATE jobs SET status = 'pending', visible_at = ?, attempts = 0, \
         last_error = NULL, last_response_code = NULL, updated_at = ? \
         WHERE id = ? AND status = 'dead'",
    )
    .bind(&now)
    .bind(&now)
    .bind(&id)
    .execute(pool)
    .await;

    match res {
        Ok(r) if r.rows_affected() == 0 => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "no dead job with that id"})),
        )
            .into_response(),
        Ok(_) => Json(json!({"status": "requeued", "job_id": id})).into_response(),
        Err(e) => internal(&e.to_string()),
    }
}

fn internal(msg: &str) -> axum::response::Response {
    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": msg}))).into_response()
}
