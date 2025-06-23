use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize, Serialize)]
pub struct TrelloBadges {
    pub attachments: i32,
    pub description: bool,
    pub due: Option<String>,
    pub due_complete: bool,
    pub comments: i32,
    pub votes: i32,
    pub viewing_member_voted: bool,
    pub subscribed: bool,
    pub fogbugz: String,
    pub check_items: i32,
    pub check_items_checked: i32,
    pub check_items_earliest_due: Option<String>,
    pub last_updated_by_ai: bool,
    pub start: Option<String>,
    pub external_source: Option<String>,
    pub location: bool,
    pub malicious_attachments: i32,
}

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
    #[serde(default)]
    pub desc: String,
    #[serde(rename = "idList")]
    pub id_list: String,
    #[serde(rename = "idBoard")]
    pub id_board: String,
    pub closed: bool,
    pub url: String,
    #[serde(rename = "shortUrl")]
    pub short_url: String,
    #[serde(rename = "idMembers", default)]
    pub id_members: Vec<String>,
    #[serde(rename = "idLabels", default)]
    pub id_labels: Vec<String>,
    #[serde(default)]
    pub labels: Vec<TrelloLabel>,
    pub due: Option<String>,
    #[serde(rename = "dueComplete")]
    pub due_complete: bool,
    pub pos: f64,
    pub email: Option<String>,
    pub dateLastActivity: String,
    pub badges: Value,
    pub subscribed: bool,
    pub cover: Value,
    pub nodeId: Option<String>,
    pub idChecklists: Vec<String>,
    pub idAttachmentCover: Option<String>,
    pub idShort: i32,
    pub manualCoverAttachment: bool,
    pub shortLink: String,
    pub isTemplate: bool,
    pub cardRole: Option<String>,
    pub mirrorSourceId: Option<String>,
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
    pub pos: i64,
    pub subscribed: bool,
    pub nodeId: Option<String>,
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