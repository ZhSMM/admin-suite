use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub display_name: String,
    /// Never sent to the frontend. Use UserSafe for display.
    pub password_hash: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub avatar: Option<String>,
    pub status: String,
    pub is_super_admin: bool,
    pub created_at: String,
    pub updated_at: String,
    pub last_login_at: Option<String>,
    /// Role ids assigned to this user. Resolved at fetch time.
    pub role_ids: Vec<String>,
}

/// Frontend-facing projection (no password hash).
#[derive(Debug, Clone, Serialize)]
pub struct UserSafe {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub avatar: Option<String>,
    pub status: String,
    pub is_super_admin: bool,
    pub created_at: String,
    pub updated_at: String,
    pub last_login_at: Option<String>,
    pub role_ids: Vec<String>,
    pub role_codes: Vec<String>,
}

impl From<User> for UserSafe {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            username: u.username,
            display_name: u.display_name,
            email: u.email,
            phone: u.phone,
            avatar: u.avatar,
            status: u.status,
            is_super_admin: u.is_super_admin,
            created_at: u.created_at,
            updated_at: u.updated_at,
            last_login_at: u.last_login_at,
            role_ids: u.role_ids,
            role_codes: vec![],
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserCreate {
    pub username: String,
    pub display_name: String,
    pub password: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub avatar: Option<String>,
    pub status: Option<String>,
    pub role_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserUpdate {
    pub id: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub avatar: Option<String>,
    pub status: Option<String>,
    pub password: Option<String>,
    pub role_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserListQuery {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub role_id: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserListResult {
    pub items: Vec<UserSafe>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}