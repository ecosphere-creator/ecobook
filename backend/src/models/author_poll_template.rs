use bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorPollTemplate {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    #[serde(rename = "ownerId")]
    pub owner_id: String,
    pub title: String,
    #[serde(rename = "pollStyleJson", skip_serializing_if = "Option::is_none")]
    pub poll_style_json: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: bson::DateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: bson::DateTime,
}

impl AuthorPollTemplate {
    pub fn id_string(&self) -> String {
        self.id.map(|id| id.to_hex()).unwrap_or_default()
    }
}
