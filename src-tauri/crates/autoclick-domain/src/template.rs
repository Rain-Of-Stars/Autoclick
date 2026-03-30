use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TemplateRef {
    pub id: Uuid,
    pub name: String,
    pub hash: String,
    pub source_path: Option<String>,
    pub stored_path: Option<String>,
    pub width: u32,
    pub height: u32,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl TemplateRef {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            hash: String::new(),
            source_path: None,
            stored_path: None,
            width: 0,
            height: 0,
            tags: Vec::new(),
            created_at: Utc::now(),
        }
    }
}
