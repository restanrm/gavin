//! Authentication (OIDC and development mode)

use crate::{
    config::{AuthMode, Config},
    error::AppError,
};
use openidconnect::{
    core::{CoreClient, CoreProviderMetadata, CoreResponseType},
    reqwest::async_http_client,
    AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce,
    RedirectUrl, Scope, TokenResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_sessions::Session;

const SESSION_USER_KEY: &str = "user";
const SESSION_AUTH_MODE_KEY: &str = "auth_mode";
const OIDC_AUTH_MODE: &str = "oidc";
const DEV_AUTHENTICATED_KEY: &str = "dev_authenticated";
const SESSION_STATE_KEY: &str = "oauth_state";
const SESSION_NONCE_KEY: &str = "oauth_nonce";

/// Authentication client - supports OIDC and dev mode
#[derive(Clone)]
pub enum AuthClient {
    /// OIDC authentication (production)
    Oidc(OidcClientInner),
    /// Development mode - no OIDC required; login enables a local dev-admin session
    Dev,
}

impl AuthClient {
    /// Create a new authentication client based on config
    pub async fn new(config: &Config) -> Result<Self, AppError> {
        match config.auth_mode {
            AuthMode::Oidc => {
                let oidc = OidcClientInner::new(config).await?;
                Ok(Self::Oidc(oidc))
            }
            AuthMode::Dev => {
                tracing::warn!(
                    "Running in DEV auth mode - clicking login enables dev-admin for that browser session"
                );
                Ok(Self::Dev)
            }
        }
    }

    /// Generate authorization URL. In dev mode, mark this browser session as logged in.
    pub async fn authorize_url(&self, session: &Session) -> Result<String, AppError> {
        match self {
            Self::Oidc(client) => client.authorize_url(session).await,
            Self::Dev => {
                set_dev_authenticated(session).await?;
                Ok("/".to_string())
            }
        }
    }

    /// Handle callback (only used by OIDC; dev mode also enables dev-admin for convenience)
    pub async fn handle_callback(
        &self,
        session: &Session,
        code: String,
        state: String,
    ) -> Result<User, AppError> {
        match self {
            Self::Oidc(client) => client.handle_callback(session, code, state).await,
            Self::Dev => {
                set_dev_authenticated(session).await?;
                Ok(dev_user())
            }
        }
    }

    /// Get the authenticated user, if any.
    pub async fn current_user(&self, session: &Session) -> Result<Option<User>, AppError> {
        match self {
            Self::Dev => {
                let authenticated: Option<bool> = session
                    .get(DEV_AUTHENTICATED_KEY)
                    .await
                    .map_err(|e| {
                        AppError::Internal(anyhow::anyhow!(
                            "Failed to get dev auth flag from session: {}",
                            e
                        ))
                    })?;

                Ok(authenticated.unwrap_or(false).then(dev_user))
            }
            Self::Oidc(_) => {
                let auth_mode: Option<String> = session
                    .get(SESSION_AUTH_MODE_KEY)
                    .await
                    .map_err(|e| {
                        AppError::Internal(anyhow::anyhow!(
                            "Failed to get auth mode from session: {}",
                            e
                        ))
                    })?;

                if auth_mode.as_deref() != Some(OIDC_AUTH_MODE) {
                    return Ok(None);
                }

                session.get(SESSION_USER_KEY).await.map_err(|e| {
                    AppError::Internal(anyhow::anyhow!("Failed to get user from session: {}", e))
                })
            }
        }
    }

    /// Require an authenticated user.
    pub async fn require_user(&self, session: &Session) -> Result<User, AppError> {
        self.current_user(session).await?.ok_or(AppError::Unauthorized)
    }

    /// Logout by clearing the active authentication state.
    pub async fn logout(&self, session: &Session) -> Result<(), AppError> {
        match self {
            Self::Dev => {
                session
                    .remove::<bool>(DEV_AUTHENTICATED_KEY)
                    .await
                    .map_err(|e| {
                        AppError::Internal(anyhow::anyhow!(
                            "Failed to remove dev auth flag: {}",
                            e
                        ))
                    })?;
                Ok(())
            }
            Self::Oidc(_) => session.flush().await.map_err(|e| {
                AppError::Internal(anyhow::anyhow!("Failed to flush session: {}", e))
            }),
        }
    }
}

/// OIDC client wrapper
#[derive(Clone)]
pub struct OidcClientInner {
    client: Arc<CoreClient>,
}

/// Authenticated user information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub subject: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

impl OidcClientInner {
    /// Create a new OIDC client
    pub async fn new(config: &Config) -> Result<Self, AppError> {
        let issuer_url = config
            .oidc_issuer_url
            .as_ref()
            .ok_or_else(|| AppError::Oidc("OIDC_ISSUER_URL not set".to_string()))?;
        let client_id = config
            .oidc_client_id
            .as_ref()
            .ok_or_else(|| AppError::Oidc("OIDC_CLIENT_ID not set".to_string()))?;
        let client_secret = config
            .oidc_client_secret
            .as_ref()
            .ok_or_else(|| AppError::Oidc("OIDC_CLIENT_SECRET not set".to_string()))?;
        let redirect_url = config
            .oidc_redirect_url
            .as_ref()
            .ok_or_else(|| AppError::Oidc("OIDC_REDIRECT_URL not set".to_string()))?;

        let issuer = IssuerUrl::new(issuer_url.clone())
            .map_err(|e| AppError::Oidc(format!("Invalid issuer URL: {}", e)))?;

        let provider_metadata = CoreProviderMetadata::discover_async(issuer, async_http_client)
            .await
            .map_err(|e| AppError::Oidc(format!("Failed to discover provider metadata: {}", e)))?;

        let client = CoreClient::from_provider_metadata(
            provider_metadata,
            ClientId::new(client_id.clone()),
            Some(ClientSecret::new(client_secret.clone())),
        )
        .set_redirect_uri(
            RedirectUrl::new(redirect_url.clone())
                .map_err(|e| AppError::Oidc(format!("Invalid redirect URL: {}", e)))?,
        );

        Ok(Self {
            client: Arc::new(client),
        })
    }

    /// Generate authorization URL
    pub async fn authorize_url(&self, session: &Session) -> Result<String, AppError> {
        let (auth_url, csrf_token, nonce) = self
            .client
            .authorize_url(
                AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .url();

        // Store state and nonce in session
        session
            .insert(SESSION_STATE_KEY, csrf_token.secret().clone())
            .await
            .map_err(|e| AppError::Oidc(format!("Failed to store state: {}", e)))?;

        session
            .insert(SESSION_NONCE_KEY, nonce.secret().clone())
            .await
            .map_err(|e| AppError::Oidc(format!("Failed to store nonce: {}", e)))?;

        Ok(auth_url.to_string())
    }

    /// Handle callback and exchange code for token
    pub async fn handle_callback(
        &self,
        session: &Session,
        code: String,
        state: String,
    ) -> Result<User, AppError> {
        // Verify state
        let expected_state: Option<String> = session
            .get(SESSION_STATE_KEY)
            .await
            .map_err(|e| AppError::Oidc(format!("Failed to get state: {}", e)))?;

        let expected_state =
            expected_state.ok_or_else(|| AppError::Oidc("No state found in session".to_string()))?;

        if state != expected_state {
            return Err(AppError::Oidc("State mismatch".to_string()));
        }

        // Get nonce
        let expected_nonce: Option<String> = session
            .get(SESSION_NONCE_KEY)
            .await
            .map_err(|e| AppError::Oidc(format!("Failed to get nonce: {}", e)))?;

        let expected_nonce =
            expected_nonce.ok_or_else(|| AppError::Oidc("No nonce found in session".to_string()))?;

        // Exchange code for token
        let token_response = self
            .client
            .exchange_code(AuthorizationCode::new(code))
            .request_async(async_http_client)
            .await
            .map_err(|e| AppError::Oidc(format!("Failed to exchange code: {}", e)))?;

        // Verify ID token
        let id_token = token_response
            .id_token()
            .ok_or_else(|| AppError::Oidc("No ID token in response".to_string()))?;

        let claims = id_token
            .claims(&self.client.id_token_verifier(), &Nonce::new(expected_nonce))
            .map_err(|e| AppError::Oidc(format!("Failed to verify ID token: {}", e)))?;

        // Extract user info
        let user = User {
            subject: claims.subject().to_string(),
            email: claims.email().map(|e| e.to_string()),
            name: claims
                .name()
                .and_then(|n| n.get(None))
                .map(|n| n.to_string()),
        };

        // Store user in session and mark it as an OIDC-authenticated session.
        session
            .insert(SESSION_USER_KEY, user.clone())
            .await
            .map_err(|e| AppError::Oidc(format!("Failed to store user: {}", e)))?;

        session
            .insert(SESSION_AUTH_MODE_KEY, OIDC_AUTH_MODE)
            .await
            .map_err(|e| AppError::Oidc(format!("Failed to store auth mode: {}", e)))?;

        // Clean up state and nonce
        session.remove::<String>(SESSION_STATE_KEY).await.ok();
        session.remove::<String>(SESSION_NONCE_KEY).await.ok();

        Ok(user)
    }
}

async fn set_dev_authenticated(session: &Session) -> Result<(), AppError> {
    session
        .insert(DEV_AUTHENTICATED_KEY, true)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to store dev auth flag: {}", e)))
}

/// Create a dev mode user
fn dev_user() -> User {
    User {
        subject: "dev-admin".to_string(),
        email: Some("dev@localhost".to_string()),
        name: Some("Development Admin".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dev_user() {
        let user = dev_user();
        assert_eq!(user.subject, "dev-admin");
        assert_eq!(user.email, Some("dev@localhost".to_string()));
        assert_eq!(user.name, Some("Development Admin".to_string()));
    }

    #[tokio::test]
    async fn test_dev_auth_requires_login() {
        use std::sync::Arc;
        use tower_sessions::{MemoryStore, Session};

        let auth_client = AuthClient::Dev;
        let session = Session::new(None, Arc::new(MemoryStore::default()), None);

        assert!(auth_client.current_user(&session).await.unwrap().is_none());

        auth_client.authorize_url(&session).await.unwrap();
        let user = auth_client.current_user(&session).await.unwrap().unwrap();
        assert_eq!(user.subject, "dev-admin");

        auth_client.logout(&session).await.unwrap();
        assert!(auth_client.current_user(&session).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_oidc_ignores_dev_login_marker() {
        use openidconnect::{core::CoreJsonWebKeySet, AuthUrl, TokenUrl};
        use std::sync::Arc;
        use tower_sessions::{MemoryStore, Session};

        let oidc_client = CoreClient::new(
            ClientId::new("client".to_string()),
            None,
            IssuerUrl::new("https://issuer.example".to_string()).unwrap(),
            AuthUrl::new("https://issuer.example/auth".to_string()).unwrap(),
            Some(TokenUrl::new("https://issuer.example/token".to_string()).unwrap()),
            None,
            CoreJsonWebKeySet::new(vec![]),
        );
        let auth_client = AuthClient::Oidc(OidcClientInner {
            client: Arc::new(oidc_client),
        });
        let session = Session::new(None, Arc::new(MemoryStore::default()), None);

        session.insert(DEV_AUTHENTICATED_KEY, true).await.unwrap();
        session.insert(SESSION_USER_KEY, dev_user()).await.unwrap();

        assert!(auth_client.current_user(&session).await.unwrap().is_none());
    }
}
