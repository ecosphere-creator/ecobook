use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

use crate::{auth_extractor::AuthUser, error::AppResult, models::poll_vote::PollVote, repo, state::AppState};

const VOTER_ROLES: &[&str] = &["OWNER", "MENTOR", "MEMBER"];

fn normalize_choice_ids(raw: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for item in raw {
        let trimmed = item.trim();
        if !trimmed.is_empty() && seen.insert(trimmed.to_string()) {
            out.push(trimmed.to_string());
        }
    }
    out
}

async fn aggregate(
    state: &AppState,
    caller: Option<&str>,
    deck_id: &str,
    slide_id: &str,
    element_id: &str,
) -> AppResult<Value> {
    let votes = repo::poll_votes::find_all_for_element(state, deck_id, slide_id, element_id).await?;
    let mut counts: HashMap<String, i32> = HashMap::new();
    for vote in &votes {
        for choice_id in normalize_choice_ids(&vote.choice_ids) {
            *counts.entry(choice_id).or_insert(0) += 1;
        }
    }

    let my_choice_ids = match caller {
        Some(user_id) => repo::poll_votes::find_one(state, user_id, deck_id, slide_id, element_id)
            .await?
            .map(|v| normalize_choice_ids(&v.choice_ids))
            .unwrap_or_default(),
        None => Vec::new(),
    };

    Ok(json!({
        "totalVoters": votes.len(),
        "countsByChoiceId": counts,
        "myChoiceIds": my_choice_ids,
    }))
}

pub async fn get_poll_votes(
    State(state): State<AppState>,
    auth: Option<AuthUser>,
    Path((deck_id, slide_id, element_id)): Path<(String, String, String)>,
) -> AppResult<Json<Value>> {
    let caller = auth.as_ref().map(|a| a.user_id.as_str());
    Ok(Json(aggregate(&state, caller, &deck_id, &slide_id, &element_id).await?))
}

#[derive(Debug, Deserialize)]
pub struct SubmitVoteRequest {
    #[serde(default, rename = "choiceIds")]
    pub choice_ids: Vec<String>,
}

pub async fn submit_vote(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((deck_id, slide_id, element_id)): Path<(String, String, String)>,
    Json(body): Json<SubmitVoteRequest>,
) -> AppResult<Json<Value>> {
    auth.require_role(VOTER_ROLES)?;
    let normalized = normalize_choice_ids(&body.choice_ids);
    let now = bson::DateTime::now();

    match repo::poll_votes::find_one(&state, &auth.user_id, &deck_id, &slide_id, &element_id).await? {
        Some(mut existing) => {
            existing.choice_ids = normalized;
            existing.updated_at = now;
            repo::poll_votes::save(&state, &existing).await?;
        }
        None => {
            repo::poll_votes::insert(
                &state,
                PollVote {
                    id: None,
                    user_id: auth.user_id.clone(),
                    deck_id: deck_id.clone(),
                    slide_id: slide_id.clone(),
                    element_id: element_id.clone(),
                    choice_ids: normalized,
                    created_at: now,
                    updated_at: now,
                },
            )
            .await?;
        }
    }

    Ok(Json(
        aggregate(&state, Some(&auth.user_id), &deck_id, &slide_id, &element_id).await?,
    ))
}
