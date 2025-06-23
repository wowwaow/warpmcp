use anyhow::Result;
use env_logger;
use log::{error, info};
use std::env;

mod server;
mod schemas;
mod tools;
mod utils;

use server::MCPServer;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging to stderr
    env_logger::Builder::from_default_env()
        .target(env_logger::Target::Stderr)
        .init();

    info!("Starting Warp MCP Tasks Server v1.0.0");

    // Validate required environment variables
    validate_environment()?;

    // Create and start the MCP server
    let server = MCPServer::new().await?;
    server.run().await?;

    Ok(())
}

fn validate_environment() -> Result<()> {
    let required_vars = [
        "TRELLO_KEY",
        "TRELLO_TOKEN", 
        "TRELLO_BOARD_ID"
    ];

    for var in &required_vars {
        if env::var(var).is_err() {
            error!("Missing required environment variable: {}", var);
            return Err(anyhow::anyhow!("Missing environment variable: {}", var));
        }
    }

    // Optional vars with defaults
    if env::var("REDIS_URL").is_err() {
        info!("REDIS_URL not set, using default: redis://127.0.0.1:6379");
        env::set_var("REDIS_URL", "redis://127.0.0.1:6379");
    }

    if env::var("HEARTBEAT_TIMEOUT").is_err() {
        info!("HEARTBEAT_TIMEOUT not set, using default: 120 seconds");
        env::set_var("HEARTBEAT_TIMEOUT", "120");
    }

    Ok(())
}
