use crate::schemas::*;
use crate::utils::RedisManager;
use anyhow::Result;
use redis::{AsyncCommands, JsonAsyncCommands};
use serde_json::{json, Value};
use uuid::Uuid;

use super::search::{SearchIndex, SearchParams};

const MEMORY_EXPIRATION: i64 = 604800; // 7 days in seconds

async fn ensure_index(redis: &RedisManager) -> Result<()> {
    let index = SearchIndex::new("knowledge-idx");
    index.create(redis).await
}

pub async fn store_knowledge(
    redis: &RedisManager,
    args: Value,
) -> Result<String> {
    let params: StoreKnowledgeArgs = serde_json::from_value(args)?;
    let mut conn = redis.get_connection().await?;
    
    let knowledge_id = Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now().timestamp();
    
    let entry = KnowledgeEntry {
        id: knowledge_id.clone(),
        agent_id: params.agent_id.clone(),
        category: params.category.clone(),
        key: params.key.clone(),
        content: params.content,
        tags: params.tags.clone(),
        metadata: params.metadata.unwrap_or(json!({})),
        created_at: timestamp,
        updated_at: timestamp,
        access_count: 0,
    };
    
    // Ensure search index exists
    if let Err(e) = ensure_index(redis).await {
        eprintln!("Warning: Failed to create search index: {}", e);
    }

    // Store in RedisJSON for complex queries
    let json_key = format!("knowledge:{}", knowledge_id);
    let _: () = conn.json_set(&json_key, "$", &entry).await?;
    let _: () = conn.expire(&json_key, MEMORY_EXPIRATION).await?;
    
    Ok(format!("Knowledge stored with ID: {}", knowledge_id))
}

pub async fn search_knowledge(
    redis: &RedisManager,
    args: Value,
) -> Result<String> {
    let params: SearchKnowledgeArgs = serde_json::from_value(args)?;
    
    // Ensure search index exists
    if let Err(e) = ensure_index(redis).await {
        eprintln!("Warning: Failed to create search index: {}", e);
    }

    // Use search index
    let index = SearchIndex::new("knowledge-idx");
    let results = index.search(redis, &params).await?;
    
    // Update access counts for returned entries
    let mut conn = redis.get_connection().await?;
    if let Some(entries) = results.get("results").and_then(|v| v.as_array()) {
        for entry in entries {
            if let Some(id) = entry.get("id").and_then(|v| v.as_str()) {
                let key = format!("knowledge:{}", id);
                let _: () = redis::cmd("JSON.NUMINCRBY")
                    .arg(&key)
                    .arg("$.access_count")
                    .arg(1)
                    .query_async(&mut conn)
                    .await?;
            }
        }
    }
    
    Ok(results.to_string())
}

pub async fn learn_from_agents(
    redis: &RedisManager,
    args: Value,
) -> Result<String> {
    let topic = args.get("topic")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Topic required"))?;
    
    let _time_range = args.get("time_range")
        .and_then(|v| v.as_str())
        .unwrap_or("all");
    
    // Search for knowledge entries related to the topic
    let search_args = json!({
        "query": topic,
        "limit": 20
    });
    
    let search_results = search_knowledge(redis, search_args).await?;
    let results: Value = serde_json::from_str(&search_results)?;
    
    // Group by agent and extract learnings
    let mut learnings = json!({
        "topic": topic,
        "agent_learnings": {},
        "common_patterns": [],
        "error_solutions": []
    });
    
    if let Some(entries) = results.get("results").and_then(|v| v.as_array()) {
        for entry in entries {
            if let Some(category) = entry.get("category").and_then(|v| v.as_str()) {
                if category == "errors" || category == "solutions" {
                    learnings["error_solutions"].as_array_mut().unwrap().push(entry.clone());
                }
            }
        }
    }
    
    Ok(learnings.to_string())
}