use serde::{Deserialize, Serialize};

/// Anything that lives in the `resources` table: themes, locales, ...
/// The `content` field is a JSON string whose shape depends on `resource_type`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub id: String,
    pub resource_type: String,
    pub code: String,
    pub name: String,
    pub content: String,
    pub source: String,
    pub built_in: bool,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResourceImport {
    pub resource_type: String, // "theme" | "locale"
    pub code: String,
    pub name: String,
    /// JSON content. Validated against the type's expected shape on import.
    pub content: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResourceUpdate {
    pub id: String,
    pub name: Option<String>,
    pub content: Option<serde_json::Value>,
}

/// Theme payload (parsed from `content` JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub is_dark: bool,
    pub tokens: serde_json::Map<String, serde_json::Value>,
}

/// Locale payload (parsed from `content` JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Locale {
    pub id: String,
    pub label: String,
    pub messages: serde_json::Map<String, serde_json::Value>,
}