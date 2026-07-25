//! Authentication handlers

use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower_sessions::Session;

use crate::{error::Result, routes::AppState};

/// Login - redirects to OIDC provider or home in dev mode
pub async fn login(
    State(state): State<AppState>,
    session: Session,
) -> Result<impl IntoResponse> {
    let url = state.auth_client.authorize_url(&session).await?;
    Ok(Redirect::to(&url))
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: String,
    state: String,
}

/// OIDC callback - validates token and creates session (or succeeds immediately in dev mode)
pub async fn callback(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<CallbackQuery>,
) -> Result<impl IntoResponse> {
    state.auth_client
        .handle_callback(&session, query.code, query.state)
        .await?;

    // Redirect to frontend
    Ok(Redirect::to("/"))
}

/// Logout - clears session
pub async fn logout(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<serde_json::Value>> {
    state.auth_client.logout(&session).await?;
    Ok(Json(json!({ "success": true })))
}

#[derive(Serialize)]
pub struct MeResponse {
    pub authenticated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Get current user info
pub async fn me(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<MeResponse>> {
    match state.auth_client.current_user(&session).await? {
        Some(user) => Ok(Json(MeResponse {
            authenticated: true,
            subject: Some(user.subject),
            email: user.email,
            name: user.name,
        })),
        None => Ok(Json(MeResponse {
            authenticated: false,
            subject: None,
            email: None,
            name: None,
        })),
    }
}
