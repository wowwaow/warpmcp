use anyhow::{Result, anyhow};
use redis::{Client, aio::Connection, AsyncCommands, RedisResult};
use std::{env, fs};
use serde_json::Value;

const MAX_RETRIES: u32 = 3;
const RETRY_DELAY: u64 = 1000; // milliseconds

#[derive(Clone)]
pub struct RedisManager {
    client: Client,
    initialized: bool,
}

impl RedisManager {
    async fn ensure_modules_loaded(&self) -> Result<()> {
        let mut conn = self.get_connection().await?;
        
        // Check for required modules
        let modules: Vec<(String, String, u32)> = redis::cmd("MODULE")
            .arg("LIST")
            .query_async(&mut conn)
            .await?;
            
        let has_search = modules.iter().any(|m| m.1 == "search");
        let has_json = modules.iter().any(|m| m.1 == "ReJSON");
        
        if !has_search || !has_json {
            return Err(anyhow!("Required Redis modules not loaded. Please install: redisearch, rejson"));
        }
        
        Ok(())
    }
    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = Client::open(redis_url)?;
        
        // Test connection
        let mut conn = client.get_async_connection().await?;
        let _: String = redis::cmd("PING").query_async(&mut conn).await?;
        
        let mut manager = Self { 
            client,
            initialized: false
        };
        
        // Initialize search
        manager.init_search().await?;
        
        Ok(manager)
    }
    
    pub async fn init_search(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }
        
        let mut conn = self.get_connection().await?;
        let mut retries = 0;
        
        while retries < MAX_RETRIES {
            // Load and execute initialization script
            let script = fs::read_to_string("src/scripts/init_redis.lua")?;
            let result: RedisResult<Value> = redis::cmd("EVAL")
                .arg(&script)
                .arg(0) // No script keys
                .query_async(&mut conn)
                .await;
                
            match result {
                Ok(_) => {
                    self.initialized = true;
                    return Ok(());
                }
                Err(e) => {
                    if retries == MAX_RETRIES - 1 {
                        return Err(anyhow!("Failed to initialize search: {}", e));
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(RETRY_DELAY)).await;
                    retries += 1;
                }
            }
        }
        
        Err(anyhow!("Failed to initialize search after {} retries", MAX_RETRIES))
    }

    pub async fn get_connection(&self) -> Result<Connection> {
        Ok(self.client.get_async_connection().await?)
    }
}

pub fn get_trello_list_ids() -> (String, String, String) {
    let todo = env::var("TRELLO_TODO_LIST_ID")
        .unwrap_or_else(|_| "684c6555b0095319f40a07d9".to_string());
    let in_progress = env::var("TRELLO_IN_PROGRESS_LIST_ID")
        .unwrap_or_else(|_| "684c6555443ad8d1a8248b73".to_string());
    let done = env::var("TRELLO_DONE_LIST_ID")
        .unwrap_or_else(|_| "684c65561cd33835b591cac6".to_string());
    
    (todo, in_progress, done)
}

pub fn get_heartbeat_timeout() -> u64 {
    env::var("HEARTBEAT_TIMEOUT")
        .unwrap_or_else(|_| "120".to_string())
        .parse()
        .unwrap_or(120)
}

pub fn get_trello_config() -> (String, String, String) {
    let key = env::var("TRELLO_KEY").expect("TRELLO_KEY must be set");
    let token = env::var("TRELLO_TOKEN").unwrap_or_else(|_| {
        env::var("TRELLO_API_TOKEN").expect("TRELLO_TOKEN or TRELLO_API_TOKEN must be set")
    });
    let board_id = env::var("TRELLO_BOARD_ID").expect("TRELLO_BOARD_ID must be set");
    
    (key, token, board_id)
}