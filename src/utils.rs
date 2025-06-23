use anyhow::Result;
use redis::{Client, aio::Connection};
use std::env;

#[derive(Clone)]
pub struct RedisManager {
    client: Client,
}

impl RedisManager {
    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = Client::open(redis_url)?;
        
        // Test connection
        let mut conn = client.get_async_connection().await?;
        let _: String = redis::cmd("PING").query_async(&mut conn).await?;
        
        Ok(Self { client })
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
    let token = env::var("TRELLO_TOKEN").expect("TRELLO_TOKEN must be set");
    let board_id = env::var("TRELLO_BOARD_ID").expect("TRELLO_BOARD_ID must be set");
    
    (key, token, board_id)
}