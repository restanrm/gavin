//! Admin API handlers (require authentication)

use axum::{
    extract::{Multipart, Path, Query, State},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Duration};
use tower_sessions::Session;

use crate::{
    album_metadata::{AlbumCandidate, CoverImageAnalysis},
    db::{BulkCreateRequest, CreateVinyl, MetadataUpdate, UpdateVinyl, Vinyl},
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
    let vinyl = state
        .metadata_client
        .enrich_vinyl(&state.pool, &vinyl.id)
        .await?;
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
    for (index, item) in request.items.into_iter().enumerate() {
        if index > 0 {
            // MusicBrainz asks clients to keep request rates modest. Bulk imports
            // enrich synchronously so admins immediately see rows needing choice.
            tokio::time::sleep(Duration::from_millis(1100)).await;
        }

        let vinyl = Vinyl::create(&state.pool, item).await?;
        let vinyl = state
            .metadata_client
            .enrich_vinyl(&state.pool, &vinyl.id)
            .await?;
        created.push(vinyl);
    }

    Ok(Json(created))
}

#[derive(Serialize)]
pub struct UploadResponse {
    pub url: String,
}

#[derive(Serialize)]
pub struct CoverImportResponse {
    pub status: String,
    pub detected_terms: Vec<String>,
    pub candidates: Vec<AlbumCandidate>,
    pub vinyl: Option<Vinyl>,
    pub error: Option<String>,
}

#[derive(Deserialize)]
pub struct CandidateImportRequest {
    pub candidate: AlbumCandidate,
}

#[derive(Deserialize)]
pub struct ArtistAlbumSearchQuery {
    pub artist: String,
}

/// Search albums by artist for manual selection/import.
pub async fn search_artist_albums(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<ArtistAlbumSearchQuery>,
) -> Result<Json<Vec<AlbumCandidate>>> {
    require_admin(&state, &session).await?;

    tracing::info!(artist = %query.artist, "admin artist album search requested");

    let candidates = state
        .metadata_client
        .search_artist_albums(&query.artist)
        .await
        .map_err(|err| AppError::InvalidInput(err.to_string()))?;

    tracing::info!(
        artist = %query.artist,
        results = candidates.len(),
        "admin artist album search completed"
    );

    Ok(Json(candidates))
}

/// Import a vinyl by visually recognizing an uploaded album-cover photo.
pub async fn import_cover_image(
    State(state): State<AppState>,
    session: Session,
    mut multipart: Multipart,
) -> Result<Json<CoverImportResponse>> {
    require_admin(&state, &session).await?;

    let field = multipart
        .next_field()
        .await
        .map_err(|e| AppError::InvalidInput(format!("Failed to read multipart: {}", e)))?
        .ok_or_else(|| AppError::InvalidInput("No file field found".to_string()))?;

    validate_image_field(&field)?;

    let data = field
        .bytes()
        .await
        .map_err(|e| AppError::InvalidInput(format!("Failed to read file data: {}", e)))?;
    validate_image_size(data.len())?;

    match state.metadata_client.analyze_cover_image(&data).await {
        Ok(analysis) => build_cover_import_response(&state, analysis).await,
        Err(err) => Ok(Json(CoverImportResponse {
            status: "error".to_string(),
            detected_terms: Vec::new(),
            candidates: Vec::new(),
            vinyl: None,
            error: Some(err.to_string()),
        })),
    }
}

/// Create a vinyl from a MusicBrainz candidate selected during cover import.
pub async fn import_cover_candidate(
    State(state): State<AppState>,
    session: Session,
    Json(request): Json<CandidateImportRequest>,
) -> Result<Json<Vinyl>> {
    require_admin(&state, &session).await?;
    create_vinyl_from_candidate(&state, request.candidate)
        .await
        .map(Json)
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

    validate_image_field(&field)?;

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
    validate_image_size(data.len())?;

    // Write file
    tokio::fs::write(&file_path, &data).await?;

    // Return URL
    let url = format!("/uploads/{}", unique_filename);
    Ok(Json(UploadResponse { url }))
}

async fn build_cover_import_response(
    _state: &AppState,
    analysis: CoverImageAnalysis,
) -> Result<Json<CoverImportResponse>> {
    // Visual album-cover recognition can be wrong even when the metadata lookup
    // returns a high-scoring candidate. Always ask the admin to confirm by
    // choosing the matching jacket before creating the vinyl.
    let status = if analysis.candidates.is_empty() {
        analysis.status
    } else {
        "needs_choice".to_string()
    };

    Ok(Json(CoverImportResponse {
        status,
        detected_terms: analysis.detected_terms,
        candidates: analysis.candidates,
        vinyl: None,
        error: None,
    }))
}

async fn create_vinyl_from_candidate(
    state: &AppState,
    candidate: AlbumCandidate,
) -> Result<Vinyl> {
    if candidate.artist.trim().is_empty() || candidate.title.trim().is_empty() {
        return Err(AppError::InvalidInput(
            "candidate artist and title are required".to_string(),
        ));
    }

    let notes = Some(format!("Metadata: {}", candidate.source_url));
    let vinyl = Vinyl::create(
        &state.pool,
        CreateVinyl {
            artist: candidate.artist.clone(),
            title: candidate.title.clone(),
            release_year: candidate.release_year,
            notes: notes.clone(),
            cover_image_url: candidate.cover_image_url.clone(),
        },
    )
    .await?;

    Vinyl::update_metadata(
        &state.pool,
        &vinyl.id,
        MetadataUpdate {
            release_year: candidate.release_year,
            notes,
            cover_image_url: candidate.cover_image_url,
            metadata_status: "complete".to_string(),
            metadata_source: Some(candidate.source),
            metadata_source_id: Some(candidate.id),
            metadata_source_url: Some(candidate.source_url),
            metadata_candidates: None,
            metadata_error: None,
            metadata_checked_at: Some(Utc::now()),
        },
    )
    .await?;

    Vinyl::get(&state.pool, &vinyl.id).await
}

fn validate_image_field(field: &axum::extract::multipart::Field<'_>) -> Result<()> {
    if let Some(content_type) = field.content_type() {
        if !content_type.starts_with("image/") {
            return Err(AppError::InvalidInput(
                "Please upload an image file".to_string(),
            ));
        }
    }

    Ok(())
}

fn validate_image_size(size: usize) -> Result<()> {
    const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024;
    if size > MAX_IMAGE_SIZE {
        return Err(AppError::InvalidInput(
            "Image file size must be less than 10MB".to_string(),
        ));
    }

    Ok(())
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
