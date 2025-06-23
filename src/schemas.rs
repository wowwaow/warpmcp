use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize, Serialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TakeTaskArgs {
    pub agent_id: String,
    pub card_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateTaskArgs {
    pub agent_id: String,
    pub card_id: String,
    pub update_type: String,
    pub content: String,
    pub list_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StoreKnowledgeArgs {
    pub agent_id: String,
    pub category: String,
    pub key: String,
    pub content: String,
    pub tags: Vec<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SearchKnowledgeArgs {
    pub query: String,
    pub category_filter: Option<String>,
    pub agent_filter: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HeartbeatArgs {
    pub agent_id: String,
    pub card_id: String,
    pub status: String,
    pub progress_percentage: Option<f32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TrelloCard {
    pub id: String,
    pub name: String,
    pub desc: String,
    #[serde(rename = "idList")]
    pub id_list: String,
    #[serde(rename = "idBoard")]
    pub id_board: String,
    pub closed: bool,
    pub url: String,
    pub shortUrl: Option<String>,
    #[serde(rename = "idMembers")]
    pub id_members: Vec<String>,
    pub labels: Vec<TrelloLabel>,
    pub due: Option<String>,
    pub dueComplete: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TrelloLabel {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TrelloList {
    pub id: String,
    pub name: String,
    #[serde(rename = "idBoard")]
    pub id_board: String,
    pub closed: bool,
    pub pos: f64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AgentStatus {
    pub agent_id: String,
    pub card_id: String,
    pub card_name: String,
    pub status: String,
    pub last_heartbeat: i64,
    pub progress: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct KnowledgeEntry {
    pub id: String,
    pub agent_id: String,
    pub category: String,
    pub key: String,
    pub content: String,
    pub tags: Vec<String>,
    pub metadata: Value,
    pub created_at: i64,
    pub updated_at: i64,
    pub access_count: u32,
}