use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;

use crate::{
    auth_extractor::AuthUser, dto::SlideEditorPrefsDto, error::AppResult, models::editor_prefs::SlideEditorPrefs,
    repo, state::AppState,
};

const AUTHOR_ROLES: &[&str] = &["OWNER", "MENTOR", "MEMBER"];

pub async fn get_editor_prefs(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(deck_id): Path<String>,
) -> AppResult<Json<SlideEditorPrefsDto>> {
    auth.require_role(AUTHOR_ROLES)?;
    let prefs = repo::editor_prefs::find_by_deck_and_user(&state, &deck_id, &auth.user_id)
        .await?
        .unwrap_or_else(|| SlideEditorPrefs::default_for(&deck_id, &auth.user_id, bson::DateTime::now()));
    Ok(Json(SlideEditorPrefsDto::from(&prefs)))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEditorPrefsRequest {
    #[serde(default)]
    pub show_grid: bool,
    #[serde(default)]
    pub grid_density: Option<String>,
    #[serde(default)]
    pub show_padding_guides: bool,
    #[serde(default)]
    pub snap_to_grid: bool,
    #[serde(default)]
    pub padding_top: i32,
    #[serde(default)]
    pub padding_right: i32,
    #[serde(default)]
    pub padding_bottom: i32,
    #[serde(default)]
    pub padding_left: i32,
}

fn normalize_grid_density(density: Option<&str>) -> String {
    match density.map(|d| d.trim().to_lowercase()) {
        Some(d) if d == "coarse" || d == "medium" || d == "fine" => d,
        _ => "medium".to_string(),
    }
}

fn clamp_padding(value: i32) -> i32 {
    value.clamp(0, 400)
}

pub async fn update_editor_prefs(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(deck_id): Path<String>,
    Json(body): Json<UpdateEditorPrefsRequest>,
) -> AppResult<Json<SlideEditorPrefsDto>> {
    auth.require_role(AUTHOR_ROLES)?;
    let now = bson::DateTime::now();

    let mut prefs = match repo::editor_prefs::find_by_deck_and_user(&state, &deck_id, &auth.user_id).await? {
        Some(p) => p,
        None => SlideEditorPrefs::default_for(&deck_id, &auth.user_id, now),
    };
    let is_new = prefs.id.is_none();

    prefs.show_grid = body.show_grid;
    prefs.grid_density = normalize_grid_density(body.grid_density.as_deref());
    prefs.show_padding_guides = body.show_padding_guides;
    prefs.snap_to_grid = body.snap_to_grid;
    prefs.padding_top = clamp_padding(body.padding_top);
    prefs.padding_right = clamp_padding(body.padding_right);
    prefs.padding_bottom = clamp_padding(body.padding_bottom);
    prefs.padding_left = clamp_padding(body.padding_left);
    prefs.updated_at = now;

    let saved = if is_new {
        repo::editor_prefs::insert(&state, prefs).await?
    } else {
        repo::editor_prefs::save(&state, &prefs).await?;
        prefs
    };

    Ok(Json(SlideEditorPrefsDto::from(&saved)))
}
