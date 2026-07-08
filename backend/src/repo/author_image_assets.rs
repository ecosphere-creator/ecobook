use bson::doc;
use mongodb::{options::FindOptions, Collection};

use crate::{error::AppError, models::author_image_asset::AuthorImageAsset, state::AppState};

fn collection(state: &AppState) -> Collection<AuthorImageAsset> {
    state.db.collection("author_image_assets")
}

pub async fn find_by_owner(state: &AppState, owner_id: &str) -> Result<Vec<AuthorImageAsset>, AppError> {
    let mut cursor = collection(state)
        .find(doc! { "ownerId": owner_id }, FindOptions::builder().sort(doc! { "updatedAt": -1 }).build())
        .await?;
    let mut out = Vec::new();
    use futures_util::StreamExt;
    while let Some(asset) = cursor.next().await {
        out.push(asset?);
    }
    Ok(out)
}

pub async fn find_by_owner_and_file_url(
    state: &AppState,
    owner_id: &str,
    file_url: &str,
) -> Result<Option<AuthorImageAsset>, AppError> {
    Ok(collection(state)
        .find_one(doc! { "ownerId": owner_id, "fileUrl": file_url }, None)
        .await?)
}

pub async fn insert(state: &AppState, asset: AuthorImageAsset) -> Result<AuthorImageAsset, AppError> {
    let result = collection(state).insert_one(&asset, None).await?;
    let id = result
        .inserted_id
        .as_object_id()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("insert did not return an ObjectId")))?;
    Ok(AuthorImageAsset { id: Some(id), ..asset })
}

pub async fn save(state: &AppState, asset: &AuthorImageAsset) -> Result<(), AppError> {
    let oid = asset.id.ok_or_else(|| AppError::Internal(anyhow::anyhow!("asset has no id")))?;
    collection(state).replace_one(doc! { "_id": oid }, asset, None).await?;
    Ok(())
}

pub async fn delete_by_owner_and_file_url(state: &AppState, owner_id: &str, file_url: &str) -> Result<(), AppError> {
    collection(state)
        .delete_one(doc! { "ownerId": owner_id, "fileUrl": file_url }, None)
        .await?;
    Ok(())
}
