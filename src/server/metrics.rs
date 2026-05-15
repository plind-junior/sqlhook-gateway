use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::http::header;

use super::AppState;

pub async fn metrics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let body = state.metrics.encode();
    (
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        body,
    )
}
