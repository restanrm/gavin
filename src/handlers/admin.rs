//! Admin API handlers (require authentication)

use axum::{
    extract::{Multipart, Path, Query, State},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, path::{Path as FsPath, PathBuf}, time::Duration};
use tower_sessions::Session;

use crate::{
    album_metadata::{AlbumCandidate, CoverImageAnalysis},
    db::{BulkCreateRequest, CreateVinyl, MetadataUpdate, PatchField, UpdateVinyl, Vinyl},
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
    Json(mut input): Json<CreateVinyl>,
) -> Result<Json<Vinyl>> {
    require_admin(&state, &session).await?;
    localize_create_cover(&state, &mut input).await;
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
    let input = localize_update_cover(&state, input).await;
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

        let mut item = item;
        localize_create_cover(&state, &mut item).await;
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

#[derive(Serialize)]
pub struct MetadataRefreshResponse {
    pub checked: usize,
}

#[derive(Serialize)]
pub struct OrphanedImageCleanupResponse {
    pub deleted: usize,
    pub kept: usize,
    pub errors: Vec<String>,
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

/// Apply a reviewed MusicBrainz candidate to an existing vinyl.
pub async fn select_vinyl_metadata_candidate(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<String>,
    Json(request): Json<CandidateImportRequest>,
) -> Result<Json<Vinyl>> {
    require_admin(&state, &session).await?;
    apply_candidate_to_vinyl(&state, &id, request.candidate)
        .await
        .map(Json)
}

/// Retry album metadata enrichment for an existing vinyl.
pub async fn refresh_vinyl_metadata(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<String>,
) -> Result<Json<Vinyl>> {
    require_admin(&state, &session).await?;
    state
        .metadata_client
        .enrich_vinyl(&state.pool, &id)
        .await
        .map(Json)
}

/// Retry metadata enrichment for all vinyls currently missing metadata.
pub async fn refresh_missing_metadata(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<MetadataRefreshResponse>> {
    require_admin(&state, &session).await?;

    const ADMIN_METADATA_REFRESH_LIMIT: i64 = 100;
    let ids = Vinyl::list_requiring_metadata(&state.pool, ADMIN_METADATA_REFRESH_LIMIT).await?;
    let mut checked = 0;

    for (index, id) in ids.into_iter().enumerate() {
        if index > 0 {
            tokio::time::sleep(Duration::from_millis(1100)).await;
        }

        state.metadata_client.enrich_vinyl(&state.pool, &id).await?;
        checked += 1;
    }

    Ok(Json(MetadataRefreshResponse { checked }))
}

/// Delete uploaded or cached images that are no longer referenced by any vinyl.
pub async fn cleanup_orphaned_images(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<OrphanedImageCleanupResponse>> {
    require_admin(&state, &session).await?;

    cleanup_orphaned_upload_images(&state.pool, &state.upload_dir)
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

async fn localize_create_cover(state: &AppState, input: &mut CreateVinyl) {
    if let Some(cover_url) = input.cover_image_url.as_deref() {
        input.cover_image_url = state.metadata_client.local_cover_url(None, cover_url).await;
    }
}

async fn localize_update_cover(state: &AppState, input: UpdateVinyl) -> UpdateVinyl {
    let UpdateVinyl {
        artist,
        title,
        release_year,
        genre,
        notes,
        cover_image_url,
    } = input;

    let cover_image_url = match cover_image_url {
        PatchField::Value(value) => match state.metadata_client.local_cover_url(None, &value).await {
            Some(local_url) => PatchField::Value(local_url),
            None => PatchField::Value(value),
        },
        other => other,
    };

    UpdateVinyl {
        artist,
        title,
        release_year,
        genre,
        notes,
        cover_image_url,
    }
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
    validate_candidate_identity(&candidate)?;

    let notes = Some(format!("Metadata: {}", candidate.source_url));
    let cover_image_url = local_candidate_cover_url(state, &candidate).await;
    let vinyl = Vinyl::create(
        &state.pool,
        CreateVinyl {
            artist: candidate.artist.clone(),
            title: candidate.title.clone(),
            release_year: candidate.release_year,
            genre: candidate.genre.clone(),
            notes: notes.clone(),
            cover_image_url: cover_image_url.clone(),
        },
    )
    .await?;

    Vinyl::update_metadata(
        &state.pool,
        &vinyl.id,
        MetadataUpdate {
            release_year: candidate.release_year,
            genre: candidate.genre,
            notes,
            cover_image_url,
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

async fn apply_candidate_to_vinyl(
    state: &AppState,
    id: &str,
    candidate: AlbumCandidate,
) -> Result<Vinyl> {
    validate_candidate_identity(&candidate)?;

    let existing = Vinyl::get(&state.pool, id).await?;
    let cover_image_url = local_candidate_cover_url(state, &candidate).await;
    let metadata_note = format!("Metadata: {}", candidate.source_url);
    let notes_update = if existing
        .notes
        .as_deref()
        .map(|notes| notes.trim().is_empty() || is_generated_metadata_note(notes))
        .unwrap_or(true)
    {
        Some(metadata_note.clone())
    } else {
        None
    };

    Vinyl::update(
        &state.pool,
        id,
        UpdateVinyl {
            artist: PatchField::Value(candidate.artist.clone()),
            title: PatchField::Value(candidate.title.clone()),
            release_year: candidate
                .release_year
                .map_or(PatchField::Missing, PatchField::Value),
            genre: candidate
                .genre
                .clone()
                .map_or(PatchField::Missing, PatchField::Value),
            notes: notes_update
                .clone()
                .map_or(PatchField::Missing, PatchField::Value),
            cover_image_url: cover_image_url
                .clone()
                .map_or(PatchField::Missing, PatchField::Value),
        },
    )
    .await?;

    Vinyl::update_metadata(
        &state.pool,
        id,
        MetadataUpdate {
            release_year: candidate.release_year,
            genre: candidate.genre,
            notes: notes_update,
            cover_image_url,
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

    Vinyl::get(&state.pool, id).await
}

async fn local_candidate_cover_url(state: &AppState, candidate: &AlbumCandidate) -> Option<String> {
    match candidate.cover_image_url.as_deref() {
        Some(url) => state
            .metadata_client
            .local_cover_url(Some(&candidate.id), url)
            .await,
        None => None,
    }
}

async fn cleanup_orphaned_upload_images(
    pool: &sqlx::SqlitePool,
    upload_dir: &str,
) -> Result<OrphanedImageCleanupResponse> {
    let referenced = referenced_upload_paths(&Vinyl::list_cover_image_urls(pool).await?);
    let upload_dir = PathBuf::from(upload_dir);
    let mut files = Vec::<PathBuf>::new();
    collect_upload_image_files(&upload_dir, &mut files)?;

    let mut deleted = 0;
    let mut kept = 0;
    let mut errors = Vec::<String>::new();

    for file in files {
        let relative = match file.strip_prefix(&upload_dir) {
            Ok(path) => path.to_string_lossy().replace('\\', "/"),
            Err(err) => {
                errors.push(format!("{}: {}", file.display(), err));
                continue;
            }
        };

        if referenced.contains(&relative) {
            kept += 1;
            continue;
        }

        match fs::remove_file(&file) {
            Ok(()) => deleted += 1,
            Err(err) => errors.push(format!("{}: {}", file.display(), err)),
        }
    }

    Ok(OrphanedImageCleanupResponse {
        deleted,
        kept,
        errors,
    })
}

fn referenced_upload_paths(urls: &[String]) -> HashSet<String> {
    urls.iter()
        .filter_map(|url| url.trim().strip_prefix("/uploads/"))
        .map(|path| path.trim_start_matches('/').to_string())
        .filter(|path| !path.is_empty() && !path.contains(".."))
        .collect()
}

fn collect_upload_image_files(dir: &FsPath, files: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_upload_image_files(&path, files)?;
        } else if file_type.is_file() && is_cleanup_candidate_image(&path) {
            files.push(path);
        }
    }

    Ok(())
}

fn is_cleanup_candidate_image(path: &FsPath) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "jpg" | "jpeg" | "png" | "webp" | "gif"
            )
        })
        .unwrap_or(false)
}

fn validate_candidate_identity(candidate: &AlbumCandidate) -> Result<()> {
    if candidate.artist.trim().is_empty() || candidate.title.trim().is_empty() {
        return Err(AppError::InvalidInput(
            "candidate artist and title are required".to_string(),
        ));
    }

    Ok(())
}

fn is_generated_metadata_note(notes: &str) -> bool {
    notes.trim().to_lowercase().starts_with("metadata:")
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
