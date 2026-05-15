pub mod schema;

use std::collections::HashMap;
use std::path::Path;

use crate::error::{AppError, AppResult};

pub use schema::{Config, Route, SignatureSpec, DestinationSpec, RetrySpec, ServerSpec, WorkerSpec};

/// Loaded configuration with secrets resolved from the environment.
pub struct LoadedConfig {
    pub server: ServerSpec,
    pub worker: WorkerSpec,
    pub routes_by_path: HashMap<String, ResolvedRoute>,
}

/// A route with its HMAC secret already resolved from env vars.
pub struct ResolvedRoute {
    pub id: String,
    pub source_path: String,
    pub signature: SignatureSpec,
    pub secret: Vec<u8>,
    pub transform: String,
    pub destination: DestinationSpec,
    pub retry: RetrySpec,
}

pub fn load<P: AsRef<Path>>(path: P) -> AppResult<LoadedConfig> {
    let raw = std::fs::read_to_string(path.as_ref()).map_err(AppError::Io)?;
    let parsed: Config = serde_yaml::from_str(&raw).map_err(AppError::Yaml)?;
    resolve(parsed)
}

pub fn validate<P: AsRef<Path>>(path: P) -> AppResult<()> {
    load(path).map(|_| ())
}

fn resolve(config: Config) -> AppResult<LoadedConfig> {
    if config.routes.is_empty() {
        return Err(AppError::Config("no routes defined".into()));
    }

    let mut routes_by_path = HashMap::new();
    for route in config.routes {
        if !route.source_path.starts_with('/') {
            return Err(AppError::Config(format!(
                "route {}: source_path must start with /",
                route.id
            )));
        }

        let secret = std::env::var(&route.signature.secret_env).map_err(|_| {
            AppError::Config(format!(
                "route {}: env var {} is not set",
                route.id, route.signature.secret_env
            ))
        })?;

        let resolved = ResolvedRoute {
            id: route.id.clone(),
            source_path: route.source_path.clone(),
            signature: route.signature,
            secret: secret.into_bytes(),
            transform: route.transform,
            destination: route.destination,
            retry: route.retry,
        };

        if routes_by_path.insert(route.source_path, resolved).is_some() {
            return Err(AppError::Config(format!(
                "duplicate source_path for route {}",
                route.id
            )));
        }
    }

    Ok(LoadedConfig {
        server: config.server,
        worker: config.worker,
        routes_by_path,
    })
}
