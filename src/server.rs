use crate::cache::{ResponseCache, ResponseBuffer, CacheMetrics};
use crate::schemas::*;
use crate::tools::{database, heartbeat, memory, tasks}; // Removed unused 'trello'
use crate::utils::RedisManager;
use anyhow::Result;
use log::{error, info};
use std::time::Duration;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use flate2::write::GzEncoder;
use flate2::Compression;

#[derive(Clone)]
pub struct MCPServer {
    redis: RedisManager,
    trello_client: reqwest::Client,
    response_cache: ResponseCache,
}

impl MCPServer {
    pub async fn new() -> Result<Self> {
        let redis = RedisManager::new().await?;
        let trello_client = reqwest::Client::new();
        let response_cache = ResponseCache::new(
            1000, // Cache up to 1000 responses
            Duration::from_secs(300), // 5 minute TTL
        );

        info!("MCP Server initialized with response caching and enhanced database capabilities");
        
        Ok(Self {
            redis,
            trello_client,
            response_cache,
        })
    }

    pub async fn run(&self) -> Result<()> {
        info!("Warp MCP server running on stdio with request batching");
        
        const MAX_BATCH_SIZE: usize = 100;
        const MAX_REQUEST_SIZE: usize = 10 * 1024 * 1024; // 10MB limit
        const BATCH_TIMEOUT: Duration = Duration::from_millis(50);
        
        let stdin = tokio::io::stdin();
        let mut reader = BufReader::with_capacity(16 * 1024, stdin); // 16KB read buffer
        let mut stdout = tokio::io::stdout();
        
        let mut response_buffer = ResponseBuffer::new(10 * 1024 * 1024); // 10MB buffer
        let mut batch_buffer = Vec::with_capacity(MAX_BATCH_SIZE);
        let mut line_buffer = String::with_capacity(1024); // Pre-allocate 1KB for common request sizes
        
        loop {
            // Start a timeout for batch collection
            let timeout = tokio::time::sleep(BATCH_TIMEOUT);
            tokio::pin!(timeout);
            
            loop {
                // Break inner loop if batch is full
                if batch_buffer.len() >= MAX_BATCH_SIZE {
                    break;
                }
                
                tokio::select! {
                    // Read next line
                    read_result = reader.read_line(&mut line_buffer) => {
                        match read_result {
                            Ok(0) => break, // EOF
                            Ok(n) => {
                                // Check request size limit
                                if n > MAX_REQUEST_SIZE {
                                    error!("Request size {} exceeds limit of {}", n, MAX_REQUEST_SIZE);
                                    continue;
                                }
                                
                                // Parse and add request to batch
                                let line = line_buffer.trim();
                                if !line.is_empty() {
                                    batch_buffer.push(line.to_string());
                                }
                                line_buffer.clear();
                            }
                            Err(e) => {
                                error!("Error reading from stdin: {}", e);
                                break;
                            }
                        }
                    }
                    
                    // Break if timeout elapsed
                    _ = &mut timeout => {
                        break;
                    }
                }
            }
            
                // Process batch in parallel and buffer responses
                if !batch_buffer.is_empty() {
                let mut handles = Vec::with_capacity(batch_buffer.len());
                let mut responses = Vec::with_capacity(batch_buffer.len());
                
                // Spawn tasks for parallel processing
                for request in batch_buffer.drain(..) {
                    let server = self.clone(); // Assumes Clone is implemented
                    handles.push(tokio::spawn(async move {
                        // Use from_slice for zero-copy parsing
                        server.handle_request(request.as_str()).await
                    }));
                }
                
                // Collect responses maintaining order
                for handle in handles {
                    match handle.await {
                        Ok(Some(response)) => {
                            match serde_json::to_string(&response) {
                                Ok(response_str) => responses.push(response_str),
                                Err(e) => {
                                    error!("Failed to serialize response: {}", e);
                                    responses.push(json!({
                                        "jsonrpc": "2.0",
                                        "error": {
                                            "code": -32603,
                                            "message": "Internal error"
                                        }
                                    }).to_string());
                                }
                            }
                        }
                        Ok(None) => {
                            responses.push(json!({
                                "jsonrpc": "2.0",
                                "result": null
                            }).to_string());
                        }
                        Err(e) => {
                            error!("Task processing error: {}", e);
                            responses.push(json!({
                                "jsonrpc": "2.0",
                                "error": {
                                    "code": -32603,
                                    "message": format!("Internal error: {}", e)
                                }
                            }).to_string());
                        }
                    }
                }
                
                // Buffer responses and flush when needed
                for response in responses {
                    if !response_buffer.add(response) || response_buffer.should_flush() {
                        // Buffer full or threshold reached, flush it
                        let buffered = response_buffer.take_buffer();
                        if buffered.len() > 100 { // Compress large batches
                            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                            for resp in buffered {
                                encoder.write_all(resp.as_bytes()).await?;
                                encoder.write_all(b"\n").await?;
                            }
                            let compressed = encoder.finish()?;
                            stdout.write_all(&compressed).await?;
                        } else {
                            for resp in buffered {
                                stdout.write_all(resp.as_bytes()).await?;
                                stdout.write_all(b"\n").await?;
                            }
                        }
                        stdout.flush().await?;
                    }
                }
            }
            
            // Break main loop if EOF was reached
            if batch_buffer.is_empty() {
                break;
            }
        }

        Ok(())
    }

