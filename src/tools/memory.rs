use crate::schemas::*;
use crate::utils::RedisManager;
use anyhow::Result;
use redis::{AsyncCommands, JsonAsyncCommands};
use serde_json::{json, Value};
use uuid::Uuid;

const MEMORY_EXPIRATION: i64 = 604800; // 7 days in seconds

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
    
    // Store in RedisJSON for complex queries
    let json_key = format!("knowledge:{}", knowledge_id);
    let _: () = conn.json_set(&json_key, "$", &entry).await?;
    let _: () = conn.expire(&json_key, MEMORY_EXPIRATION).await?;
    
    // Index by multiple dimensions for RAG
    // Category index
    let category_key = format!("idx:category:{}", params.category);
    let _: () = conn.sadd(&category_key, &knowledge_id).await?;
    
    // Agent index
    let agent_key = format!("idx:agent:{}", params.agent_id);
    let _: () = conn.sadd(&agent_key, &knowledge_id).await?;
    
    // Tag indices
    for tag in &params.tags {
        let tag_key = format!("idx:tag:{}", tag);
        let _: () = conn.sadd(&tag_key, &knowledge_id).await?;
    }
    
    // Key-based index for quick lookups
    let lookup_key = format!("lookup:{}:{}", params.agent_id, params.key);
    let _: () = conn.set_ex(&lookup_key, &knowledge_id, MEMORY_EXPIRATION as u64).await?;
    
    Ok(format!("Knowledge stored with ID: {}", knowledge_id))
}

pub async fn search_knowledge(
    redis: &RedisManager,
    args: Value,
) -> Result<String> {
    let params: SearchKnowledgeArgs = serde_json::from_value(args)?;
    let mut conn = redis.get_connection().await?;
    
    // Search across multiple indices
    let mut candidate_ids = Vec::new();
    
    // Search in content using pattern matching
    let pattern = format!("knowledge:*");
    let keys: Vec<String> = conn.keys(&pattern).await?;
    
    for key in keys {
        let entry: Option<String> = conn.json_get(&key, "$").await?;
        if let Some(json_str) = entry {
            if let Ok(entry) = serde_json::from_str::<Vec<KnowledgeEntry>>(&json_str) {
                if let Some(knowledge) = entry.first() {
                    // Simple text search - in production, use proper text search
                    if knowledge.content.to_lowercase().contains(&params.query.to_lowercase()) ||
                       knowledge.tags.iter().any(|t| t.to_lowercase().contains(&params.query.to_lowercase())) {
                        candidate_ids.push(knowledge.id.clone());
                    }
                }
            }
        }
    }
    
    // Apply filters
    let mut results = Vec::new();
    let limit = params.limit.unwrap_or(10);
    
    for id in candidate_ids.iter().take(limit) {
        let key = format!("knowledge:{}", id);
        let entry: Option<String> = conn.json_get(&key, "$").await?;
        
        if let Some(json_str) = entry {
            if let Ok(mut entries) = serde_json::from_str::<Vec<KnowledgeEntry>>(&json_str) {
                if let Some(mut knowledge) = entries.pop() {
                    // Apply filters
                    if let Some(ref category) = params.category_filter {
                        if &knowledge.category != category {
                            continue;
                        }
                    }
                    
                    if let Some(ref agent) = params.agent_filter {
                        if &knowledge.agent_id != agent {
                            continue;
                        }
                    }
                    
                    // Increment access count
                    knowledge.access_count += 1;
                    let _: () = conn.json_set(&key, "$", &vec![&knowledge]).await?;
                    
                    results.push(json!({
                        "id": knowledge.id,
                        "agent_id": knowledge.agent_id,
                        "category": knowledge.category,
                        "key": knowledge.key,
                        "content": knowledge.content,
                        "tags": knowledge.tags,
                        "created_at": knowledge.created_at,
                        "access_count": knowledge.access_count
                    }));
                }
            }
        }
    }
    
    Ok(json!({
        "query": params.query,
        "results": results,
        "count": results.len()
    }).to_string())
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