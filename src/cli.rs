use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "sqlhook", version, about = "Webhook gateway with signed ingress, jq transforms, and durable retry.")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Start the HTTP server and worker loop.
    Serve {
        #[arg(long, default_value = "config.yaml")]
        config: PathBuf,
    },
    /// Load and validate a config file without starting the server.
    ValidateConfig {
        #[arg(long, default_value = "config.yaml")]
        config: PathBuf,
    },
    /// Run a route's transform against a JSON payload on stdin and print the result.
    TransformTest {
        #[arg(long, default_value = "config.yaml")]
        config: PathBuf,
        #[arg(long)]
        route: String,
    },
    /// Re-queue a dead-lettered job by id.
    Replay {
        #[arg(long, default_value = "config.yaml")]
        config: PathBuf,
        job_id: String,
    },
}
