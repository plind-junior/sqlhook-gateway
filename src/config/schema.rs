use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub server: ServerSpec,
    #[serde(default)]
    pub worker: WorkerSpec,
    pub routes: Vec<Route>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerSpec {
    pub bind: String,
    pub database_url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WorkerSpec {
    #[serde(default = "default_poll_interval")]
    pub poll_interval_ms: u64,
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
}

impl Default for WorkerSpec {
    fn default() -> Self {
        Self {
            poll_interval_ms: default_poll_interval(),
            concurrency: default_concurrency(),
        }
    }
}

fn default_poll_interval() -> u64 { 500 }
fn default_concurrency() -> usize { 4 }

#[derive(Debug, Deserialize)]
pub struct Route {
    pub id: String,
    pub source_path: String,
    pub signature: SignatureSpec,
    pub transform: String,
    pub destination: DestinationSpec,
    pub retry: RetrySpec,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SignatureSpec {
    pub header: String,
    pub algo: SignatureAlgo,
    pub secret_env: String,
    #[serde(default)]
    pub prefix: String,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SignatureAlgo {
    HmacSha256,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DestinationSpec {
    pub url: String,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_timeout() -> u64 { 5000 }

#[derive(Debug, Deserialize, Clone)]
pub struct RetrySpec {
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    #[serde(default = "default_initial_backoff")]
    pub initial_backoff_ms: u64,
    #[serde(default = "default_max_backoff")]
    pub max_backoff_ms: u64,
}

fn default_max_attempts() -> u32 { 5 }
fn default_initial_backoff() -> u64 { 500 }
fn default_max_backoff() -> u64 { 60_000 }
