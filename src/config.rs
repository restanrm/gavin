//! Application configuration

use std::env;

/// Authentication mode
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthMode {
    /// OIDC authentication (production)
    Oidc,
    /// Development mode - OIDC is disabled and login enables a local dev-admin session
    Dev,
}

impl AuthMode {
    /// Parse from string (case-insensitive)
    pub fn from_str(s: &str) -> anyhow::Result<Self> {
        match s.to_lowercase().as_str() {
            "oidc" => Ok(Self::Oidc),
            "dev" => Ok(Self::Dev),
            _ => anyhow::bail!("Invalid AUTH_MODE: {}. Must be 'oidc' or 'dev'", s),
        }
    }
}

impl Default for AuthMode {
    fn default() -> Self {
        Self::Oidc
    }
}

/// Application configuration loaded from environment variables
#[derive(Clone, Debug)]
#[allow(dead_code)] // Fields used via env vars
pub struct Config {
    pub database_url: String,
    pub upload_dir: String,
    pub frontend_dir: String,
    pub host: String,
    pub port: u16,
    pub public_domain: String,
    pub auth_mode: AuthMode,
    pub oidc_issuer_url: Option<String>,
    pub oidc_client_id: Option<String>,
    pub oidc_client_secret: Option<String>,
    pub oidc_redirect_url: Option<String>,
    pub session_secret: Option<String>,
    pub cookie_secure: bool,
    pub album_metadata_enabled: bool,
    pub album_metadata_user_agent: Option<String>,
    pub album_cover_recognition_provider: String,
    pub openai_api_key: Option<String>,
    pub openai_base_url: String,
    pub openai_model: String,
    pub gemini_api_key: Option<String>,
    pub gemini_base_url: String,
    pub gemini_model: String,
    pub musicbrainz_base_url: String,
    pub cover_art_archive_base_url: String,
}

fn optional_env(key: &str) -> Option<String> {
    env::var(key).ok().filter(|value| !value.trim().is_empty())
}