    async fn handle_request(&self, line: &str) -> Option<Value> {
        // Try cache first
        let cache_key = format!("{}", line);
        if let Some(cached_response) = self.response_cache.get(&cache_key).await {
            return Some(cached_response);
        }
        let line = line.trim();
        if line.is_empty() {
            return Some(json!({
                "jsonrpc": "2.0",
                "result": null
            }));
        }
        
        // Zero-copy parsing using from_slice for better performance
        let request: Value = match serde_json::from_str(line) {
            Ok(req) => req,
            Err(e) => {
                error!("Failed to parse JSON request: {}", e);
                // Return detailed error for debugging
                return Some(json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32700,
                        "message": "Parse error",
                        "data": {
                            "details": e.to_string(),
                            "line": line
                        }
                    }
                }));
            }
        };

        let id = request.get("id").cloned();
        let method = match request.get("method").and_then(|m| m.as_str()) {
            Some(method) => method,
            None => {
                return Some(self.error_response(id, -32600, "Invalid Request"));
            }
        };

        let params = request.get("params").cloned().unwrap_or(Value::Null);

        match method {
            "initialize" => Some(self.handle_initialize(id)),
            "tools/list" => Some(self.handle_tools_list(id)),
            "tools/call" => Some(self.handle_tools_call(id, params).await),
            _ => Some(self.error_response(id, -32601, "Method not found")),
        }
    }

    fn handle_initialize(&self, id: Option<Value>) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "warp-tasks-mcp",
                    "version": "1.0.0"
                }
            }
        })
    }

    fn handle_tools_list(&self, id: Option<Value>) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": [
                    // Task Management Tools
                    {
                        "name": "scan_trello_tasks",
                        "description": "List all Trello cards from configured boards - agents MUST use this to find tasks",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "list_filter": {
                                    "type": "string",
                                    "enum": ["todo", "in_progress", "done", "all"],
                                    "description": "Filter cards by list"
                                }
                            },
                            "required": []
                        }
                    },
                    {
                        "name": "take_trello_task",
                        "description": "Claim a Trello task - REQUIRED before working on any task",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "agent_id": {
                                    "type": "string",
                                    "description": "Unique agent identifier"
                                },
                                "card_id": {
                                    "type": "string", 
                                    "description": "Trello card ID to claim"
                                }
                            },
                            "required": ["agent_id", "card_id"]
                        }
                    },
                    {
                        "name": "update_trello_task",
                        "description": "Update task progress, add comments, checklists - MUST be called frequently",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "agent_id": {"type": "string"},
                                "card_id": {"type": "string"},
                                "update_type": {
                                    "type": "string",
                                    "enum": ["comment", "checklist", "description", "move_list"]
                                },
                                "content": {"type": "string"},
                                "list_id": {"type": "string", "description": "For move_list only"}
                            },
                            "required": ["agent_id", "card_id", "update_type", "content"]
                        }
                    },
                    // Memory and Learning Tools
                    {
                        "name": "store_knowledge",
                        "description": "Store task progress, learnings, API docs, or any knowledge with RAG tags",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "agent_id": {"type": "string"},
                                "category": {
                                    "type": "string",
                                    "enum": ["task_progress", "api_docs", "code_patterns", "errors", "solutions", "project_knowledge"]
                                },
                                "key": {"type": "string"},
                                "content": {"type": "string"},
                                "tags": {
                                    "type": "array",
                                    "items": {"type": "string"},
                                    "description": "RAG search tags"
                                },
                                "metadata": {"type": "object"}
                            },
                            "required": ["agent_id", "category", "key", "content", "tags"]
                        }
                    },
                    {
                        "name": "search_knowledge",
                        "description": "RAG search across all stored knowledge using semantic queries",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": {"type": "string"},
                                "category_filter": {"type": "string"},
                                "agent_filter": {"type": "string"},
                                "limit": {"type": "number", "default": 10}
                            },
                            "required": ["query"]
                        }
                    },
                    {
                        "name": "learn_from_agents",
                        "description": "Query what other agents learned about specific topics or errors",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "topic": {"type": "string"},
                                "error_pattern": {"type": "string"},
                                "time_range": {"type": "string", "enum": ["hour", "day", "week", "all"]}
                            },
                            "required": ["topic"]
                        }
                    },
                    // Heartbeat and Coordination
                    {
                        "name": "heartbeat",
                        "description": "Send heartbeat with current task status - MUST be called every 30 seconds",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "agent_id": {"type": "string"},
                                "card_id": {"type": "string"},
                                "status": {"type": "string"},
                                "progress_percentage": {"type": "number"}
                            },
                            "required": ["agent_id", "card_id", "status"]
                        }
                    },
                    {
                        "name": "check_agent_status",
                        "description": "Check what other agents are working on to avoid collisions",
                        "inputSchema": {
                            "type": "object",
                            "properties": {},
                            "required": []
                        }
                    },
                    // Advanced Database Operations
                    {
                        "name": "execute_rag_query",
                        "description": "Execute advanced RAG queries with RedisJSON for complex knowledge retrieval",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "json_path": {"type": "string"},
                                "query": {"type": "string"},
                                "aggregation": {"type": "string"}
                            },
                            "required": ["query"]
                        }
                    }
                ]
            }
        })
    }

    async fn handle_tools_call(&self, id: Option<Value>, params: Value) -> Value {
        let tool_call: ToolCall = match serde_json::from_value(params) {
            Ok(call) => call,
            Err(e) => {
                error!("Invalid tool call parameters: {}", e);
                return self.error_response(id, -32602, "Invalid params");
            }
        };

        let result = match tool_call.name.as_str() {
            "scan_trello_tasks" => {
                tasks::scan_trello_tasks(&self.redis, &self.trello_client, tool_call.arguments).await
            }
            "take_trello_task" => {
                tasks::take_trello_task(&self.redis, &self.trello_client, tool_call.arguments).await
            }
            "update_trello_task" => {
                tasks::update_trello_task(&self.redis, &self.trello_client, tool_call.arguments).await
            }
            "store_knowledge" => {
                memory::store_knowledge(&self.redis, tool_call.arguments).await
            }
            "search_knowledge" => {
                memory::search_knowledge(&self.redis, tool_call.arguments).await
            }
            "learn_from_agents" => {
                memory::learn_from_agents(&self.redis, tool_call.arguments).await
            }
            "heartbeat" => {
                heartbeat::send_heartbeat(&self.redis, tool_call.arguments).await
            }
            "check_agent_status" => {
                heartbeat::check_agent_status(&self.redis).await
            }
            "execute_rag_query" => {
                database::execute_rag_query(&self.redis, tool_call.arguments).await
            }
            _ => {
                return self.error_response(id, -32601, "Unknown tool");
            }
        };

        match result {
            Ok(content) => {
                let response = json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [{"type": "text", "text": content}]
                }
                });
                // Cache successful responses
                self.response_cache.set(cache_key, response.clone()).await;
                response
            },
            Err(e) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "isError": true,
                    "content": [{"type": "text", "text": format!("Error: {}", e)}]
                }
            })
        }
    }

    fn error_response(&self, id: Option<Value>, code: i32, message: &str) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": code,
                "message": message
            }
        })
    }
}