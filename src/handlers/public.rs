//! Public API handlers

use axum::{
    extract::{Query, State},
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