fn album_cover_recognition_provider() -> String {
    env::var("ALBUM_COVER_RECOGNITION_PROVIDER")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            if optional_env("GEMINI_API_KEY").is_some() {
                "gemini".to_string()
            } else {
                "openai".to_string()
            }
        })
        .to_lowercase()
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> anyhow::Result<Self> {
        let auth_mode = env::var("AUTH_MODE")
            .ok()
            .map(|s| AuthMode::from_str(&s))
            .transpose()?
            .unwrap_or_default();

        let public_domain =
            env::var("PUBLIC_DOMAIN").unwrap_or_else(|_| "gavin.restanrm.fr".to_string());

        // In OIDC mode, require OIDC provider env vars.
        // OIDC_REDIRECT_URL defaults from PUBLIC_DOMAIN for convenient test/prod deploys.
        // In Dev mode, OIDC env vars are optional.
        let (
            oidc_issuer_url,
            oidc_client_id,
            oidc_client_secret,
            oidc_redirect_url,
            session_secret,
        ) = if auth_mode == AuthMode::Oidc {
            (
                Some(env::var("OIDC_ISSUER_URL")?),
                Some(env::var("OIDC_CLIENT_ID")?),
                Some(env::var("OIDC_CLIENT_SECRET")?),
                Some(env::var("OIDC_REDIRECT_URL").unwrap_or_else(|_| {
                    format!("https://{}/api/auth/callback", public_domain)
                })),
                Some(env::var("SESSION_SECRET")?),
            )
        } else {
            (
                env::var("OIDC_ISSUER_URL").ok(),
                env::var("OIDC_CLIENT_ID").ok(),
                env::var("OIDC_CLIENT_SECRET").ok(),
                env::var("OIDC_REDIRECT_URL").ok(),
                env::var("SESSION_SECRET").ok(),
            )
        };

        Ok(Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://data/gavin.db".to_string()),
            upload_dir: env::var("UPLOAD_DIR").unwrap_or_else(|_| "data/uploads".to_string()),
            frontend_dir: env::var("FRONTEND_DIR").unwrap_or_else(|_| "dist".to_string()),
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()?,
            public_domain,
            auth_mode,
            oidc_issuer_url,
            oidc_client_id,
            oidc_client_secret,
            oidc_redirect_url,
            session_secret,
            cookie_secure: env::var("COOKIE_SECURE")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            album_metadata_enabled: env::var("ALBUM_METADATA_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            album_metadata_user_agent: env::var("ALBUM_METADATA_USER_AGENT").ok(),
            album_cover_recognition_provider: album_cover_recognition_provider(),
            openai_api_key: optional_env("OPENAI_API_KEY"),
            openai_base_url: env::var("OPENAI_BASE_URL")
                .unwrap_or_else(|_| "https://api.openai.com".to_string()),
            openai_model: env::var("OPENAI_ALBUM_COVER_MODEL")
                .unwrap_or_else(|_| "gpt-4o-mini".to_string()),
            gemini_api_key: optional_env("GEMINI_API_KEY"),
            gemini_base_url: env::var("GEMINI_BASE_URL")
                .unwrap_or_else(|_| "https://generativelanguage.googleapis.com".to_string()),
            gemini_model: env::var("GEMINI_ALBUM_COVER_MODEL")
                .unwrap_or_else(|_| "gemini-2.0-flash".to_string()),
            musicbrainz_base_url: env::var("MUSICBRAINZ_BASE_URL")
                .unwrap_or_else(|_| "https://musicbrainz.org".to_string()),
            cover_art_archive_base_url: env::var("COVER_ART_ARCHIVE_BASE_URL")
                .unwrap_or_else(|_| "https://coverartarchive.org".to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_mode_from_str() {
        assert_eq!(AuthMode::from_str("oidc").unwrap(), AuthMode::Oidc);
        assert_eq!(AuthMode::from_str("OIDC").unwrap(), AuthMode::Oidc);
        assert_eq!(AuthMode::from_str("dev").unwrap(), AuthMode::Dev);
        assert_eq!(AuthMode::from_str("DEV").unwrap(), AuthMode::Dev);
        assert!(AuthMode::from_str("invalid").is_err());
    }

    #[test]
    fn test_auth_mode_default() {
        assert_eq!(AuthMode::default(), AuthMode::Oidc);
    }

    #[test]
    fn test_dev_mode_no_oidc_required() {
        // Save current env
        let saved_vars: Vec<_> = [
            "AUTH_MODE",
            "OIDC_ISSUER_URL",
            "OIDC_CLIENT_ID",
            "OIDC_CLIENT_SECRET",
            "OIDC_REDIRECT_URL",
            "SESSION_SECRET",
        ]
        .iter()
        .map(|k| (*k, env::var(k).ok()))
        .collect();

        // Clear OIDC vars and set dev mode
        env::remove_var("OIDC_ISSUER_URL");
        env::remove_var("OIDC_CLIENT_ID");
        env::remove_var("OIDC_CLIENT_SECRET");
        env::remove_var("OIDC_REDIRECT_URL");
        env::remove_var("SESSION_SECRET");
        env::set_var("AUTH_MODE", "dev");

        // Should succeed without OIDC vars
        let result = Config::from_env();
        assert!(result.is_ok(), "Dev mode should not require OIDC vars");

        let config = result.unwrap();
        assert_eq!(config.auth_mode, AuthMode::Dev);
        assert!(config.oidc_issuer_url.is_none());

        // Restore env
        for (key, value) in saved_vars {
            match value {
                Some(v) => env::set_var(key, v),
                None => env::remove_var(key),
            }
        }
    }

    #[test]
    fn test_oidc_mode_requires_vars() {
        // Save current env
        let saved_vars: Vec<_> = ["AUTH_MODE", "OIDC_ISSUER_URL"]
            .iter()
            .map(|k| (*k, env::var(k).ok()))
            .collect();

        env::set_var("AUTH_MODE", "oidc");
        env::remove_var("OIDC_ISSUER_URL");

        // Should fail without OIDC vars
        let result = Config::from_env();
        assert!(result.is_err(), "OIDC mode should require OIDC vars");

        // Restore env
        for (key, value) in saved_vars {
            match value {
                Some(v) => env::set_var(key, v),
                None => env::remove_var(key),
            }
        }
    }
}
