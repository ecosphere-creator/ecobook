use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    auth_extractor::AuthUser, dto::AuthorImageAssetDto, error::AppResult, models::author_image_asset::AuthorImageAsset,
    repo, state::AppState,
};

/// Every handler here is already scoped to the caller's own `user_id` (see
/// the `repo::author_image_assets::find_by_owner*` calls below) -- this is
/// a per-author personal image library, not a platform-wide resource, so it
/// should use the same author role set as the rest of this domain
/// (slide_decks.rs, files.rs, editor_prefs.rs, poll_votes.rs) rather than
/// platform-OWNER-only. Gating on OWNER blocked every non-owner author from
/// using their own image library when inserting an image into their own
/// deck.
const AUTHOR_ROLES: &[&str] = &["OWNER", "MENTOR", "MEMBER"];

fn normalize(url: &str) -> String {
    url.trim().to_string()
}

pub async fn list(State(state): State<AppState>, auth: AuthUser) -> AppResult<Json<Vec<AuthorImageAssetDto>>> {
    auth.require_role(AUTHOR_ROLES)?;
    let assets = repo::author_image_assets::find_by_owner(&state, &auth.user_id).await?;
    Ok(Json(assets.iter().map(AuthorImageAssetDto::from).collect()))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddAssetRequest {
    #[serde(default)]
    pub file_url: String,
}

pub async fn add(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<AddAssetRequest>,
) -> AppResult<Json<AuthorImageAssetDto>> {
    auth.require_role(AUTHOR_ROLES)?;
    let file_url = normalize(&body.file_url);
    let now = bson::DateTime::now();

    let asset = match repo::author_image_assets::find_by_owner_and_file_url(&state, &auth.user_id, &file_url).await? {
        Some(mut existing) => {
            existing.updated_at = now;
            repo::author_image_assets::save(&state, &existing).await?;
            existing
        }
        None => {
            repo::author_image_assets::insert(
                &state,
                AuthorImageAsset {
                    id: None,
                    owner_id: auth.user_id.clone(),
                    file_url,
                    created_at: now,
                    updated_at: now,
                },
            )
            .await?
        }
    };
    Ok(Json(AuthorImageAssetDto::from(&asset)))
}

#[derive(Debug, Deserialize)]
pub struct FileUrlQuery {
    #[serde(rename = "fileUrl")]
    pub file_url: String,
}

pub async fn remove(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<FileUrlQuery>,
) -> AppResult<StatusCode> {
    auth.require_role(AUTHOR_ROLES)?;
    repo::author_image_assets::delete_by_owner_and_file_url(&state, &auth.user_id, &normalize(&q.file_url)).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageUsage {
    pub deck_id: String,
    pub deck_name: Option<String>,
    pub slide_id: String,
    pub slide_index: i32,
    pub element_id: String,
}

pub async fn usage(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<FileUrlQuery>,
) -> AppResult<Json<Vec<ImageUsage>>> {
    auth.require_role(AUTHOR_ROLES)?;
    let normalized = normalize(&q.file_url);

    let decks = repo::slide_decks::find_by_owner(&state, &auth.user_id).await?;
    let mut usage = Vec::new();
    for deck in &decks {
        for (i, slide) in deck.slides.iter().enumerate() {
            for element in &slide.elements {
                if element.element_type.as_deref() != Some("image") {
                    continue;
                }
                let content = element.content.as_deref().map(normalize).unwrap_or_default();
                if content != normalized {
                    continue;
                }
                usage.push(ImageUsage {
                    deck_id: deck.id_string(),
                    deck_name: deck.name.clone(),
                    slide_id: slide.id.clone(),
                    slide_index: i as i32,
                    element_id: element.id.clone(),
                });
            }
        }
    }
    Ok(Json(usage))
}
