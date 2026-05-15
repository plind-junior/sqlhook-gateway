mod cli;
mod config;
mod delivery;
mod error;
mod metrics;
mod queue;
mod retry;
mod server;
mod signature;
mod transform;
mod worker;

use std::collections::HashMap;
use std::io::Read;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use clap::Parser;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::ConnectOptions;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use crate::cli::{Cli, Command};
use crate::config::LoadedConfig;
use crate::metrics::Metrics;
use crate::queue::sqlite::SqliteQueue;
use crate::queue::Queue;
use crate::server::AppState;
use crate::transform::Transform;
use crate::worker::Worker;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let cli = Cli::parse();

    match cli.command {
        Command::ValidateConfig { config } => {
            config::validate(&config).with_context(|| format!("validating {:?}", config))?;
            println!("ok");
            Ok(())
        }
        Command::TransformTest { config, route } => transform_test(&config, &route).await,
        Command::Serve { config } => serve(&config).await,
        Command::Replay { config, job_id } => replay(&config, &job_id).await,
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

async fn serve(config_path: &std::path::Path) -> anyhow::Result<()> {
    let cfg = config::load(config_path).with_context(|| format!("loading {:?}", config_path))?;
    let pool = open_pool(&cfg).await?;
    run_migrations(&pool).await?;

    let queue: Arc<dyn Queue> = Arc::new(SqliteQueue::new(pool.clone()));
    let metrics = Arc::new(Metrics::new());

    let (transforms, snapshots) = Worker::snapshots_from_config(&cfg.routes_by_path)
        .context("compiling transforms")?;
    let transforms = Arc::new(transforms);
    let snapshots = Arc::new(snapshots);

    let poll_interval = Duration::from_millis(cfg.worker.poll_interval_ms);
    let worker_count = cfg.worker.concurrency.max(1);

    let bind_addr = cfg.server.bind.clone();
    let state = Arc::new(AppState {
        config: cfg,
        queue: queue.clone(),
        metrics: metrics.clone(),
        read_pool: Some(pool.clone()),
    });

    // Spawn worker pool.
    let deliverer = Arc::new(crate::delivery::Deliverer::new());
    for i in 0..worker_count {
        let worker = Worker::new(
            queue.clone(),
            deliverer.clone(),
            transforms.clone(),
            snapshots.clone(),
            metrics.clone(),
            poll_interval,
        );
        tokio::spawn(async move {
            tracing::info!(worker = i, "worker started");
            worker.run().await;
        });
    }

    let app = server::build_router(state);
    let listener = TcpListener::bind(&bind_addr).await
        .with_context(|| format!("binding {bind_addr}"))?;
    tracing::info!(bind = %bind_addr, "sqlhook serving");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn transform_test(config_path: &std::path::Path, route_id: &str) -> anyhow::Result<()> {
    let cfg = config::load(config_path)?;
    let route = cfg
        .routes_by_path
        .values()
        .find(|r| r.id == route_id)
        .ok_or_else(|| anyhow::anyhow!("route not found: {route_id}"))?;

    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    let payload: serde_json::Value = serde_json::from_str(&buf).context("parsing stdin as JSON")?;

    let transform = Transform::compile(&route.transform)?;
    let out = transform.apply(payload)?;
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

async fn replay(config_path: &std::path::Path, job_id: &str) -> anyhow::Result<()> {
    let cfg = config::load(config_path)?;
    let pool = open_pool(&cfg).await?;
    let now = chrono::Utc::now().to_rfc3339();
    let res = sqlx::query(
        "UPDATE jobs SET status = 'pending', visible_at = ?, attempts = 0, updated_at = ? WHERE id = ? AND status = 'dead'",
    )
    .bind(&now)
    .bind(&now)
    .bind(job_id)
    .execute(&pool)
    .await?;
    if res.rows_affected() == 0 {
        anyhow::bail!("no dead job with id {job_id}");
    }
    println!("re-queued {job_id}");
    Ok(())
}

async fn open_pool(cfg: &LoadedConfig) -> anyhow::Result<sqlx::sqlite::SqlitePool> {
    let url = &cfg.server.database_url;
    // sqlx parses sqlite:// URLs; we enable create-if-missing via the URL `mode=rwc`.
    let opts: SqliteConnectOptions = url.parse()
        .with_context(|| format!("parsing database_url {url}"))?;
    let opts = opts.disable_statement_logging();
    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(opts)
        .await
        .with_context(|| format!("opening {url}"))?;
    Ok(pool)
}

async fn run_migrations(pool: &sqlx::sqlite::SqlitePool) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}

// Keeps the HashMap import warning-free in main even though we use it via worker.
#[allow(dead_code)]
fn _typecheck() {
    let _: HashMap<String, Transform>;
}
