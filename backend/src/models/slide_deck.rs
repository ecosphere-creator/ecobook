use bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlideDeck {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Immutable once published (SEO stability).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(rename = "coverUrl", skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "longSummary", skip_serializing_if = "Option::is_none")]
    pub long_summary: Option<String>,
    #[serde(rename = "learningObjectives", default)]
    pub learning_objectives: Vec<String>,
    #[serde(default)]
    pub requirements: Vec<String>,
    #[serde(rename = "targetAudience", default)]
    pub target_audience: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(rename = "instructorName", skip_serializing_if = "Option::is_none")]
    pub instructor_name: Option<String>,
    #[serde(rename = "estimatedDurationMinutes", skip_serializing_if = "Option::is_none")]
    pub estimated_duration_minutes: Option<i32>,
    /// draft | published
    pub status: String,
    #[serde(rename = "ownerId")]
    pub owner_id: String,
    #[serde(rename = "communityId", skip_serializing_if = "Option::is_none")]
    pub community_id: Option<String>,
    /// Linked to an event.
    #[serde(rename = "eventId", skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<String>,
    /// "presentation" (default) or "responsive". Null treated as "presentation".
    #[serde(rename = "layoutFormat", skip_serializing_if = "Option::is_none")]
    pub layout_format: Option<String>,
    /// Optional price for purchasing access to this slide deck (IDR).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,
    #[serde(rename = "compareAtPrice", skip_serializing_if = "Option::is_none")]
    pub compare_at_price: Option<f64>,
    /// Paywall semantics: null => legacy (fully locked); <=0 => fully
    /// locked from the start; >0 => preview [0, start) for non-entitled;
    /// >= totalSlides => full free in the response.
    #[serde(rename = "paywallStartSlideIndex", skip_serializing_if = "Option::is_none")]
    pub paywall_start_slide_index: Option<i32>,
    #[serde(default)]
    pub slides: Vec<Slide>,
    /// Optional branching flow graph for non-linear playback. If null =>
    /// legacy linear behavior.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow: Option<Flow>,
    #[serde(rename = "galleryImages", default)]
    pub gallery_images: Vec<String>,
    #[serde(rename = "guidedAudioLibrary", default)]
    pub guided_audio_library: Vec<GuidedAudioAsset>,
    #[serde(rename = "createdAt")]
    pub created_at: bson::DateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: bson::DateTime,
}

impl SlideDeck {
    pub fn id_string(&self) -> String {
        self.id.map(|id| id.to_hex()).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slide {
    pub id: String,
    /// Independent page label in tree outline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub elements: Vec<Element>,
    /// Color or image URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    /// Outline level: 0=title, 1=H1, 2=H2, 3=H3 (null = 0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<i32>,
    #[serde(rename = "progressiveReveal", skip_serializing_if = "Option::is_none")]
    pub progressive_reveal: Option<bool>,
    #[serde(rename = "revealOrder", skip_serializing_if = "Option::is_none")]
    pub reveal_order: Option<Vec<String>>,
    #[serde(rename = "guidedReveal", skip_serializing_if = "Option::is_none")]
    pub guided_reveal: Option<GuidedReveal>,
    /// Opaque passthrough for authoring-tool-specific per-slide state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rpp: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Element {
    pub id: String,
    /// "text", "image", "link", "paragraph", "poll", "callout", "input", "choice"
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub element_type: Option<String>,
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    #[serde(default)]
    pub width: f64,
    #[serde(default)]
    pub height: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// JSON string for additional CSS/styling (also carries
    /// `pollTemplateId` for poll elements linked to an author template).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(rename = "linkUrl", skip_serializing_if = "Option::is_none")]
    pub link_url: Option<String>,
    #[serde(rename = "flipH", skip_serializing_if = "Option::is_none")]
    pub flip_h: Option<bool>,
    #[serde(rename = "flipV", skip_serializing_if = "Option::is_none")]
    pub flip_v: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flow {
    #[serde(rename = "startSlideId", skip_serializing_if = "Option::is_none")]
    pub start_slide_id: Option<String>,
    #[serde(default)]
    pub edges: Vec<FlowEdge>,
    #[serde(rename = "nodePositions", skip_serializing_if = "Option::is_none")]
    pub node_positions: Option<serde_json::Value>,
    #[serde(rename = "layoutMode", skip_serializing_if = "Option::is_none")]
    pub layout_mode: Option<String>,
    #[serde(rename = "editorState", skip_serializing_if = "Option::is_none")]
    pub editor_state: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowEdge {
    #[serde(rename = "fromSlideId", skip_serializing_if = "Option::is_none")]
    pub from_slide_id: Option<String>,
    #[serde(rename = "toSlideId", skip_serializing_if = "Option::is_none")]
    pub to_slide_id: Option<String>,
    /// ConditionExpr JSON-ish object. Kept generic to avoid rigid schema.
    #[serde(default)]
    pub condition: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuidedReveal {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(rename = "audioFileId", skip_serializing_if = "Option::is_none")]
    pub audio_file_id: Option<String>,
    #[serde(rename = "durationMs", skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i32>,
    #[serde(default)]
    pub cues: Vec<RevealCue>,
    #[serde(rename = "recordedAt", skip_serializing_if = "Option::is_none")]
    pub recorded_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codec: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevealCue {
    #[serde(rename = "elementId", skip_serializing_if = "Option::is_none")]
    pub element_id: Option<String>,
    #[serde(rename = "atMs", skip_serializing_if = "Option::is_none")]
    pub at_ms: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuidedAudioAsset {
    #[serde(rename = "fileId")]
    pub file_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}
