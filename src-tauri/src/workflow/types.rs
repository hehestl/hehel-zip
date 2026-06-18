use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowStatus {
    pub id: String,
    pub label: String,
    pub color: String,
    pub sort_order: i64,
    pub is_default: bool,
}

pub type EntryStatusMap = HashMap<String, String>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionLogEntry {
    pub id: i64,
    pub archive_id: String,
    pub archive_path: Option<String>,
    pub action_type: String,
    pub entry_path: Option<String>,
    pub from_status_id: Option<String>,
    pub to_status_id: Option<String>,
    pub detail: Option<String>,
    pub created_at: String,
}
