//! Gavin - Vinyl Library Catalog Backend
//!
//! A REST API backend for managing a vinyl record collection with OIDC authentication.

mod auth;
mod config;
mod db;
mod error;
mod handlers;
mod routes;

use anyhow::Context;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::{net::SocketAddr, path::PathBuf, str::FromStr};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables
    let _ = dotenvy::dotenv();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "gavin=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = config::Config::from_env()?;
    tracing::info!("Configuration loaded (auth mode: {:?})", config.auth_mode);

    // Set up database
    if let Some(database_path) = sqlite_database_path(&config.database_url) {
        if let Some(parent) = database_path.parent().filter(|path| !path.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
    }

    let connect_options = SqliteConnectOptions::from_str(&config.database_url)?
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await
        .context("Failed to connect to database")?;

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("Failed to run database migrations")?;
    tracing::info!("Database migrations complete");

    // Create upload directory
    std::fs::create_dir_all(&config.upload_dir)?;
    tracing::info!("Upload directory ready: {}", config.upload_dir);

    // Initialize authentication client
    let auth_client = auth::AuthClient::new(&config).await?;
    tracing::info!("Authentication client initialized");

    // Build application
    let app = routes::create_router(pool, config.clone(), auth_client).await?;

    // Apply middleware
    let app = app.layer(TraceLayer::new_for_http());

    // Start server
    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .context("Invalid HOST/PORT configuration")?;
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn sqlite_database_path(database_url: &str) -> Option<PathBuf> {
    let path = database_url
        .strip_prefix("sqlite://")
        .or_else(|| database_url.strip_prefix("sqlite:"))?;

    if path == ":memory:" || path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}
