#[cfg(test)]
mod tests {
    use super::super::search::{SearchIndex, SearchParams, IndexType};
    use crate::utils::RedisManager;
    use crate::schemas::StoreKnowledgeArgs;
    use serde_json::json;
    use std::time::SystemTime;
    
    #[tokio::test]
    async fn test_search_functionality() {
        let redis = RedisManager::new().await.unwrap();
        let index = SearchIndex::new("test-idx");
        
        // Create index
        index.create(&redis).await.unwrap();
        
        // Store test data
        let test_entries = vec![
            json!({
                "agent_id": "test_agent_1",
                "category": "api_docs",
                "key": "test_doc_1",
                "content": "This is a test document about Redis search implementation",
                "tags": ["redis", "search", "test"],
                "metadata": {}
            }),
            json!({
                "agent_id": "test_agent_1",
                "category": "code_patterns",
                "key": "test_pattern_1",
                "content": "Pattern for implementing fuzzy search with Redis",
                "tags": ["redis", "fuzzy", "search"],
                "metadata": {}
            }),
            json!({
                "agent_id": "test_agent_2",
                "category": "api_docs",
                "key": "test_doc_2",
                "content": "Guide for semantic vector search in Redis",
                "tags": ["redis", "vector", "semantic"],
                "metadata": {}
            })
        ];
        
        for entry in test_entries {
            let args = StoreKnowledgeArgs::from_value(entry).unwrap();
            let _ = crate::tools::memory::store_knowledge(&redis, json!(args)).await;
        }
        
        // Test exact match search
        let exact_params = SearchParams {
            query: "Redis search implementation".to_string(),
            ..Default::default()
        };
        let results = index.advanced_search(&redis, &exact_params).await.unwrap();
        assert!(results["count"].as_u64().unwrap() > 0);
        
        // Test fuzzy search
        let fuzzy_params = SearchParams {
            query: "fuzzy serch".to_string(), // Intentional typo
            fuzzy_distance: Some(2),
            ..Default::default()
        };
        let results = index.advanced_search(&redis, &fuzzy_params).await.unwrap();
        assert!(results["count"].as_u64().unwrap() > 0);
        
        // Test filtering
        let filter_params = SearchParams {
            query: "redis".to_string(),
            filters: vec![("category".to_string(), "api_docs".to_string())],
            ..Default::default()
        };
        let results = index.advanced_search(&redis, &filter_params).await.unwrap();
        assert!(results["count"].as_u64().unwrap() == 2);
        
        // Test numeric filtering
        let time_filter_params = SearchParams {
            query: "redis".to_string(),
            numeric_filters: vec![
                ("created_at".to_string(), 0.0, SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as f64)
            ],
            ..Default::default()
        };
        let results = index.advanced_search(&redis, &time_filter_params).await.unwrap();
        assert!(results["count"].as_u64().unwrap() > 0);
        
        // Test highlighting and summarization
        let highlight_params = SearchParams {
            query: "semantic vector search".to_string(),
            highlight: true,
            summarize: true,
            ..Default::default()
        };
        let results = index.advanced_search(&redis, &highlight_params).await.unwrap();
        assert!(results["count"].as_u64().unwrap() > 0);
        
        // Cleanup
        let _: () = redis::cmd("FT.DROPINDEX")
            .arg("test-idx")
            .query_async(&mut redis.get_connection().await.unwrap())
            .await
            .unwrap();
    }
}
