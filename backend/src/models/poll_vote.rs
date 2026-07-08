use bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollVote {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "deckId")]
    pub deck_id: String,
    #[serde(rename = "slideId")]
    pub slide_id: String,
    #[serde(rename = "elementId")]
    pub element_id: String,
    #[serde(rename = "choiceIds", default)]
    pub choice_ids: Vec<String>,
    #[serde(rename = "createdAt")]
    pub created_at: bson::DateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: bson::DateTime,
}
