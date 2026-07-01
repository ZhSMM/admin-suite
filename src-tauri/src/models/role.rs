use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub built_in: bool,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
    /// Permission codes this role grants. Resolved at fetch time.
    pub permission_codes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoleCreate {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub sort_order: Option<i64>,
    pub permission_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoleUpdate {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub sort_order: Option<i64>,
    pub permission_ids: Option<Vec<String>>,
}