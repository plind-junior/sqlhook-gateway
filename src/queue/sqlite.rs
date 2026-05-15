//! SQLite-backed implementation of the Queue trait.
//!
//! Claim semantics: a single-statement UPDATE moves one ready job from `pending`
//! to `processing` and returns its row. SQLite serializes writes, so two workers
//! racing for the same job is safe — only one wins the UPDATE.

use async_trait::async_trait;
use chrono::Utc;
use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

use super::{DeliveryRecord, Job, JobOutcome, Queue};
use crate::error::AppResult;

pub struct SqliteQueue {
    pool: SqlitePool,
}

impl SqliteQueue {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl Queue for SqliteQueue {
    async fn enqueue(&self, route_id: &str, source_path: &str, raw_payload: &str) -> AppResult<String> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO jobs (id, route_id, source_path, raw_payload, status, attempts, visible_at, created_at, updated_at)
            VALUES (?, ?, ?, ?, 'pending', 0, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(route_id)
        .bind(source_path)
        .bind(raw_payload)
        .bind(&now)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    async fn claim_one(&self) -> AppResult<Option<Job>> {
        let now = Utc::now().to_rfc3339();

        let row: Option<(String, String, String, String, i64)> = sqlx::query_as(
            r#"
            UPDATE jobs
            SET status = 'processing', attempts = attempts + 1, updated_at = ?
            WHERE id = (
                SELECT id FROM jobs
                WHERE status = 'pending' AND visible_at <= ?
                ORDER BY visible_at
                LIMIT 1
            )
            RETURNING id, route_id, source_path, raw_payload, attempts
            "#,
        )
        .bind(&now)
        .bind(&now)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(id, route_id, source_path, raw_payload, attempts)| Job {
            id,
            route_id,
            source_path,
            raw_payload,
            attempts: attempts as u32,
        }))
    }

    async fn report(&self, job_id: &str, outcome: JobOutcome) -> AppResult<()> {
        let now = Utc::now().to_rfc3339();

        match outcome {
            JobOutcome::Done => {
                sqlx::query("UPDATE jobs SET status = 'done', updated_at = ? WHERE id = ?")
                    .bind(&now)
                    .bind(job_id)
                    .execute(&self.pool)
                    .await?;
            }
            JobOutcome::Retry { visible_at, last_error, last_response_code } => {
                sqlx::query(
                    r#"
                    UPDATE jobs
                    SET status = 'pending',
                        visible_at = ?,
                        last_error = ?,
                        last_response_code = ?,
                        updated_at = ?
                    WHERE id = ?
                    "#,
                )
                .bind(visible_at.to_rfc3339())
                .bind(last_error)
                .bind(last_response_code)
                .bind(&now)
                .bind(job_id)
                .execute(&self.pool)
                .await?;
            }
            JobOutcome::Dead { last_error, last_response_code } => {
                sqlx::query(
                    r#"
                    UPDATE jobs
                    SET status = 'dead',
                        last_error = ?,
                        last_response_code = ?,
                        updated_at = ?
                    WHERE id = ?
                    "#,
                )
                .bind(last_error)
                .bind(last_response_code)
                .bind(&now)
                .bind(job_id)
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    async fn record_delivery(&self, delivery: DeliveryRecord) -> AppResult<()> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO deliveries
                (id, job_id, route_id, attempt, transformed_payload, destination_url,
                 success, response_code, response_body, error, duration_ms, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&delivery.job_id)
        .bind(&delivery.route_id)
        .bind(delivery.attempt as i64)
        .bind(&delivery.transformed_payload)
        .bind(&delivery.destination_url)
        .bind(delivery.success as i64)
        .bind(delivery.response_code)
        .bind(&delivery.response_body)
        .bind(&delivery.error)
        .bind(delivery.duration_ms)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
