use bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlideEditorPrefs {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    #[serde(rename = "deckId")]
    pub deck_id: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "showGrid")]
    pub show_grid: bool,
    /// coarse | medium | fine
    #[serde(rename = "gridDensity")]
    pub grid_density: String,
    #[serde(rename = "showPaddingGuides")]
    pub show_padding_guides: bool,
    #[serde(rename = "snapToGrid")]
    pub snap_to_grid: bool,
    #[serde(rename = "paddingTop")]
    pub padding_top: i32,
    #[serde(rename = "paddingRight")]
    pub padding_right: i32,
    #[serde(rename = "paddingBottom")]
    pub padding_bottom: i32,
    #[serde(rename = "paddingLeft")]
    pub padding_left: i32,
    #[serde(rename = "createdAt")]
    pub created_at: bson::DateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: bson::DateTime,
}

impl SlideEditorPrefs {
    pub fn default_for(deck_id: &str, user_id: &str, now: bson::DateTime) -> Self {
        SlideEditorPrefs {
            id: None,
            deck_id: deck_id.to_string(),
            user_id: user_id.to_string(),
            show_grid: false,
            grid_density: "medium".to_string(),
            show_padding_guides: false,
            snap_to_grid: false,
            padding_top: 64,
            padding_right: 64,
            padding_bottom: 64,
            padding_left: 64,
            created_at: now,
            updated_at: now,
        }
    }
}
