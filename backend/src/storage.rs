use bson::doc;
use chrono::Utc;
use image::ImageReader;
use mongodb::Collection;
use std::io::Cursor;
use std::path::PathBuf;
use uuid::Uuid;

use crate::{config::StorageBackend, error::AppError, models::file::FileRecord, s3_storage, state::AppState};

/// Same proven pattern as auth/courses/community's storage.rs. "slide-
/// image" was one of the Java FileService's IMAGE_FILE_TYPES (recompressed
/// to lossless WebP), so this mirrors that behavior exactly.
const MAX_IMAGE_PIXELS: u64 = 30_000_000;

/// `storage_path` doubles as the local-disk relative path and the S3
/// object key -- same identifier either way, just where it resolves to.
async fn write_bytes(state: &AppState, storage_path: &str, bytes: &[u8], content_type: &str) -> Result<(), AppError> {
    match state.config.storage_backend {
        StorageBackend::Local => {
            let full_path = PathBuf::from(&state.config.storage_local_path).join(storage_path);
            if let Some(parent) = full_path.parent() {
                tokio::fs::create_dir_all(parent).await.map_err(|e| AppError::Internal(e.into()))?;
            }
            tokio::fs::write(&full_path, bytes).await.map_err(|e| AppError::Internal(e.into()))?;
        }
        StorageBackend::S3 => {
            let client = state.s3_client.as_ref().expect("s3_client set when storage_backend is S3");
            s3_storage::put_object(client, &state.config.s3_bucket, storage_path, bytes.to_vec(), content_type)
                .await
                .map_err(AppError::Internal)?;
        }
    }
    Ok(())
}

async fn read_bytes(state: &AppState, storage_path: &str) -> Result<Vec<u8>, AppError> {
    match state.config.storage_backend {
        StorageBackend::Local => {
            let full_path = PathBuf::from(&state.config.storage_local_path).join(storage_path);
            tokio::fs::read(&full_path).await.map_err(|e| AppError::Internal(e.into()))
        }
        StorageBackend::S3 => {
            let client = state.s3_client.as_ref().expect("s3_client set when storage_backend is S3");
            s3_storage::get_object(client, &state.config.s3_bucket, storage_path)
                .await
                .map_err(AppError::Internal)
        }
    }
}

async fn delete_bytes(state: &AppState, storage_path: &str) {
    match state.config.storage_backend {
        StorageBackend::Local => {
            let full_path = PathBuf::from(&state.config.storage_local_path).join(storage_path);
            let _ = tokio::fs::remove_file(&full_path).await;
        }
        StorageBackend::S3 => {
            if let Some(client) = state.s3_client.as_ref() {
                let _ = s3_storage::delete_object(client, &state.config.s3_bucket, storage_path).await;
            }
        }
    }
}

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

    // Lossy, not lossless: this crate wraps real libwebp, unlike the
    // `image` crate's own WebP encoder (image-webp), which is lossless-
    // only. Lossless-recompressing a JPEG (already lossy) routinely
    // inflates the file several times over instead of shrinking it --
    // quality 80 keeps visual fidelity close to the source while actually
    // achieving the compression the format is chosen for.
    let webp_bytes = webp::Encoder::from_rgba(&rgba, width, height).encode(80.0).to_vec();

    let filename = format!("{}.webp", Uuid::new_v4());
    let storage_path = format!("slide-image/{owner_id}/{filename}");
    write_bytes(state, &storage_path, &webp_bytes, "image/webp").await?;

    let record = FileRecord {
        id: None,
        filename: filename.clone(),
        file_type: "slide-image".to_string(),
        file_size: webp_bytes.len() as i64,
        storage_path,
        content_type: "image/webp".to_string(),
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

    // Domain-prefixed path (not bare /files/view/{id}) so the production
    // gateway, which shares one public /api origin across every domain,
    // can route this to the right backend by path alone. See
    // routes.rs for the matching route registration.
    let base = state.config.api_base_url.trim_end_matches('/');
    Ok(UploadedImage {
        file_url: format!("{base}/slides-files/view/{file_id}"),
        file_id,
    })
}

/// Guided-narration audio uploads. Not one of the Java FileService's
/// IMAGE_FILE_TYPES, so stored as-is (no WebP conversion), matching the
/// same "raw" pattern used for lead-proofs in `site`.
pub async fn upload_narration_audio(
    state: &AppState,
    owner_id: &str,
    content_type: Option<&str>,
    original_filename: Option<&str>,
    bytes: Vec<u8>,
) -> Result<UploadedImage, AppError> {
    let safe_name = match original_filename {
        Some(name) if !name.trim().is_empty() => name.to_string(),
        _ => "narration.webm".to_string(),
    };
    let filename = format!("{}_{safe_name}", Uuid::new_v4());
    let storage_path = format!("slide-narration/{owner_id}/{filename}");
    let content_type_str = content_type.unwrap_or("application/octet-stream").to_string();
    write_bytes(state, &storage_path, &bytes, &content_type_str).await?;

    let record = FileRecord {
        id: None,
        filename: filename.clone(),
        file_type: "slide-narration".to_string(),
        file_size: bytes.len() as i64,
        storage_path,
        content_type: content_type_str,
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

    // Domain-prefixed path (not bare /files/view/{id}) so the production
    // gateway, which shares one public /api origin across every domain,
    // can route this to the right backend by path alone. See
    // routes.rs for the matching route registration.
    let base = state.config.api_base_url.trim_end_matches('/');
    Ok(UploadedImage {
        file_url: format!("{base}/slides-files/view/{file_id}"),
        file_id,
    })
}

pub struct StoredFile {
    pub bytes: Vec<u8>,
    pub content_type: String,
}

pub async fn read_file(state: &AppState, file_id: &str) -> Result<StoredFile, AppError> {
    let record = find_file(state, file_id).await?;
    let bytes = read_bytes(state, &record.storage_path).await?;
    Ok(StoredFile {
        content_type: record.content_type,
        bytes,
    })
}

pub async fn delete_file(state: &AppState, file_id: &str, requester_id: &str) -> Result<(), AppError> {
    let record = find_file(state, file_id).await?;
    if record.uploaded_by != requester_id {
        return Err(AppError::Forbidden("Cannot delete another user's file".to_string()));
    }
    delete_bytes(state, &record.storage_path).await;

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
