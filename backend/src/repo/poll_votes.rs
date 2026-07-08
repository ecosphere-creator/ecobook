use bson::doc;
use mongodb::Collection;

use crate::{error::AppError, models::poll_vote::PollVote, state::AppState};

fn collection(state: &AppState) -> Collection<PollVote> {
    state.db.collection("poll_votes")
}

pub async fn find_one(
    state: &AppState,
    user_id: &str,
    deck_id: &str,
    slide_id: &str,
    element_id: &str,
) -> Result<Option<PollVote>, AppError> {
    Ok(collection(state)
        .find_one(
            doc! { "userId": user_id, "deckId": deck_id, "slideId": slide_id, "elementId": element_id },
            None,
        )
        .await?)
}

pub async fn find_all_for_element(
    state: &AppState,
    deck_id: &str,
    slide_id: &str,
    element_id: &str,
) -> Result<Vec<PollVote>, AppError> {
    let mut cursor = collection(state)
        .find(doc! { "deckId": deck_id, "slideId": slide_id, "elementId": element_id }, None)
        .await?;
    let mut out = Vec::new();
    use futures_util::StreamExt;
    while let Some(v) = cursor.next().await {
        out.push(v?);
    }
    Ok(out)
}

pub async fn insert(state: &AppState, vote: PollVote) -> Result<(), AppError> {
    collection(state).insert_one(&vote, None).await?;
    Ok(())
}

pub async fn save(state: &AppState, vote: &PollVote) -> Result<(), AppError> {
    let oid = vote.id.ok_or_else(|| AppError::Internal(anyhow::anyhow!("vote has no id")))?;
    collection(state).replace_one(doc! { "_id": oid }, vote, None).await?;
    Ok(())
}
