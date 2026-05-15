use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("config error: {0}")]
    Config(String),

    #[error("route not found: {0}")]
    RouteNotFound(String),

    #[error("signature verification failed")]
    InvalidSignature,

    #[error("missing signature header: {0}")]
    MissingSignature(String),

    #[error("invalid payload: {0}")]
    InvalidPayload(String),

    #[error("transform error: {0}")]
    Transform(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
}

pub type AppResult<T> = Result<T, AppError>;
