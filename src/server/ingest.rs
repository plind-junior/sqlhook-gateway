use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;

use crate::signature;
use super::AppState;

/// POST /ingest/{*path}
pub async fn ingest(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let source_path = format!("/{path}");
    let Some(route) = state.config.routes_by_path.get(&source_path) else {
        state.metrics.ingest_unknown_route.inc();
        return (StatusCode::NOT_FOUND, Json(json!({"error": "no route"}))).into_response();
    };

    let header_value = headers
        .get(&route.signature.header)
        .and_then(|v| v.to_str().ok());

    if let Err(e) = signature::verify(&route.signature, &route.secret, body.as_ref(), header_value) {
        state.metrics.ingest_signature_failed.inc();
        tracing::warn!(route = %route.id, error = %e, "signature verification failed");
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "signature"}))).into_response();
    }

    if serde_json::from_slice::<serde_json::Value>(body.as_ref()).is_err() {
        state.metrics.ingest_bad_payload.inc();
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "invalid json"}))).into_response();
    }

    let raw = match std::str::from_utf8(body.as_ref()) {
        Ok(s) => s.to_string(),
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(json!({"error": "invalid utf-8"}))).into_response();
        }
    };

    match state.queue.enqueue(&route.id, &source_path, &raw).await {
        Ok(job_id) => {
            state.metrics.ingest_accepted.inc();
            (StatusCode::ACCEPTED, Json(json!({"status": "accepted", "job_id": job_id}))).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "enqueue failed");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "enqueue failed"}))).into_response()
        }
    }
}
