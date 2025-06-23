use crate::schemas::*;
use crate::utils::RedisManager;
use anyhow::Result;
use redis::{AsyncCommands, RedisResult, FromRedisValue};
use serde_json::{json, Value};
use std::collections::HashMap;

// Constants for search configuration
const DEFAULT_LANGUAGE: &str = "english";
const DEFAULT_SCORE_FIELD: &str = "_score";
const KNOWLEDGE_PREFIX: &str = "knowledge:";
const DEFAULT_FUZZY_DISTANCE: u32 = 2;
const MAX_EXPANSIONS: u32 = 50;
const MIN_SCORE: f64 = 0.1;

// Search index types
#[derive(Debug, Clone)]
pub enum IndexType {
    Text,
    Tag,
    Numeric,
    Vector,
    Geo
}

// Field definition for search index
#[derive(Debug, Clone)]
pub struct IndexField {
    name: String,
    field_type: IndexType,
    weight: Option<f64>,
    sortable: bool,
    fuzzy: bool,
    phonetic: bool,
}

impl IndexField {
    pub fn new(name: &str, field_type: IndexType) -> Self {
        Self {
            name: name.to_string(),
            field_type,
            weight: None,
            sortable: false,
            fuzzy: false,
            phonetic: false,
        }
    }
    
    pub fn weight(mut self, weight: f64) -> Self {
        self.weight = Some(weight);
        self
    }
    
    pub fn sortable(mut self) -> Self {
        self.sortable = true;
        self
    }
    
    pub fn fuzzy(mut self) -> Self {
        self.fuzzy = true;
        self
    }
    
    pub fn phonetic(mut self) -> Self {
        self.phonetic = true;
        self
    }
}

// Extended search parameters
pub struct SearchParams {
    pub query: String,
    pub filters: Vec<(String, String)>,
    pub numeric_filters: Vec<(String, f64, f64)>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub sort_by: Option<String>,
    pub sort_asc: bool,
    pub min_score: Option<f64>,
    pub return_fields: Option<Vec<String>>,
    pub summarize: bool,
    pub highlight: bool,
    pub fuzzy_distance: Option<u32>,
}

impl Default for SearchParams {
    fn default() -> Self {
        Self {
            query: String::new(),
            filters: Vec::new(),
            numeric_filters: Vec::new(),
            limit: Some(10),
            offset: Some(0),
            sort_by: None,
            sort_asc: true,
            min_score: Some(MIN_SCORE),
            return_fields: None,
            summarize: false,
            highlight: false,
            fuzzy_distance: Some(DEFAULT_FUZZY_DISTANCE),
        }
    }
}

pub struct SearchIndex {
    name: String,
    prefix: String,
    language: String,
    score_field: String,
}

// Search query builder
pub struct QueryBuilder {
    parts: Vec<String>,
}

impl QueryBuilder {
    pub fn new() -> Self {
        Self { parts: Vec::new() }
    }
    
    pub fn text_match(mut self, field: &str, value: &str, fuzzy: bool) -> Self {
        let query = if fuzzy {
            format!("@{}:%{}%", field, value)
        } else {
            format!("@{}:{}", field, value)
        };
        self.parts.push(query);
        self
    }
    
    pub fn tag_filter(mut self, field: &str, value: &str) -> Self {
        self.parts.push(format!("@{}:{{{}}}", field, value));
        self
    }
    
    pub fn numeric_range(mut self, field: &str, min: f64, max: f64) -> Self {
        self.parts.push(format!("@{}:[{} {}]", field, min, max));
        self
    }
    
    pub fn build(self) -> String {
        if self.parts.is_empty() {
            "*".to_string()
        } else {
            self.parts.join(" ") 
        }
    }
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

