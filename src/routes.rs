//! Application routes

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use sqlx::SqlitePool;
use tower_http::services::{ServeDir, ServeFile};
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::SqliteStore;

use crate::{album_metadata::AlbumMetadataClient, auth::AuthClient, config::Config, handlers};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub auth_client: AuthClient,
    pub upload_dir: String,
    pub metadata_client: AlbumMetadataClient,
}

/// Create the application router
pub async fn create_router(
    pool: SqlitePool,
    config: Config,
    auth_client: AuthClient,
    metadata_client: AlbumMetadataClient,
) -> anyhow::Result<Router> {
    // Set up session store
    let session_store = SqliteStore::new(pool.clone());
    session_store.migrate().await?;

    // Session secret - use a default for dev mode if not provided
    let _session_secret = config.session_secret.clone()
        .unwrap_or_else(|| "dev-secret-not-for-production-use-only".to_string());
    
    if config.session_secret.is_none() {
        tracing::warn!("SESSION_SECRET not set, using default (NOT SECURE FOR PRODUCTION)");
    }

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(config.cookie_secure)
        .with_expiry(Expiry::OnInactivity(time::Duration::hours(24)));

    // Create shared state
    let state = AppState {
        pool: pool.clone(),
        auth_client,
        upload_dir: config.upload_dir.clone(),
        metadata_client,
    };

    // API routes
    let api_routes = Router::new()
        .route("/health", get(handlers::health))
        .route("/vinyls", get(handlers::public::list_vinyls))
        .route("/auth/login", get(handlers::auth::login))
        .route("/auth/callback", get(handlers::auth::callback))
        .route("/auth/logout", post(handlers::auth::logout))
        .route("/auth/me", get(handlers::auth::me))
        .route("/admin/vinyls", post(handlers::admin::create_vinyl))
        .route("/admin/vinyls/:id", put(handlers::admin::update_vinyl))
        .route("/admin/vinyls/:id", delete(handlers::admin::delete_vinyl))
        .route(
            "/admin/vinyls/bulk",
            post(handlers::admin::bulk_create_vinyls),
        )
        .route("/admin/uploads", post(handlers::admin::upload_file))
        .with_state(state);

    // Main router
    let mut app = Router::new().nest("/api", api_routes);

    // Serve uploaded files
    app = app.nest_service("/uploads", ServeDir::new(&config.upload_dir));

    // Serve frontend if directory exists
    if std::path::Path::new(&config.frontend_dir).exists() {
        tracing::info!("Serving frontend from: {}", config.frontend_dir);
        let index_file = std::path::Path::new(&config.frontend_dir).join("index.html");
        let serve_dir = ServeDir::new(&config.frontend_dir).fallback(ServeFile::new(index_file));
        app = app.fallback_service(serve_dir);
    } else {
        tracing::warn!(
            "Frontend directory not found: {}. SPA fallback disabled.",
            config.frontend_dir
        );
    }

    // Add session layer
    app = app.layer(session_layer);

    Ok(app)
}
