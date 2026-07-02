use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Menu {
    pub id: String,
    pub parent_id: Option<String>,
    pub code: String,
    pub title: String,
    /// i18n key for the sidebar title. The frontend looks this up with `t()`;
    /// if missing or empty, the raw `title` field is shown.
    pub title_key: Option<String>,
    pub path: Option<String>,
    pub icon: Option<String>,
    pub component: Option<String>,
    pub sort_order: i64,
    pub visible: bool,
    pub status: String,
    pub menu_type: String, // menu | group | button
    pub permission_code: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MenuCreate {
    pub parent_id: Option<String>,
    pub code: String,
    pub title: String,
    pub title_key: Option<String>,
    pub path: Option<String>,
    pub icon: Option<String>,
    pub component: Option<String>,
    pub sort_order: Option<i64>,
    pub visible: Option<bool>,
    pub status: Option<String>,
    pub menu_type: Option<String>,
    pub permission_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MenuUpdate {
    pub id: String,
    pub parent_id: Option<Option<String>>,
    pub title: Option<String>,
    pub title_key: Option<Option<String>>,
    pub path: Option<Option<String>>,
    pub icon: Option<Option<String>>,
    pub component: Option<Option<String>>,
    pub sort_order: Option<i64>,
    pub visible: Option<bool>,
    pub status: Option<String>,
    pub menu_type: Option<String>,
    pub permission_code: Option<Option<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MenuNode {
    #[serde(flatten)]
    pub menu: Menu,
    pub children: Vec<MenuNode>,
}