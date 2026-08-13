use serde::{Deserialize, Serialize};

use crate::models::{
    author_image_asset::AuthorImageAsset, author_poll_template::AuthorPollTemplate, book_session::BookSession,
    editor_prefs::SlideEditorPrefs, slide_deck::{Flow, GuidedAudioAsset, Slide, SlideDeck},
};

/// Response shape for a full slide deck. Slide/Element/Flow/etc. carry no
/// datetime fields (their "id"s are plain client-generated strings, not
/// Mongo ObjectIds) so they're reused as-is -- only the outer struct's
/// id/createdAt/updatedAt need the bson -> chrono/String conversion at
/// this DTO boundary (see the auth port's bson/chrono write-up for why
/// that conversion has to happen explicitly).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlideDeckDto {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_summary: Option<String>,
    pub learning_objectives: Vec<String>,
    pub requirements: Vec<String>,
    pub target_audience: Vec<String>,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructor_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_duration_minutes: Option<i32>,
    pub status: String,
    pub owner_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub community_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compare_at_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paywall_start_slide_index: Option<i32>,
    pub slides: Vec<Slide>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow: Option<Flow>,
    pub gallery_images: Vec<String>,
    pub guided_audio_library: Vec<GuidedAudioAsset>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<&SlideDeck> for SlideDeckDto {
    fn from(d: &SlideDeck) -> Self {
        SlideDeckDto {
            id: d.id_string(),
            name: d.name.clone(),
            slug: d.slug.clone(),
            subtitle: d.subtitle.clone(),
            cover_url: d.cover_url.clone(),
            description: d.description.clone(),
            long_summary: d.long_summary.clone(),
            learning_objectives: d.learning_objectives.clone(),
            requirements: d.requirements.clone(),
            target_audience: d.target_audience.clone(),
            tags: d.tags.clone(),
            level: d.level.clone(),
            language: d.language.clone(),
            instructor_name: d.instructor_name.clone(),
            estimated_duration_minutes: d.estimated_duration_minutes,
            status: d.status.clone(),
            owner_id: d.owner_id.clone(),
            community_id: d.community_id.clone(),
            event_id: d.event_id.clone(),
            theme: d.theme.clone(),
            transition: d.transition.clone(),
            layout_format: d.layout_format.clone(),
            price: d.price,
            compare_at_price: d.compare_at_price,
            paywall_start_slide_index: d.paywall_start_slide_index,
            slides: d.slides.clone(),
            flow: d.flow.clone(),
            gallery_images: d.gallery_images.clone(),
            guided_audio_library: d.guided_audio_library.clone(),
            created_at: d.created_at.to_chrono(),
            updated_at: d.updated_at.to_chrono(),
        }
    }
}

