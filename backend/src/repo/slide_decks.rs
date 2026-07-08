use bson::{doc, oid::ObjectId, Regex};
use mongodb::Collection;

use crate::{error::AppError, models::slide_deck::SlideDeck, state::AppState};

fn collection(state: &AppState) -> Collection<SlideDeck> {
    state.db.collection("slide_decks")
}

pub async fn find_by_id(state: &AppState, id: &str) -> Result<Option<SlideDeck>, AppError> {
    let Ok(oid) = ObjectId::parse_str(id) else {
        return Ok(None);
    };
    Ok(collection(state).find_one(doc! { "_id": oid }, None).await?)
}

pub async fn find_by_slug(state: &AppState, slug: &str) -> Result<Option<SlideDeck>, AppError> {
    Ok(collection(state).find_one(doc! { "slug": slug }, None).await?)
}

/// Mirrors `findById(id).or(() -> findBySlug(id))` from the Java version.
pub async fn find_by_id_or_slug(state: &AppState, id_or_slug: &str) -> Result<Option<SlideDeck>, AppError> {
    if let Some(deck) = find_by_id(state, id_or_slug).await? {
        return Ok(Some(deck));
    }
    find_by_slug(state, id_or_slug).await
}

pub async fn require_by_id_or_slug(state: &AppState, id_or_slug: &str) -> Result<SlideDeck, AppError> {
    find_by_id_or_slug(state, id_or_slug)
        .await?
        .ok_or_else(|| AppError::NotFound("SlideDeck not found".to_string()))
}

pub async fn find_by_owner(state: &AppState, owner_id: &str) -> Result<Vec<SlideDeck>, AppError> {
    let mut cursor = collection(state).find(doc! { "ownerId": owner_id }, None).await?;
    let mut out = Vec::new();
    use futures_util::StreamExt;
    while let Some(deck) = cursor.next().await {
        out.push(deck?);
    }
    Ok(out)
}

pub async fn find_by_event(state: &AppState, event_id: &str) -> Result<Vec<SlideDeck>, AppError> {
    let mut cursor = collection(state).find(doc! { "eventId": event_id }, None).await?;
    let mut out = Vec::new();
    use futures_util::StreamExt;
    while let Some(deck) = cursor.next().await {
        out.push(deck?);
    }
    Ok(out)
}

pub async fn find_published_for_catalog(state: &AppState) -> Result<Vec<SlideDeck>, AppError> {
    let filter = doc! { "status": { "$regex": Regex { pattern: "^published$".to_string(), options: "i".to_string() } } };
    let mut cursor = collection(state).find(filter, None).await?;
    let mut out = Vec::new();
    use futures_util::StreamExt;
    while let Some(deck) = cursor.next().await {
        out.push(deck?);
    }
    Ok(out)
}

pub async fn insert(state: &AppState, deck: SlideDeck) -> Result<SlideDeck, AppError> {
    let result = collection(state).insert_one(&deck, None).await?;
    let id = result
        .inserted_id
        .as_object_id()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("insert did not return an ObjectId")))?;
    Ok(SlideDeck { id: Some(id), ..deck })
}

pub async fn save(state: &AppState, deck: &SlideDeck) -> Result<(), AppError> {
    let oid = deck.id.ok_or_else(|| AppError::Internal(anyhow::anyhow!("deck has no id")))?;
    collection(state).replace_one(doc! { "_id": oid }, deck, None).await?;
    Ok(())
}

pub async fn delete(state: &AppState, id: &str) -> Result<(), AppError> {
    let oid = ObjectId::parse_str(id).map_err(|_| AppError::BadRequest(format!("Invalid deck id: {id}")))?;
    collection(state).delete_one(doc! { "_id": oid }, None).await?;
    Ok(())
}
