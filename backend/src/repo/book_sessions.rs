use bson::doc;
use mongodb::Collection;

use crate::{error::AppError, models::book_session::BookSession, state::AppState};

fn collection(state: &AppState) -> Collection<BookSession> {
    state.db.collection("book_sessions")
}

pub async fn find_by_deck_and_user(
    state: &AppState,
    deck_id: &str,
    user_id: &str,
) -> Result<Option<BookSession>, AppError> {
    Ok(collection(state)
        .find_one(doc! { "deckId": deck_id, "userId": user_id }, None)
        .await?)
}

pub async fn find_by_deck_and_anon(
    state: &AppState,
    deck_id: &str,
    anon_session_id: &str,
) -> Result<Option<BookSession>, AppError> {
    Ok(collection(state)
        .find_one(doc! { "deckId": deck_id, "anonSessionId": anon_session_id }, None)
        .await?)
}

pub async fn insert(state: &AppState, session: BookSession) -> Result<BookSession, AppError> {
    let result = collection(state).insert_one(&session, None).await?;
    let id = result
        .inserted_id
        .as_object_id()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("insert did not return an ObjectId")))?;
    Ok(BookSession { id: Some(id), ..session })
}

pub async fn save(state: &AppState, session: &BookSession) -> Result<(), AppError> {
    let oid = session
        .id
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("session has no id")))?;
    collection(state).replace_one(doc! { "_id": oid }, session, None).await?;
    Ok(())
}

pub async fn delete(state: &AppState, session: &BookSession) -> Result<(), AppError> {
    if let Some(oid) = session.id {
        collection(state).delete_one(doc! { "_id": oid }, None).await?;
    }
    Ok(())
}
