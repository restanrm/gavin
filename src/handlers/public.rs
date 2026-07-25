//! Public API handlers

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::{db::Vinyl, error::Result, routes::AppState};

#[derive(Deserialize)]
pub struct SearchQuery {
    pub search: Option<String>,
}

/// List vinyls with optional search
pub async fn list_vinyls(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<Vinyl>>> {
    let vinyls = Vinyl::list(&state.pool, query.search).await?;
    Ok(Json(vinyls))
}

/// Return detailed album information for one vinyl.
pub async fn get_vinyl_details(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<crate::album_metadata::AlbumDetails>> {
    let vinyl = Vinyl::get(&state.pool, &id).await?;
    let details = state.metadata_client.album_details(&vinyl).await;
    Ok(Json(details))
}
