use bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookSession {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    #[serde(rename = "deckId")]
    pub deck_id: String,
    /// Present when authenticated. Null for anonymous sessions.
    #[serde(rename = "userId", skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// Present when anonymous (stored on the client).
    #[serde(rename = "anonSessionId", skip_serializing_if = "Option::is_none")]
    pub anon_session_id: Option<String>,
    #[serde(default = "default_variables")]
    pub variables: serde_json::Value,
    /// Stack of visited slideIds to support true back navigation.
    #[serde(default)]
    pub history: Vec<String>,
    #[serde(rename = "currentSlideId", skip_serializing_if = "Option::is_none")]
    pub current_slide_id: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: bson::DateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: bson::DateTime,
}

fn default_variables() -> serde_json::Value {
    serde_json::json!({})
}

impl BookSession {
    pub fn id_string(&self) -> String {
        self.id.map(|id| id.to_hex()).unwrap_or_default()
    }
}
