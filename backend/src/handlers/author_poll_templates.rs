use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    auth_extractor::AuthUser, dto::AuthorPollTemplateDto, error::AppResult,
    models::author_poll_template::AuthorPollTemplate, repo, state::AppState,
};

pub async fn list(State(state): State<AppState>, auth: AuthUser) -> AppResult<Json<Vec<AuthorPollTemplateDto>>> {
    auth.require_role(&["OWNER"])?;
    let templates = repo::author_poll_templates::find_by_owner(&state, &auth.user_id).await?;
    Ok(Json(templates.iter().map(AuthorPollTemplateDto::from).collect()))
}

#[derive(Debug, Deserialize)]
pub struct TemplateBody {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(rename = "pollStyleJson", default)]
    pub poll_style_json: Option<String>,
}

/// Injects/overwrites `pollTemplateId` in the style JSON blob so linked
/// poll elements can be traced back to their template -- mirrors
/// AuthorPollTemplateService.normalizeStyleJsonWithTemplateId.
fn normalize_style_json(style_json: Option<&str>, template_id: Option<&str>) -> String {
    let mut parsed: Value = style_json
        .filter(|s| !s.trim().is_empty())
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_else(|| json!({}));
    if let (Some(id), Some(obj)) = (template_id, parsed.as_object_mut()) {
        if !id.trim().is_empty() {
            obj.insert("pollTemplateId".to_string(), json!(id));
        }
    }
    serde_json::to_string(&parsed).unwrap_or_else(|_| style_json.unwrap_or("{}").to_string())
}

fn extract_poll_template_id(style_json: Option<&str>) -> Option<String> {
    let value: Value = serde_json::from_str(style_json?).ok()?;
    value.get("pollTemplateId")?.as_str().map(|s| s.to_string())
}

pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<TemplateBody>,
) -> AppResult<Json<AuthorPollTemplateDto>> {
    auth.require_role(&["OWNER"])?;
    let now = bson::DateTime::now();
    let title = body.title.as_deref().map(str::trim).filter(|s| !s.is_empty()).unwrap_or("Poll Template").to_string();

    let template = repo::author_poll_templates::insert(
        &state,
        AuthorPollTemplate {
            id: None,
            owner_id: auth.user_id.clone(),
            title,
            poll_style_json: Some(normalize_style_json(body.poll_style_json.as_deref(), None)),
            created_at: now,
            updated_at: now,
        },
    )
    .await?;
    Ok(Json(AuthorPollTemplateDto::from(&template)))
}

#[derive(Debug, Deserialize)]
pub struct UpdateQuery {
    #[serde(default = "default_true", rename = "syncLinked")]
    pub sync_linked: bool,
}

fn default_true() -> bool {
    true
}

pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(template_id): Path<String>,
    Query(q): Query<UpdateQuery>,
    Json(body): Json<TemplateBody>,
) -> AppResult<Json<AuthorPollTemplateDto>> {
    auth.require_role(&["OWNER"])?;
    let mut existing = repo::author_poll_templates::require_by_id_and_owner(&state, &template_id, &auth.user_id).await?;

    if let Some(title) = body.title.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        existing.title = title.to_string();
    }
    existing.poll_style_json = Some(normalize_style_json(body.poll_style_json.as_deref(), Some(&template_id)));
    existing.updated_at = bson::DateTime::now();
    repo::author_poll_templates::save(&state, &existing).await?;

    if q.sync_linked {
        sync_linked_polls(&state, &auth.user_id, &template_id, existing.poll_style_json.as_deref()).await?;
    }

    Ok(Json(AuthorPollTemplateDto::from(&existing)))
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(template_id): Path<String>,
) -> AppResult<StatusCode> {
    auth.require_role(&["OWNER"])?;
    let existing = repo::author_poll_templates::require_by_id_and_owner(&state, &template_id, &auth.user_id).await?;
    repo::author_poll_templates::delete(&state, &existing.id_string()).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PollTemplateUsage {
    pub deck_id: String,
    pub deck_name: Option<String>,
    pub slide_id: String,
    pub slide_index: i32,
    pub element_id: String,
}

pub async fn usage(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(template_id): Path<String>,
) -> AppResult<Json<Vec<PollTemplateUsage>>> {
    auth.require_role(&["OWNER"])?;
    let decks = repo::slide_decks::find_by_owner(&state, &auth.user_id).await?;
    let mut usage = Vec::new();
    for deck in &decks {
        for (i, slide) in deck.slides.iter().enumerate() {
            for element in &slide.elements {
                if element.element_type.as_deref() != Some("poll") {
                    continue;
                }
                if extract_poll_template_id(element.style.as_deref()).as_deref() != Some(template_id.as_str()) {
                    continue;
                }
                usage.push(PollTemplateUsage {
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

pub async fn sync(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(template_id): Path<String>,
) -> AppResult<Json<Value>> {
    auth.require_role(&["OWNER"])?;
    let template = repo::author_poll_templates::require_by_id_and_owner(&state, &template_id, &auth.user_id).await?;
    let updated = sync_linked_polls(&state, &auth.user_id, &template_id, template.poll_style_json.as_deref()).await?;
    Ok(Json(json!({ "updatedElements": updated })))
}

async fn sync_linked_polls(
    state: &AppState,
    owner_id: &str,
    template_id: &str,
    poll_style_json: Option<&str>,
) -> AppResult<i32> {
    let normalized = normalize_style_json(poll_style_json, Some(template_id));
    let mut changed = 0;
    let decks = repo::slide_decks::find_by_owner(state, owner_id).await?;
    for mut deck in decks {
        let mut deck_updated = false;
        for slide in &mut deck.slides {
            for element in &mut slide.elements {
                if element.element_type.as_deref() != Some("poll") {
                    continue;
                }
                if extract_poll_template_id(element.style.as_deref()).as_deref() != Some(template_id) {
                    continue;
                }
                element.style = Some(normalized.clone());
                changed += 1;
                deck_updated = true;
            }
        }
        if deck_updated {
            deck.updated_at = bson::DateTime::now();
            repo::slide_decks::save(state, &deck).await?;
        }
    }
    Ok(changed)
}
