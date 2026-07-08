use bson::doc;
use mongodb::Collection;

use crate::{error::AppError, models::editor_prefs::SlideEditorPrefs, state::AppState};

fn collection(state: &AppState) -> Collection<SlideEditorPrefs> {
    state.db.collection("slide_deck_editor_prefs")
}

pub async fn find_by_deck_and_user(
    state: &AppState,
    deck_id: &str,
    user_id: &str,
) -> Result<Option<SlideEditorPrefs>, AppError> {
    Ok(collection(state)
        .find_one(doc! { "deckId": deck_id, "userId": user_id }, None)
        .await?)
}

pub async fn insert(state: &AppState, prefs: SlideEditorPrefs) -> Result<SlideEditorPrefs, AppError> {
    let result = collection(state).insert_one(&prefs, None).await?;
    let id = result
        .inserted_id
        .as_object_id()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("insert did not return an ObjectId")))?;
    Ok(SlideEditorPrefs { id: Some(id), ..prefs })
}

pub async fn save(state: &AppState, prefs: &SlideEditorPrefs) -> Result<(), AppError> {
    let oid = prefs
        .id
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("prefs has no id")))?;
    collection(state).replace_one(doc! { "_id": oid }, prefs, None).await?;
    Ok(())
}
