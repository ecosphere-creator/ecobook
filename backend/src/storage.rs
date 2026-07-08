use bson::doc;
use chrono::Utc;
use image::{codecs::webp::WebPEncoder, ExtendedColorType, ImageEncoder, ImageReader};
use mongodb::Collection;
use std::io::Cursor;
use std::path::PathBuf;
use uuid::Uuid;

use crate::{error::AppError, models::file::FileRecord, state::AppState};

/// Same proven pattern as auth/courses/community's storage.rs. "slide-
/// image" was one of the Java FileService's IMAGE_FILE_TYPES (recompressed
/// to lossless WebP), so this mirrors that behavior exactly.
const MAX_IMAGE_PIXELS: u64 = 30_000_000;

pub struct UploadedImage {
    pub file_url: String,
    pub file_id: String,
}

pub async fn upload_slide_image(
    state: &AppState,
    owner_id: &str,
    content_type: Option<&str>,
    bytes: Vec<u8>,
) -> Result<UploadedImage, AppError> {
    match content_type {
        Some(ct) if ct.starts_with("image/") => {}
        _ => return Err(AppError::BadRequest("Invalid file type. Expected an image upload.".to_string())),
    }

    let (width, height) = ImageReader::new(Cursor::new(&bytes))
        .with_guessed_format()
        .map_err(|e| AppError::BadRequest(format!("Failed to read image header: {e}")))?
        .into_dimensions()
        .map_err(|e| AppError::BadRequest(format!("Failed to read image header: {e}")))?;
    if u64::from(width) * u64::from(height) > MAX_IMAGE_PIXELS {
        return Err(AppError::BadRequest(format!(
            "Image is too large ({width}x{height}); maximum is {MAX_IMAGE_PIXELS} pixels"
        )));
    }

    let image = image::load_from_memory(&bytes)
        .map_err(|e| AppError::BadRequest(format!("Failed to decode image: {e}")))?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();

    let mut webp_bytes: Vec<u8> = Vec::new();
    WebPEncoder::new_lossless(&mut webp_bytes)
        .write_image(&rgba, width, height, ExtendedColorType::Rgba8)
        .map_err(|e| AppError::BadRequest(format!("Failed to convert image to WebP: {e}")))?;

    let filename = format!("{}.webp", Uuid::new_v4());
    let storage_path = format!("slide-image/{owner_id}/{filename}");
    let full_path = PathBuf::from(&state.config.storage_local_path)
        .join("slide-image")
        .join(owner_id);
    tokio::fs::create_dir_all(&full_path)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    tokio::fs::write(full_path.join(&filename), &webp_bytes)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    let record = FileRecord {
        id: None,
        filename: filename.clone(),
        file_type: "slide-image".to_string(),
        file_size: webp_bytes.len() as i64,
        storage_path,
        uploaded_by: owner_id.to_string(),
        created_at: Utc::now(),
    };

    let files: Collection<FileRecord> = state.db.collection("files");
    let result = files
        .insert_one(&record, None)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    let file_id = result
        .inserted_id
        .as_object_id()
        .map(|id| id.to_hex())
        .unwrap_or_default();

    let base = state.config.api_base_url.trim_end_matches('/');
    Ok(UploadedImage {
        file_url: format!("{base}/files/view/{file_id}"),
        file_id,
    })
}

pub struct StoredFile {
    pub bytes: Vec<u8>,
    pub content_type: String,
}

pub async fn read_file(state: &AppState, file_id: &str) -> Result<StoredFile, AppError> {
    let record = find_file(state, file_id).await?;
    let full_path = PathBuf::from(&state.config.storage_local_path).join(&record.storage_path);
    let bytes = tokio::fs::read(&full_path)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    Ok(StoredFile {
        content_type: "image/webp".to_string(),
        bytes,
    })
}

pub async fn delete_file(state: &AppState, file_id: &str, requester_id: &str) -> Result<(), AppError> {
    let record = find_file(state, file_id).await?;
    if record.uploaded_by != requester_id {
        return Err(AppError::Forbidden("Cannot delete another user's file".to_string()));
    }
    let full_path = PathBuf::from(&state.config.storage_local_path).join(&record.storage_path);
    let _ = tokio::fs::remove_file(&full_path).await;

    let oid = record.id.ok_or_else(|| AppError::Internal(anyhow::anyhow!("file record has no id")))?;
    let files: Collection<FileRecord> = state.db.collection("files");
    files
        .delete_one(doc! { "_id": oid }, None)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    Ok(())
}

async fn find_file(state: &AppState, file_id: &str) -> Result<FileRecord, AppError> {
    let oid = bson::oid::ObjectId::parse_str(file_id)
        .map_err(|_| AppError::NotFound(format!("File not found: {file_id}")))?;
    let files: Collection<FileRecord> = state.db.collection("files");
    files
        .find_one(doc! { "_id": oid }, None)
        .await
        .map_err(|e| AppError::Internal(e.into()))?
        .ok_or_else(|| AppError::NotFound(format!("File not found: {file_id}")))
}
