//! Admin API handlers (require authentication)

use axum::{
    extract::{Multipart, Path, State},
    Json,
};
use serde::Serialize;
use std::path::PathBuf;
use tower_sessions::Session;

use crate::{
    db::{BulkCreateRequest, CreateVinyl, UpdateVinyl, Vinyl},
    error::{AppError, Result},
    routes::AppState,
};

/// Require authentication for all admin handlers
async fn require_admin(state: &AppState, session: &Session) -> Result<()> {
    state.auth_client.require_user(session).await?;
    Ok(())
}

/// Create a new vinyl
pub async fn create_vinyl(
    State(state): State<AppState>,
    session: Session,
    Json(input): Json<CreateVinyl>,
) -> Result<Json<Vinyl>> {
    require_admin(&state, &session).await?;
    let vinyl = Vinyl::create(&state.pool, input).await?;
    Ok(Json(vinyl))
}

/// Update a vinyl
pub async fn update_vinyl(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<String>,
    Json(input): Json<UpdateVinyl>,
) -> Result<Json<Vinyl>> {
    require_admin(&state, &session).await?;
    let vinyl = Vinyl::update(&state.pool, &id, input).await?;
    Ok(Json(vinyl))
}

/// Delete a vinyl
pub async fn delete_vinyl(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    require_admin(&state, &session).await?;
    Vinyl::delete(&state.pool, &id).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

/// Bulk create vinyls
pub async fn bulk_create_vinyls(
    State(state): State<AppState>,
    session: Session,
    Json(request): Json<BulkCreateRequest>,
) -> Result<Json<Vec<Vinyl>>> {
    require_admin(&state, &session).await?;

    let mut created = Vec::new();
    for item in request.items {
        let vinyl = Vinyl::create(&state.pool, item).await?;
        created.push(vinyl);
    }

    Ok(Json(created))
}

#[derive(Serialize)]
pub struct UploadResponse {
    pub url: String,
}

/// Upload a file
pub async fn upload_file(
    State(state): State<AppState>,
    session: Session,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>> {
    require_admin(&state, &session).await?;

    // Get the file field
    let field = multipart
        .next_field()
        .await
        .map_err(|e| AppError::InvalidInput(format!("Failed to read multipart: {}", e)))?
        .ok_or_else(|| AppError::InvalidInput("No file field found".to_string()))?;

    let filename = field
        .file_name()
        .ok_or_else(|| AppError::InvalidInput("No filename provided".to_string()))?
        .to_string();

    // Sanitize filename
    let safe_filename = sanitize_filename(&filename);
    if safe_filename.is_empty() {
        return Err(AppError::InvalidInput("Invalid filename".to_string()));
    }

    // Generate unique filename to avoid collisions
    let timestamp = chrono::Utc::now().timestamp();
    let unique_filename = format!("{}_{}", timestamp, safe_filename);

    let file_path = PathBuf::from(&state.upload_dir).join(&unique_filename);

    // Read file data
    let data = field
        .bytes()
        .await
        .map_err(|e| AppError::InvalidInput(format!("Failed to read file data: {}", e)))?;

    // Write file
    tokio::fs::write(&file_path, &data).await?;

    // Return URL
    let url = format!("/uploads/{}", unique_filename);
    Ok(Json(UploadResponse { url }))
}

/// Sanitize filename to prevent directory traversal
fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
        .filter(|c| *c != '/')  // Extra safety
        .collect::<String>()
        .trim_matches('.')  // Remove leading/trailing dots
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("test.jpg"), "test.jpg");
        assert_eq!(sanitize_filename("my-file_123.png"), "my-file_123.png");
        assert_eq!(sanitize_filename("../../../etc/passwd"), "etcpasswd");
        assert_eq!(sanitize_filename("file name.txt"), "filename.txt");
        assert_eq!(sanitize_filename("malicious<>:\"|?*.exe"), "malicious.exe");
    }
}