    // Helper to build a field definition
    fn field_def(&self, field: &IndexField) -> Vec<String> {
        let mut args = vec![];
        
        // Field path and alias
        args.push(format!("$.{}", field.name));
        args.push("AS".to_string());
        args.push(field.name.clone());
        
        // Field type and options
        match field.field_type {
            IndexType::Text => {
                args.push("TEXT".to_string());
                if let Some(weight) = field.weight {
                    args.push("WEIGHT".to_string());
                    args.push(weight.to_string());
                }
                if field.fuzzy {
                    args.push("WITHSUFFIXTRIE".to_string());
                }
                if field.phonetic {
                    args.push("PHONETIC".to_string());
                    args.push("dm:en".to_string()); // Double Metaphone
                }
            },
            IndexType::Tag => {
                args.push("TAG".to_string());
                if field.sortable {
                    args.push("SORTABLE".to_string());
                }
            },
            IndexType::Numeric => {
                args.push("NUMERIC".to_string());
                if field.sortable {
                    args.push("SORTABLE".to_string());
                }
            },
            IndexType::Vector => {
                args.push("VECTOR".to_string());
                args.push("HNSW".to_string());
                args.push("6".to_string()); // Dimensions
                args.push("TYPE".to_string());
                args.push("FLOAT32".to_string());
                args.push("DIM".to_string());
                args.push("512".to_string()); // Vector size
            },
            IndexType::Geo => {
                args.push("GEO".to_string());
            },
        }
        
        args
    }

    pub async fn create(&self, redis: &RedisManager) -> Result<()> {
        let mut conn = redis.get_connection().await?;
        
        // Drop existing index if it exists
        let _: RedisResult<()> = redis::cmd("FT.DROPINDEX")
            .arg(&self.name)
            .query_async(&mut conn)
            .await;
        
        // Define fields with advanced options
        let fields = vec![
            IndexField::new("content", IndexType::Text)
                .weight(2.0)
                .fuzzy()
                .phonetic(),
            IndexField::new("key", IndexType::Text)
                .weight(1.5)
                .fuzzy(),
            IndexField::new("tags", IndexType::Text)
                .weight(1.0)
                .fuzzy(),
            IndexField::new("category", IndexType::Tag)
                .sortable(),
            IndexField::new("agent_id", IndexType::Tag)
                .sortable(),
            IndexField::new("created_at", IndexType::Numeric)
                .sortable(),
            IndexField::new("access_count", IndexType::Numeric)
                .sortable(),
            IndexField::new("embeddings", IndexType::Vector),
        ];
        
        // Build index creation command
        let mut cmd = redis::cmd("FT.CREATE");
        cmd.arg(&self.name)
           .arg("ON").arg("JSON")
           .arg("PREFIX").arg(1).arg(&self.prefix)
           .arg("LANGUAGE").arg(&self.language)
           .arg("SCORE").arg(&self.score_field)
           .arg("SCHEMA");
           
        // Add field definitions
        for field in fields {
            for arg in self.field_def(&field) {
                cmd.arg(arg);
            }
        }
        
        let _: () = cmd.query_async(&mut conn).await?;
            
        Ok(())
    }
    
    pub async fn search(&self, redis: &RedisManager, params: &SearchKnowledgeArgs) -> Result<Value> {
        // Convert standard args to extended search params
        let return_fields = vec!["$.id".to_string(), "$.agent_id".to_string(), "$.category".to_string(), "$.content".to_string()];
        let search_params = SearchParams {
            query: params.query.clone(),
            filters: vec![],
            numeric_filters: vec![],
            limit: params.limit,
            offset: Some(0),
            sort_by: Some("created_at".to_string()),
            sort_asc: false,
            min_score: Some(MIN_SCORE),
            return_fields: Some(return_fields),
            summarize: true,
            highlight: true,
            fuzzy_distance: Some(DEFAULT_FUZZY_DISTANCE),
        };
        
        self.advanced_search(redis, &search_params).await
    }
    
    pub async fn advanced_search(&self, redis: &RedisManager, params: &SearchParams) -> Result<Value> {
        let mut conn = redis.get_connection().await?;
        
        // Create basic index
        let _: RedisResult<()> = redis::cmd("FT.CREATE")
            .arg("knowledge-idx")
            .arg("ON").arg("JSON")
            .arg("PREFIX").arg(1).arg("knowledge:")
            .arg("SCHEMA")
            .arg("$.content").arg("AS").arg("content").arg("TEXT")
            .query_async(&mut conn)
            .await;
            
        // Basic search with tuple response
        let results: (usize, Vec<String>, HashMap<String, String>) = redis::cmd("FT.SEARCH")
            .arg("knowledge-idx")
            .arg(format!("@content:{}", params.query))
            .query_async(&mut conn)
            .await?;
            
        let mut entries = Vec::new();
        for (_key, value) in results.2 {
            if let Ok(entry) = serde_json::from_str::<Value>(&value) {
                entries.push(entry);
            }
        }
        
        Ok(json!({
            "query": params.query,
            "results": entries,
            "count": entries.len()
        }))
    }
}
