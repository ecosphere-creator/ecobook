use bson::{doc, oid::ObjectId};
use mongodb::{options::FindOptions, Collection};

use crate::{error::AppError, models::author_poll_template::AuthorPollTemplate, state::AppState};

fn collection(state: &AppState) -> Collection<AuthorPollTemplate> {
    state.db.collection("author_poll_templates")
}

pub async fn find_by_owner(state: &AppState, owner_id: &str) -> Result<Vec<AuthorPollTemplate>, AppError> {
    let mut cursor = collection(state)
        .find(doc! { "ownerId": owner_id }, FindOptions::builder().sort(doc! { "updatedAt": -1 }).build())
        .await?;
    let mut out = Vec::new();
    use futures_util::StreamExt;
    while let Some(t) = cursor.next().await {
        out.push(t?);
    }
    Ok(out)
}

pub async fn find_by_id_and_owner(
    state: &AppState,
    id: &str,
    owner_id: &str,
) -> Result<Option<AuthorPollTemplate>, AppError> {
    let Ok(oid) = ObjectId::parse_str(id) else {
        return Ok(None);
    };
    Ok(collection(state)
        .find_one(doc! { "_id": oid, "ownerId": owner_id }, None)
        .await?)
}

pub async fn require_by_id_and_owner(
    state: &AppState,
    id: &str,
    owner_id: &str,
) -> Result<AuthorPollTemplate, AppError> {
    find_by_id_and_owner(state, id, owner_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Template not found".to_string()))
}

pub async fn insert(state: &AppState, template: AuthorPollTemplate) -> Result<AuthorPollTemplate, AppError> {
    let result = collection(state).insert_one(&template, None).await?;
    let id = result
        .inserted_id
        .as_object_id()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("insert did not return an ObjectId")))?;
    Ok(AuthorPollTemplate { id: Some(id), ..template })
}

pub async fn save(state: &AppState, template: &AuthorPollTemplate) -> Result<(), AppError> {
    let oid = template
        .id
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("template has no id")))?;
    collection(state).replace_one(doc! { "_id": oid }, template, None).await?;
    Ok(())
}

pub async fn delete(state: &AppState, id: &str) -> Result<(), AppError> {
    let oid = ObjectId::parse_str(id).map_err(|_| AppError::BadRequest(format!("Invalid template id: {id}")))?;
    collection(state).delete_one(doc! { "_id": oid }, None).await?;
    Ok(())
}