/// Request shape for create/update. Deliberately has no id/ownerId/
/// createdAt/updatedAt -- those are always server-controlled (ownerId
/// especially: see the security fix in README.md, the Java version took
/// this straight from the request body).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlideDeckInput {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub subtitle: Option<String>,
    #[serde(default)]
    pub cover_url: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub long_summary: Option<String>,
    #[serde(default)]
    pub learning_objectives: Vec<String>,
    #[serde(default)]
    pub requirements: Vec<String>,
    #[serde(default)]
    pub target_audience: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub level: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub instructor_name: Option<String>,
    #[serde(default)]
    pub estimated_duration_minutes: Option<i32>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub community_id: Option<String>,
    #[serde(default)]
    pub event_id: Option<String>,
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub transition: Option<String>,
    #[serde(default)]
    pub layout_format: Option<String>,
    #[serde(default)]
    pub price: Option<f64>,
    #[serde(default)]
    pub compare_at_price: Option<f64>,
    #[serde(default)]
    pub paywall_start_slide_index: Option<i32>,
    #[serde(default)]
    pub slides: Vec<Slide>,
    #[serde(default)]
    pub flow: Option<Flow>,
    #[serde(default)]
    pub gallery_images: Vec<String>,
    #[serde(default)]
    pub guided_audio_library: Vec<GuidedAudioAsset>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlideDeckCatalogItemDto {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructor_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compare_at_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_duration_minutes: Option<i32>,
    pub slide_count: i32,
    pub tags: Vec<String>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<&SlideDeck> for SlideDeckCatalogItemDto {
    fn from(deck: &SlideDeck) -> Self {
        SlideDeckCatalogItemDto {
            id: deck.id_string(),
            slug: deck.slug.clone(),
            name: deck.name.clone(),
            subtitle: deck.subtitle.clone(),
            cover_url: deck.cover_url.clone(),
            description: deck.description.clone(),
            level: deck.level.clone(),
            language: deck.language.clone(),
            instructor_name: deck.instructor_name.clone(),
            price: deck.price,
            compare_at_price: deck.compare_at_price,
            estimated_duration_minutes: deck.estimated_duration_minutes,
            slide_count: deck.slides.len() as i32,
            tags: deck.tags.clone(),
            updated_at: deck.updated_at.to_chrono(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurriculumItemDto {
    pub index: i32,
    pub title: String,
    pub level: i32,
    pub locked: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlideDeckPublicDetailDto {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_summary: Option<String>,
    pub learning_objectives: Vec<String>,
    pub requirements: Vec<String>,
    pub target_audience: Vec<String>,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructor_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compare_at_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_duration_minutes: Option<i32>,
    pub slide_count: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paywall_start_slide_index: Option<i32>,
    pub has_access: bool,
    pub curriculum: Vec<CurriculumItemDto>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// The four DTOs below exist because their models directly reuse
/// bson::DateTime/ObjectId for the (proven) MongoDB round-trip pattern
/// established elsewhere in this port -- returning those types straight
/// through axum::Json would serialize them as BSON Extended JSON
/// (`{"$date": ...}`, `{"$oid": ...}`) instead of plain ISO strings,
/// exactly the bug documented in auth's port write-up. Every response
/// boundary needs an explicit conversion; these are it for the smaller
/// slides sub-resources.

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlideEditorPrefsDto {
    pub deck_id: String,
    pub user_id: String,
    pub show_grid: bool,
    pub grid_density: String,
    pub show_padding_guides: bool,
    pub snap_to_grid: bool,
    pub padding_top: i32,
    pub padding_right: i32,
    pub padding_bottom: i32,
    pub padding_left: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<&SlideEditorPrefs> for SlideEditorPrefsDto {
    fn from(p: &SlideEditorPrefs) -> Self {
        SlideEditorPrefsDto {
            deck_id: p.deck_id.clone(),
            user_id: p.user_id.clone(),
            show_grid: p.show_grid,
            grid_density: p.grid_density.clone(),
            show_padding_guides: p.show_padding_guides,
            snap_to_grid: p.snap_to_grid,
            padding_top: p.padding_top,
            padding_right: p.padding_right,
            padding_bottom: p.padding_bottom,
            padding_left: p.padding_left,
            created_at: p.created_at.to_chrono(),
            updated_at: p.updated_at.to_chrono(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorImageAssetDto {
    pub id: String,
    pub owner_id: String,
    pub file_url: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<&AuthorImageAsset> for AuthorImageAssetDto {
    fn from(a: &AuthorImageAsset) -> Self {
        AuthorImageAssetDto {
            id: a.id.map(|id| id.to_hex()).unwrap_or_default(),
            owner_id: a.owner_id.clone(),
            file_url: a.file_url.clone(),
            created_at: a.created_at.to_chrono(),
            updated_at: a.updated_at.to_chrono(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorPollTemplateDto {
    pub id: String,
    pub owner_id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll_style_json: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<&AuthorPollTemplate> for AuthorPollTemplateDto {
    fn from(t: &AuthorPollTemplate) -> Self {
        AuthorPollTemplateDto {
            id: t.id_string(),
            owner_id: t.owner_id.clone(),
            title: t.title.clone(),
            poll_style_json: t.poll_style_json.clone(),
            created_at: t.created_at.to_chrono(),
            updated_at: t.updated_at.to_chrono(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookSessionDto {
    pub id: String,
    pub deck_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anon_session_id: Option<String>,
    pub variables: serde_json::Value,
    pub history: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_slide_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<&BookSession> for BookSessionDto {
    fn from(s: &BookSession) -> Self {
        BookSessionDto {
            id: s.id_string(),
            deck_id: s.deck_id.clone(),
            user_id: s.user_id.clone(),
            anon_session_id: s.anon_session_id.clone(),
            variables: s.variables.clone(),
            history: s.history.clone(),
            current_slide_id: s.current_slide_id.clone(),
            created_at: s.created_at.to_chrono(),
            updated_at: s.updated_at.to_chrono(),
        }
    }
}
