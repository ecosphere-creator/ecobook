use bson::oid::ObjectId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Local-disk file record for slide images (element images, covers,
/// gallery). Interim storage until MinIO/S3 is provisioned for the
/// estate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub filename: String,
    #[serde(rename = "fileType")]
    pub file_type: String,
    #[serde(rename = "fileSize")]
    pub file_size: i64,
    #[serde(rename = "storagePath")]
    pub storage_path: String,
    /// Serving content-type. Images uploaded via `upload_slide_image` are
    /// always "image/webp" after conversion; raw uploads (narration audio)
    /// keep whatever the client sent.
    #[serde(rename = "contentType", default = "default_content_type")]
    pub content_type: String,
    #[serde(rename = "uploadedBy")]
    pub uploaded_by: String,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
}

fn default_content_type() -> String {
    "application/octet-stream".to_string()
}
