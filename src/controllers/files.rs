//! File upload and retrieval API (ticket P1-12).
//! Ensures files are securely validated, stored outside the web root, and guarded by permissions.
use axum::{
    extract::{Path as AxumPath, State},
    http::{header, StatusCode},
    response::IntoResponse,
};
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    authz::{AuthUser, Role},
    storage::{StorageService, MAX_FILE_SIZE_BYTES},
};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FileMetadata {
    pub id: String,
    pub filename: String,
    pub mime_type: String,
    pub size_bytes: usize,
    pub owner_id: i64,
    pub storage_key: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UploadPayload {
    pub filename: String,
    pub mime_type: String,
    /// Base64 encoded file content
    pub content: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UploadResponse {
    pub id: String,
    pub filename: String,
    pub mime_type: String,
    pub size_bytes: usize,
    pub created_at: String,
}

// Global thread-safe registry for uploaded files metadata
static FILE_REGISTRY: LazyLock<RwLock<HashMap<String, FileMetadata>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

fn get_storage() -> StorageService {
    StorageService::new("./storage/uploads")
}

fn decode_base64(input: &str) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0;
    for &b in input.as_bytes() {
        if b.is_ascii_whitespace() {
            continue;
        }
        if b == b'=' {
            break;
        }
        let val = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return Err(Error::string("invalid base64 character in file content")),
        };
        buf = (buf << 6) | (val as u32);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Ok(out)
}

/// Uploads a file payload (PDF, image, text) securely.
#[utoipa::path(
    post,
    path = "/api/files",
    security(("bearer" = [])),
    request_body = UploadPayload,
    responses(
        (status = 200, body = UploadResponse),
        (status = 400, description = "Invalid payload, size exceeded, or disguised executable"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    )
)]
#[debug_handler]
pub async fn upload(
    auth: AuthUser,
    State(_ctx): State<AppContext>,
    Json(payload): Json<UploadPayload>,
) -> Result<Response> {
    auth.require("files:upload")?;

    let bytes = decode_base64(&payload.content)?;

    if bytes.len() > MAX_FILE_SIZE_BYTES {
        return Err(Error::string("file exceeds 10 MB limit"));
    }

    let storage = get_storage();
    let stored = storage
        .store(&payload.filename, &payload.mime_type, &bytes)
        .await?;

    let file_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let meta = FileMetadata {
        id: file_id.clone(),
        filename: payload.filename.clone(),
        mime_type: payload.mime_type.clone(),
        size_bytes: stored.size_bytes,
        owner_id: auth.user.id,
        storage_key: stored.storage_key,
        created_at: now.clone(),
    };

    if let Ok(mut lock) = FILE_REGISTRY.write() {
        lock.insert(file_id.clone(), meta);
    }

    let response = UploadResponse {
        id: file_id,
        filename: payload.filename,
        mime_type: payload.mime_type,
        size_bytes: stored.size_bytes,
        created_at: now,
    };

    format::json(response)
}

/// Streams a stored file to authorized clients.
#[utoipa::path(
    get,
    path = "/api/files/{id}",
    security(("bearer" = [])),
    params(("id" = String, Path, description = "File ID")),
    responses(
        (status = 200, description = "File binary stream"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "File not found or not owned by user"),
        (status = 403, description = "Forbidden")
    )
)]
#[debug_handler]
pub async fn get_file(
    auth: AuthUser,
    State(_ctx): State<AppContext>,
    AxumPath(id): AxumPath<String>,
) -> Result<Response> {
    auth.require("files:read")?;

    let meta = {
        let lock = FILE_REGISTRY
            .read()
            .map_err(|_| Error::InternalServerError)?;
        lock.get(&id).cloned()
    };

    let Some(meta) = meta else {
        return Err(Error::NotFound);
    };

    // Owner check: Non-HR users can only download their own files; other user's file returns 404
    if auth.role != Role::HrAdmin && meta.owner_id != auth.user.id {
        return Err(Error::NotFound);
    }

    let storage = get_storage();
    let file_path = storage.resolve_path(&meta.storage_key)?;

    let bytes = tokio::fs::read(&file_path)
        .await
        .map_err(|_| Error::NotFound)?;

    let mut headers = axum::http::HeaderMap::new();
    if let Ok(val) = meta.mime_type.parse() {
        headers.insert(header::CONTENT_TYPE, val);
    }
    let cd = format!("attachment; filename=\"{}\"", meta.filename);
    if let Ok(val) = cd.parse() {
        headers.insert(header::CONTENT_DISPOSITION, val);
    }
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        header::HeaderValue::from_static("nosniff"),
    );

    Ok((StatusCode::OK, headers, bytes).into_response())
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/files")
        .add("/", post(upload))
        .add("/{id}", get(get_file))
}
