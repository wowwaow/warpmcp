use anyhow::Result;
use deadpool_redis::{Pool, Runtime, Config, PoolConfig, Connection, Timeouts};
use redis::{Pipeline, RedisError, AsyncCommands};
use std::{env, time::Duration};
use metrics::{counter, gauge};
use tokio::time::interval;

#[derive(Clone)]
pub struct RedisManager {
    pool: Pool,
}

impl RedisManager {
    pub async fn new() -> Result<Self> {
        let redis_url = env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

        let mut cfg = Config::from_url(redis_url);
        cfg.pool = Some(PoolConfig {
            max_size: 32,
            timeouts: Timeouts {
                wait: Some(Duration::from_secs(2)),
                create: Some(Duration::from_secs(2)),
                recycle: Some(Duration::from_secs(5)),
            },
            queue_mode: None, // Add this if required by your version
        });

        let pool = cfg.create_pool(Some(Runtime::Tokio1))?;
        let pool_clone = pool.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                if let Err(e) = Self::health_check(&pool_clone).await {
                    eprintln!("Redis health check failed: {}", e);
                    counter!("redis_health_check_failures", "status" => "failed");
                }
                Self::update_metrics(&pool_clone);
            }
        });

        Ok(Self { pool })
    }

    pub async fn get_connection(&self) -> Result<Connection> {
        let mut retries = 3;
        let mut backoff = Duration::from_millis(100);

        loop {
            match self.pool.get().await {
                Ok(mut conn) => {
                    match redis::cmd("PING").query_async::<_, String>(&mut conn).await {
                        Ok(_) => {
                            counter!("redis_connection_success", "result" => "ok");
                            return Ok(conn);
                        }
                        Err(_) => {
                            counter!("redis_connection_failure", "result" => "ping_failed");
                        }
                    }
                }
                Err(e) => {
                    counter!("redis_connection_failure", "result" => "get_failed");
                    if retries == 0 {
                        return Err(anyhow::anyhow!("Failed to get Redis connection after retries: {}", e));
                    }
                }
            }

            retries -= 1;
            tokio::time::sleep(backoff).await;
            backoff *= 2;
        }
    }

    pub fn create_pipeline(&self) -> Pipeline {
        Pipeline::new()
    }

    pub async fn execute_pipeline(&self, pipeline: Pipeline) -> Result<Vec<redis::Value>> {
        let mut conn = self.get_connection().await?;
        let timeout = Duration::from_secs(5);

        tokio::time::timeout(timeout, pipeline.query_async(&mut conn))
            .await?
            .map_err(Into::into)
    }

    async fn health_check(pool: &Pool) -> Result<()> {
        let mut conn = pool.get().await?;
        let response: String = redis::cmd("PING").query_async(&mut conn).await?;
        if response != "PONG" {
            return Err(anyhow::anyhow!("Invalid PING response"));
        }
        Ok(())
    }

    fn update_metrics(pool: &Pool) {
        let status = pool.status();
        gauge!("redis_pool_available", "pool" => "default").set(status.available as f64);
        gauge!("redis_pool_size", "pool" => "default").set(status.size as f64);
    }
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
