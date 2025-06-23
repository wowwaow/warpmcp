// src/main.rs – fixed
use anyhow::Result;
use env_logger;
use log::{error, info};
use std::env;

// Top‑level crates / modules that really exist in this project.
// All feature‑specific sub‑modules (database, heartbeat, memory, tasks, trello, …)
// live under the `tools` crate, so we don’t declare them here to avoid E0583.
mod server;
mod schemas;
mod tools;
mod utils;

use server::MCPServer;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialise logging (writes to stderr by default so Warp can capture it)
    env_logger::Builder::from_default_env()
        .target(env_logger::Target::Stderr)
        .init();

    info!("Starting Warp MCP Tasks Server v1.0.0");

    // Make sure all the mandatory environment variables are present and fall back
    // to sensible defaults for optional ones.
    validate_environment()?;    

    // Spin‑up the MCP server and block until it terminates.
    let redis_url = env::var("REDIS_URL").unwrap();
    let redis_manager = utils::RedisManager::new(&redis_url).await?;    
    let server = MCPServer::new(redis_manager).await?;
    info!("Redis URL: {}", redis_url);
    info!("Starting server...");
    info!("Server address: {}", server.address());
    info!("Server port: {}", server.port());
    server.run().await; 
    Ok(())      
    // Note: The server.run() method is assumed to be an async method that runs
    // the server indefinitely. If it returns, the server has stopped.
}

/// Ensures the process has all the variables it needs to operate.
///
/// * `TRELLO_KEY`, `TRELLO_TOKEN`  and `TRELLO_BOARD_ID` are required – we bail
///   out early if any of them are missing.
/// * `REDIS_URL` and `HEARTBEAT_TIMEOUT` are optional and get sane defaults if
///   they are absent.
fn validate_environment() -> Result<()> {
    const REQUIRED_VARS: [&str; 3] = ["TRELLO_KEY", "TRELLO_TOKEN", "TRELLO_BOARD_ID"];

    for var in REQUIRED_VARS {        
        if env::var(var).is_err() {
            error!("Missing required environment variable: {var}");
            return Err(anyhow::anyhow!("Missing environment variable: {var}"));
        }
    }

    // Provide fall‑backs for optional env‑vars so the rest of the code can just
    // unwrap() them safely.
    env::set_var(
        "REDIS_URL",
        env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_owned()),
    );

    env::set_var(
        "HEARTBEAT_TIMEOUT",
        env::var("HEARTBEAT_TIMEOUT").unwrap_or_else(|_| "120".to_owned()),
    );

    Ok(())
}
