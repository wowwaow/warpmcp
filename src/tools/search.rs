use crate::schemas::*;
use crate::utils::RedisManager;
use anyhow::Result;
use redis::{AsyncCommands, RedisResult};
use serde_json::{json, Value};
use std::collections::HashMap;

const DEFAULT_LANGUAGE: &str = "english";
const DEFAULT_SCORE_FIELD: &str = "_score";
const KNOWLEDGE_PREFIX: &str = "knowledge:";

pub struct SearchIndex {
    name: String,
    prefix: String,
    language: String,
    score_field: String,
}

impl SearchIndex {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            prefix: KNOWLEDGE_PREFIX.to_string(),
            language: DEFAULT_LANGUAGE.to_string(),
            score_field: DEFAULT_SCORE_FIELD.to_string(),
        }
    }

    pub async fn create(&self, redis: &RedisManager) -> Result<()> {
        let mut conn = redis.get_connection().await?;
        
        // Drop existing index if it exists
        let _: RedisResult<()> = redis::cmd("FT.DROPINDEX")
            .arg(&self.name)
            .query_async(&mut conn)
            .await;
        
        // Create search index
        let _: () = redis::cmd("FT.CREATE")
            .arg(&self.name)
            .arg("ON").arg("JSON")
            .arg("PREFIX").arg(1).arg(&self.prefix)
            .arg("LANGUAGE").arg(&self.language)
            .arg("SCORE").arg(&self.score_field)
            .arg("SCHEMA")
            // Text fields with weights
            .arg("$.content").arg("AS").arg("content")
            .arg("TEXT").arg("WEIGHT").arg(2.0)
            .arg("$.key").arg("AS").arg("key")
            .arg("TEXT").arg("WEIGHT").arg(1.5)
            .arg("$.tags").arg("AS").arg("tags")
            .arg("TEXT").arg("WEIGHT").arg(1.0)
            // Tag fields for filtering
            .arg("$.category").arg("AS").arg("category")
            .arg("TAG")
            .arg("$.agent_id").arg("AS").arg("agent_id")
            .arg("TAG")
            // Numeric fields for sorting
            .arg("$.created_at").arg("AS").arg("created_at")
            .arg("NUMERIC")
            .arg("$.access_count").arg("AS").arg("access_count")
            .arg("NUMERIC")
            .query_async(&mut conn)
            .await?;
            
        Ok(())
    }
    
    pub async fn search(&self, redis: &RedisManager, params: &SearchKnowledgeArgs) -> Result<Value> {
        let mut conn = redis.get_connection().await?;
        
        // Build query string
        let mut query = format!("@content:'{}'", params.query);
        
        // Apply filters
        if let Some(ref category) = params.category_filter {
            query.push_str(&format!(" @category:{{{}}}", category));
        }
        if let Some(ref agent) = params.agent_filter {
            query.push_str(&format!(" @agent_id:{{{}}}", agent));
        }
        
        // Execute search
        let results: (usize, Vec<String>, HashMap<String, Value>) = redis::cmd("FT.SEARCH")
            .arg(&self.name)
            .arg(query)
            .arg("LIMIT").arg(0).arg(params.limit.unwrap_or(10))
            .arg("SORTBY").arg("created_at").arg("DESC")
            .arg("RETURN").arg("ALL")
            .query_async(&mut conn)
            .await?;
            
        let mut entries = Vec::new();
        
        for doc in results.2.values() {
            if let Some(entry) = doc.as_object() {
                entries.push(json!({
                    "id": entry.get("id").unwrap_or(&json!(null)),
                    "agent_id": entry.get("agent_id").unwrap_or(&json!(null)),
                    "category": entry.get("category").unwrap_or(&json!(null)),
                    "key": entry.get("key").unwrap_or(&json!(null)),
                    "content": entry.get("content").unwrap_or(&json!(null)),
                    "tags": entry.get("tags").unwrap_or(&json!(null)),
                    "created_at": entry.get("created_at").unwrap_or(&json!(null)),
                    "access_count": entry.get("access_count").unwrap_or(&json!(0))
                }));
            }
        }
        
        Ok(json!({
            "query": params.query,
            "results": entries,
            "count": entries.len()
        }))
    }
}
