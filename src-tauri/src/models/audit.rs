use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: String,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    pub action: String,
    pub resource: Option<String>,
    pub target_id: Option<String>,
    pub payload: Option<String>,
    pub ip: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuditQuery {
    pub action: Option<String>,
    pub actor_id: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}