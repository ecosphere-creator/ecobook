use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    auth_extractor::AuthUser, dto::BookSessionDto, error::{AppError, AppResult}, models::book_session::BookSession,
    repo, state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct AnonQuery {
    #[serde(default, rename = "anonSessionId")]
    pub anon_session_id: Option<String>,
}

fn new_session(deck_id: &str, user_id: Option<&str>, anon_session_id: Option<&str>, now: bson::DateTime) -> BookSession {
    BookSession {
        id: None,
        deck_id: deck_id.to_string(),
        user_id: user_id.map(|s| s.to_string()),
        anon_session_id: anon_session_id.map(|s| s.to_string()),
        variables: json!({}),
        history: Vec::new(),
        current_slide_id: None,
        created_at: now,
        updated_at: now,
    }
}

pub async fn get_session(
    State(state): State<AppState>,
    auth: Option<AuthUser>,
    Path(deck_id): Path<String>,
    Query(q): Query<AnonQuery>,
) -> AppResult<Json<BookSessionDto>> {
    let user_id = auth.as_ref().map(|a| a.user_id.as_str());

    if let Some(user_id) = user_id {
        if let Some(existing) = repo::book_sessions::find_by_deck_and_user(&state, &deck_id, user_id).await? {
            return Ok(Json(BookSessionDto::from(&existing)));
        }
        let created = repo::book_sessions::insert(&state, new_session(&deck_id, Some(user_id), None, bson::DateTime::now())).await?;
        return Ok(Json(BookSessionDto::from(&created)));
    }

    if let Some(anon_id) = q.anon_session_id.as_deref().filter(|s| !s.trim().is_empty()) {
        if let Some(existing) = repo::book_sessions::find_by_deck_and_anon(&state, &deck_id, anon_id).await? {
            return Ok(Json(BookSessionDto::from(&existing)));
        }
        let created = repo::book_sessions::insert(&state, new_session(&deck_id, None, Some(anon_id), bson::DateTime::now())).await?;
        return Ok(Json(BookSessionDto::from(&created)));
    }

    Err(AppError::BadRequest("userId or anonSessionId is required".to_string()))
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpsertSessionRequest {
    #[serde(default)]
    pub variables: Option<Value>,
    #[serde(default)]
    pub history: Option<Vec<String>>,
    #[serde(default)]
    pub current_slide_id: Option<String>,
}

fn normalize_history(raw: Option<Vec<String>>) -> Option<Vec<String>> {
    let raw = raw?;
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for item in raw {
        let trimmed = item.trim().to_string();
        if !trimmed.is_empty() && seen.insert(trimmed.clone()) {
            out.push(trimmed);
        }
    }
    Some(out)
}

fn apply_incoming(target: &mut BookSession, incoming: &UpsertSessionRequest) {
    if let Some(vars) = &incoming.variables {
        target.variables = vars.clone();
    }
    if let Some(history) = normalize_history(incoming.history.clone()) {
        target.history = history;
    }
    if incoming.current_slide_id.is_some() {
        target.current_slide_id = incoming.current_slide_id.clone();
    }
}

pub async fn upsert_session(
    State(state): State<AppState>,
    auth: Option<AuthUser>,
    Path(deck_id): Path<String>,
    Query(q): Query<AnonQuery>,
    body: Option<Json<UpsertSessionRequest>>,
) -> AppResult<Json<BookSessionDto>> {
    let incoming = body.map(|Json(b)| b).unwrap_or_default();
    let user_id = auth.as_ref().map(|a| a.user_id.as_str());
    let now = bson::DateTime::now();

    if let Some(user_id) = user_id {
        let mut session = match repo::book_sessions::find_by_deck_and_user(&state, &deck_id, user_id).await? {
            Some(s) => s,
            None => new_session(&deck_id, Some(user_id), None, now),
        };
        let is_new = session.id.is_none();
        apply_incoming(&mut session, &incoming);
        session.updated_at = now;
        let saved = if is_new {
            repo::book_sessions::insert(&state, session).await?
        } else {
            repo::book_sessions::save(&state, &session).await?;
            session
        };
        return Ok(Json(BookSessionDto::from(&saved)));
    }

    if let Some(anon_id) = q.anon_session_id.as_deref().filter(|s| !s.trim().is_empty()) {
        let mut session = match repo::book_sessions::find_by_deck_and_anon(&state, &deck_id, anon_id).await? {
            Some(s) => s,
            None => new_session(&deck_id, None, Some(anon_id), now),
        };
        let is_new = session.id.is_none();
        apply_incoming(&mut session, &incoming);
        session.updated_at = now;
        let saved = if is_new {
            repo::book_sessions::insert(&state, session).await?
        } else {
            repo::book_sessions::save(&state, &session).await?;
            session
        };
        return Ok(Json(BookSessionDto::from(&saved)));
    }

    Err(AppError::BadRequest("userId or anonSessionId is required".to_string()))
}

#[derive(Debug, Deserialize)]
pub struct MergeRequest {
    #[serde(default, rename = "anonSessionId")]
    pub anon_session_id: Option<String>,
}

/// Merge an anonymous session into a user session. User values win on
/// conflict; anon values fill in only what's missing; history is
/// concatenated then de-duped, preserving order.
pub async fn merge_anon_into_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(deck_id): Path<String>,
    Json(body): Json<MergeRequest>,
) -> AppResult<Json<Value>> {
    let anon_id = body
        .anon_session_id
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| AppError::BadRequest("anonSessionId is required".to_string()))?;

    let anon = repo::book_sessions::find_by_deck_and_anon(&state, &deck_id, anon_id).await?;
    let user = repo::book_sessions::find_by_deck_and_user(&state, &deck_id, &auth.user_id).await?;
    let now = bson::DateTime::now();

    let merged = match (user, anon.clone()) {
        (None, None) => {
            let mut created = new_session(&deck_id, Some(&auth.user_id), None, now);
            created = repo::book_sessions::insert(&state, created).await?;
            created
        }
        (None, Some(anon)) => {
            let mut created = new_session(&deck_id, Some(&auth.user_id), None, now);
            created.variables = anon.variables.clone();
            created.history = normalize_history(Some(anon.history.clone())).unwrap_or_default();
            created.current_slide_id = anon.current_slide_id.clone();
            repo::book_sessions::insert(&state, created).await?
        }
        (Some(mut user), Some(anon)) => {
            if let (Value::Object(user_vars), Value::Object(anon_vars)) = (&mut user.variables, &anon.variables) {
                for (k, v) in anon_vars {
                    user_vars.entry(k.clone()).or_insert_with(|| v.clone());
                }
            }
            let combined: Vec<String> = user.history.iter().chain(anon.history.iter()).cloned().collect();
            user.history = normalize_history(Some(combined)).unwrap_or_default();
            if user.current_slide_id.as_deref().unwrap_or("").is_empty() {
                if let Some(anon_slide) = anon.current_slide_id.clone().filter(|s| !s.is_empty()) {
                    user.current_slide_id = Some(anon_slide);
                }
            }
            user.updated_at = now;
            repo::book_sessions::save(&state, &user).await?;
            user
        }
        (Some(user), None) => user,
    };

    if let Some(anon) = anon {
        repo::book_sessions::delete(&state, &anon).await?;
    }

    Ok(Json(json!({
        "id": merged.id_string(),
        "deckId": merged.deck_id,
        "userId": merged.user_id,
        "variables": merged.variables,
        "history": merged.history,
        "currentSlideId": merged.current_slide_id,
    })))
}
