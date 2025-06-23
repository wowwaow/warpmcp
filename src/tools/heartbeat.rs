use crate::schemas::*;
use crate::utils::{RedisManager, get_heartbeat_timeout};
use anyhow::Result;
use redis::{AsyncCommands, RedisResult};
use serde::{Deserialize, Serialize};
use serde_json::json;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use serde_json::{json, Value};
use std::time::Duration;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::{Write, Read};
use std::collections::HashMap;

const HEARTBEAT_JITTER: Duration = Duration::from_secs(5);
const HEARTBEAT_BUFFER_SIZE: usize = 100;
const MAX_RETRY_COUNT: u32 = 3;
const BASE_RETRY_DELAY_MS: u64 = 100;
const SLIDING_WINDOW_SIZE: i64 = 300; // 5 minutes

fn compress_status(status: &AgentStatus) -> Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&serde_json::to_vec(status)?)?;
    Ok(encoder.finish()?)
}

async fn send_heartbeat_with_retry(
    conn: &mut redis::aio::Connection,
    hash_key: &str,
    field: &str,
    value: &[u8],
    timeout: i64,
    retry_count: u32,
) -> RedisResult<()> {
    let mut current_retry = 0;
    loop {
        match redis::cmd("HSET").arg(hash_key).arg(field).arg(value).query_async::<_, ()>(conn).await {
            Ok(_) => {
                let _: () = redis::cmd("EXPIRE")
                    .arg(hash_key)
                    .arg(timeout)
                    .query_async(conn).await?;
                return Ok(());
            },
            Err(e) if current_retry < retry_count => {
                let delay = BASE_RETRY_DELAY_MS * (2_u64.pow(current_retry));
                tokio::time::sleep(Duration::from_millis(delay)).await;
                current_retry += 1;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}

pub async fn send_heartbeat(
    redis: &RedisManager,
    args: Value,
) -> Result<String> {
    let params: HeartbeatArgs = serde_json::from_value(args)?;
    let mut conn = redis.get_connection().await?;

    let timestamp = chrono::Utc::now().timestamp();
    let jitter = fastrand::u64(..=HEARTBEAT_JITTER.as_secs()) as i64;

    let status = AgentStatus {
        agent_id: params.agent_id.clone(),
        card_id: params.card_id.clone(),
        card_name: "".to_string(),
        status: params.status,
        last_heartbeat: timestamp,
        progress: params.progress_percentage.unwrap_or(0.0),
    };

    let compressed_status = compress_status(&status)?;

    let timeout = get_heartbeat_timeout() as i64 + jitter;
    let hash_key = "agent_heartbeats";
    let field = format!("{}{}", params.agent_id, params.card_id);

    send_heartbeat_with_retry(&mut conn, hash_key, &field, &compressed_status, timeout, MAX_RETRY_COUNT).await?;

    let window_key = format!("agent_window:{}", params.agent_id);
    let _: () = conn.zadd(&window_key, timestamp.to_string(), timestamp).await?;
    let _: () = redis::cmd("ZREMRANGEBYSCORE").arg(&window_key).arg("-inf").arg((timestamp - SLIDING_WINDOW_SIZE).to_string()).query_async(&mut conn).await?;
    let _: () = redis::cmd("EXPIRE").arg(&window_key).arg(timeout).query_async(&mut conn).await?;

    Ok(format!("Heartbeat recorded for agent {} on task {}", params.agent_id, params.card_id))
}

pub async fn check_agent_status(redis: &RedisManager) -> Result<String> {
    let mut conn = redis.get_connection().await?;
    let hash_key = "agent_heartbeats";

    let all_statuses: HashMap<String, Vec<u8>> = conn.hgetall(hash_key).await?;
    let mut active_agents = Vec::with_capacity(HEARTBEAT_BUFFER_SIZE);

    let current_time = chrono::Utc::now().timestamp();
    let timeout = get_heartbeat_timeout() as i64;

    for (field, compressed_data) in all_statuses {
        let mut decoder = flate2::read::ZlibDecoder::new(&compressed_data[..]);
        let mut decompressed = Vec::new();
        if decoder.read_to_end(&mut decompressed).is_ok() {
            if let Ok(status) = serde_json::from_slice::<AgentStatus>(&decompressed) {
                let window_key = format!("agent_window:{}", status.agent_id);
                let heartbeats: Vec<i64> = redis::cmd("ZRANGEBYSCORE")
                    .arg(&window_key)
                    .arg((current_time - SLIDING_WINDOW_SIZE).to_string())
                    .arg(current_time.to_string())
                    .query_async(&mut conn).await.unwrap_or_default();

                if !heartbeats.is_empty() {
                    active_agents.push(json!({
                        "agent_id": status.agent_id,
                        "card_id": status.card_id,
                        "status": status.status,
                        "progress": status.progress,
                        "last_seen": status.last_heartbeat,
                        "heartbeat_count": heartbeats.len()
                    }));
                } else {
                    let _: () = conn.hdel(hash_key, field).await?;
                }
            }
        }
    }

    Ok(json!({
        "active_agents": active_agents,
        "total_active": active_agents.len(),
        "timestamp": current_time
    }).to_string())
}
