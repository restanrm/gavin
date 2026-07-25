//! API endpoint handlers

pub mod admin;
pub mod auth;
pub mod public;

use axum::response::Json;
use serde_json::{json, Value};

/// Health check handler
pub async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
