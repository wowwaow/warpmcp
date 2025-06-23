use crate::schemas::*;
use crate::utils::{RedisManager, get_heartbeat_timeout};
use anyhow::Result;
use redis::AsyncCommands;
use serde_json::{json, Value};

pub async fn send_heartbeat(
    redis: &RedisManager,
    args: Value,
) -> Result<String> {
    let params: HeartbeatArgs = serde_json::from_value(args)?;
    let mut conn = redis.get_connection().await?;
    
    let heartbeat_key = format!("heartbeat:{}:{}", params.agent_id, params.card_id);
    let timestamp = chrono::Utc::now().timestamp();
    
    let status = AgentStatus {
        agent_id: params.agent_id.clone(),
        card_id: params.card_id.clone(),
        card_name: "".to_string(), // Would fetch from Trello
        status: params.status,
        last_heartbeat: timestamp,
        progress: params.progress_percentage.unwrap_or(0.0),
    };
    
    // Store heartbeat with expiration
    let timeout = get_heartbeat_timeout();
    let _: () = conn.set_ex(&heartbeat_key, serde_json::to_string(&status)?, timeout).await?;
    
    // Update agent's active status
    let active_key = format!("active_agents");
    let _: () = conn.zadd(&active_key, &params.agent_id, timestamp as f64).await?;
    
    Ok(format!("Heartbeat recorded for agent {} on task {}", params.agent_id, params.card_id))
}

pub async fn check_agent_status(redis: &RedisManager) -> Result<String> {
    let mut conn = redis.get_connection().await?;
    let pattern = "heartbeat:*";
    let keys: Vec<String> = conn.keys(pattern).await?;
    
    let mut active_agents = Vec::new();
    
    for key in keys {
        let status_str: Option<String> = conn.get(&key).await?;
        if let Some(status_json) = status_str {
            if let Ok(status) = serde_json::from_str::<AgentStatus>(&status_json) {
                active_agents.push(json!({
                    "agent_id": status.agent_id,
                    "card_id": status.card_id,
                    "status": status.status,
                    "progress": status.progress,
                    "last_seen": status.last_heartbeat
                }));
            }
        }
    }
    
    // Clean up stale agents
    let active_key = "active_agents";
    let cutoff = chrono::Utc::now().timestamp() - get_heartbeat_timeout() as i64;
    let _: () = conn.zrembyscore(&active_key, "-inf", cutoff as f64).await?;
    
    Ok(json!({
        "active_agents": active_agents,
        "total_active": active_agents.len()
    }).to_string())
}