use crate::utils::RedisManager;
use anyhow::Result;
use redis::{AsyncCommands, JsonAsyncCommands};
use serde_json::{json, Value};

pub async fn execute_rag_query(
    redis: &RedisManager,
    args: Value,
) -> Result<String> {
    let mut conn = redis.get_connection().await?;
    
    let query = args.get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Query required"))?;
    
    // Use RedisJSON path queries for complex RAG operations
    let json_path = args.get("json_path")
        .and_then(|v| v.as_str())
        .unwrap_or("$");
    
    // Example: Find all knowledge entries matching criteria
    let pattern = "knowledge:*";
    let keys: Vec<String> = conn.keys(pattern).await?;
    
    let mut results = Vec::new();
    
    for key in keys {
        // Use JSON path queries
        let matches: Option<String> = conn.json_get(&key, json_path).await?;
        if let Some(json_str) = matches {
            if json_str.contains(query) {
                results.push(json!({
                    "key": key,
                    "match": json_str
                }));
            }
        }
    }
    
    Ok(json!({
        "query": query,
        "path": json_path,
        "results": results,
        "count": results.len()
    }).to_string())
}