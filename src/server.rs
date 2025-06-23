use crate::schemas::*;
use crate::tools::{database, heartbeat, memory, tasks};
use crate::utils::RedisManager;
use anyhow::Result;
use log::{error, info};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub struct MCPServer {
    redis: RedisManager,
    trello_client: reqwest::Client,
    address: String,
    port: u16,
}

impl MCPServer {
    pub async fn new(redis: RedisManager) -> Result<Self> {
        let trello_client = reqwest::Client::new();

        info!("MCP Server initialized with enhanced database capabilities");
        
        Ok(Self {
            redis,
            trello_client,
            address: String::from("127.0.0.1"),
            port: 8080,
        })
    }

    pub async fn run(&self) -> Result<()> {
        info!("Warp MCP server running on stdio");
        
        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut stdout = tokio::io::stdout();
        
        loop {
            let mut line = String::new();
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break, // EOF
                Ok(_) => {
                    if let Some(response) = self.handle_request(&line).await {
                        let response_str = serde_json::to_string(&response)?;
                        stdout.write_all(response_str.as_bytes()).await?;
                        stdout.write_all(b"\n").await?;
                        stdout.flush().await?;
                    }
                }
                Err(e) => {
                    error!("Error reading from stdin: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_request(&self, line: &str) -> Option<Value> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }

        let request: Value = match serde_json::from_str(line) {
            Ok(req) => req,
            Err(e) => {
                error!("Failed to parse JSON request: {}", e);
                return Some(self.error_response(None, -32700, "Parse error"));
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
            Ok(content) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [{"type": "text", "text": content}]
                }
            }),
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

    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn port(&self) -> u16 {
        self.port
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