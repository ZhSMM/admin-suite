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

/// All filter fields are optional.  `from` / `to` are ISO-8601 strings that
/// get compared against `created_at` lexically — that works because we write
/// timestamps in the canonical `YYYY-MM-DDTHH:MM:SS.fffZ` format.  If the
/// frontend wants "last 24h" it can compute `now - 86400` itself.
#[derive(Debug, Clone, Deserialize)]
pub struct AuditQuery {
    pub action: Option<String>,
    pub actor_id: Option<String>,
    pub resource: Option<String>,
    pub payload_search: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}