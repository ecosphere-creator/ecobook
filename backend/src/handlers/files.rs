use axum::{
    extract::{Multipart, Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::{auth_extractor::AuthUser, error::{AppError, AppResult}, state::AppState, storage};

const AUTHOR_ROLES: &[&str] = &["OWNER", "MENTOR", "MEMBER"];

pub async fn upload(
    State(state): State<AppState>,
    auth: AuthUser,
    mut multipart: Multipart,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    auth.require_role(AUTHOR_ROLES)?;

    let mut content_type = None;
    let mut bytes = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Invalid multipart body: {e}")))?
    {
        if field.name() == Some("file") {
            content_type = field.content_type().map(|s| s.to_string());
            bytes = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| AppError::BadRequest(format!("Failed to read upload: {e}")))?
                    .to_vec(),
            );
            break;
        }
    }
    let bytes = bytes.ok_or_else(|| AppError::BadRequest("Missing 'file' field".to_string()))?;

    let uploaded = storage::upload_slide_image(&state, &auth.user_id, content_type.as_deref(), bytes).await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({ "fileUrl": uploaded.file_url, "fileId": uploaded.file_id })),
    ))
}

pub async fn view_file(State(state): State<AppState>, Path(file_id): Path<String>) -> AppResult<Response> {
    let file = storage::read_file(&state, &file_id).await?;
    Ok((StatusCode::OK, [(header::CONTENT_TYPE, file.content_type)], file.bytes).into_response())
}

pub async fn delete_file(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(file_id): Path<String>,
) -> AppResult<StatusCode> {
    auth.require_role(AUTHOR_ROLES)?;
    storage::delete_file(&state, &file_id, &auth.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
