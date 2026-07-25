//! Album metadata enrichment using public internet sources.
//!
//! MusicBrainz is used to identify albums, and Cover Art Archive is used for
//! cover artwork. Enrichment is best-effort: network/API failures are recorded
//! on the vinyl row instead of failing the user-facing create operation.

use std::time::Duration;

use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{
    config::Config,
    db::{MetadataUpdate, Vinyl},
    error::{AppError, Result},
};

const SOURCE_NAME: &str = "musicbrainz";
const STARTUP_BATCH_SIZE: i64 = 100;

/// Metadata provider client.
#[derive(Clone)]
pub struct AlbumMetadataClient {
    http: Client,
    enabled: bool,
    musicbrainz_base_url: String,
    cover_art_archive_base_url: String,
}

impl AlbumMetadataClient {
    pub fn from_config(config: &Config) -> anyhow::Result<Self> {
        let user_agent = config.album_metadata_user_agent.clone().unwrap_or_else(|| {
            format!(
                "gavin/{} (https://{})",
                env!("CARGO_PKG_VERSION"),
                config.public_domain
            )
        });

        let http = Client::builder()
            .timeout(Duration::from_secs(8))
            .user_agent(user_agent)
            .build()?;

        Ok(Self {
            http,
            enabled: config.album_metadata_enabled,
            musicbrainz_base_url: config.musicbrainz_base_url.trim_end_matches('/').to_string(),
            cover_art_archive_base_url: config
                .cover_art_archive_base_url
                .trim_end_matches('/')
                .to_string(),
        })
    }

    /// Enrich one vinyl row and return the refreshed row.
    pub async fn enrich_vinyl(&self, pool: &SqlitePool, vinyl_id: &str) -> Result<Vinyl> {
        let vinyl = Vinyl::get(pool, vinyl_id).await?;

        if !self.enabled {
            Vinyl::update_metadata(
                pool,
                vinyl_id,
                MetadataUpdate::error("disabled", "Album metadata lookup is disabled"),
            )
            .await?;
            return Vinyl::get(pool, vinyl_id).await;
        }

        match self.lookup(&vinyl.artist, &vinyl.title).await {
            Ok(LookupOutcome::Selected(candidate)) => {
                let update = MetadataUpdate {
                    release_year: vinyl.release_year.or(candidate.release_year),
                    notes: vinyl.notes.or_else(|| candidate.notes()),
                    cover_image_url: vinyl.cover_image_url.or(candidate.cover_image_url.clone()),
                    metadata_status: "complete".to_string(),
                    metadata_source: Some(SOURCE_NAME.to_string()),
                    metadata_source_id: Some(candidate.id.clone()),
                    metadata_source_url: Some(candidate.source_url.clone()),
                    metadata_candidates: None,
                    metadata_error: None,
                    metadata_checked_at: Some(Utc::now()),
                };
                Vinyl::update_metadata(pool, vinyl_id, update).await?;
            }
            Ok(LookupOutcome::NeedsChoice(candidates)) => {
                let candidates_json = serde_json::to_string(&candidates).map_err(|err| {
                    AppError::Internal(anyhow::anyhow!("failed to serialize candidates: {err}"))
                })?;

                Vinyl::update_metadata(
                    pool,
                    vinyl_id,
                    MetadataUpdate {
                        release_year: vinyl.release_year,
                        notes: vinyl.notes,
                        cover_image_url: vinyl.cover_image_url,
                        metadata_status: "needs_choice".to_string(),
                        metadata_source: Some(SOURCE_NAME.to_string()),
                        metadata_source_id: None,
                        metadata_source_url: None,
                        metadata_candidates: Some(candidates_json),
                        metadata_error: None,
                        metadata_checked_at: Some(Utc::now()),
                    },
                )
                .await?;
            }
            Ok(LookupOutcome::NotFound) => {
                Vinyl::update_metadata(
                    pool,
                    vinyl_id,
                    MetadataUpdate::error("not_found", "No album metadata match found"),
                )
                .await?;
            }
            Err(err) => {
                tracing::warn!(vinyl_id, error = %err, "album metadata lookup failed");
                Vinyl::update_metadata(
                    pool,
                    vinyl_id,
                    MetadataUpdate::error("error", err.to_string()),
                )
                .await?;
            }
        }

        Vinyl::get(pool, vinyl_id).await
    }

    async fn lookup(&self, artist: &str, title: &str) -> anyhow::Result<LookupOutcome> {
        let mut candidates = self.search_musicbrainz(artist, title).await?;

        if candidates.is_empty() {
            return Ok(LookupOutcome::NotFound);
        }

        for candidate in &mut candidates {
            candidate.cover_image_url = self.fetch_cover_url(&candidate.id).await.ok().flatten();
        }

        let plausible: Vec<AlbumCandidate> = candidates
            .into_iter()
            .filter(|candidate| candidate.score.unwrap_or(0) >= 90)
            .collect();

        if plausible.is_empty() {
            return Ok(LookupOutcome::NotFound);
        }

        let exact: Vec<AlbumCandidate> = plausible
            .iter()
            .filter(|candidate| {
                normalize(&candidate.title) == normalize(title)
                    && normalize(&candidate.artist).contains(&normalize(artist))
            })
            .cloned()
            .collect();

        match exact.len() {
            1 => Ok(LookupOutcome::Selected(exact.into_iter().next().unwrap())),
            n if n > 1 => Ok(LookupOutcome::NeedsChoice(exact)),
            _ if plausible.len() == 1 => Ok(LookupOutcome::Selected(
                plausible.into_iter().next().unwrap(),
            )),
            _ => Ok(LookupOutcome::NeedsChoice(plausible)),
        }
    }

    async fn search_musicbrainz(
        &self,
        artist: &str,
        title: &str,
    ) -> anyhow::Result<Vec<AlbumCandidate>> {
        let query = format!("releasegroup:\"{}\" AND artist:\"{}\"", title, artist);
        let url = format!("{}/ws/2/release-group", self.musicbrainz_base_url);

        let response: MusicBrainzReleaseGroupResponse = self
            .http
            .get(url)
            .query(&[
                ("query", query.as_str()),
                ("type", "album"),
                ("fmt", "json"),
                ("limit", "5"),
            ])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(response
            .release_groups
            .into_iter()
            .map(AlbumCandidate::from)
            .collect())
    }

    async fn fetch_cover_url(&self, release_group_id: &str) -> anyhow::Result<Option<String>> {
        let url = format!(
            "{}/release-group/{}",
            self.cover_art_archive_base_url, release_group_id
        );

        let response = self.http.get(url).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let cover: CoverArtArchiveResponse = response.error_for_status()?.json().await?;
        Ok(cover.images.into_iter().find_map(|image| {
            if !image.front.unwrap_or(false) {
                return None;
            }
            image
                .thumbnails
                .and_then(|thumbnails| thumbnails.large.or(thumbnails.small))
                .or(image.image)
        }))
    }
}

/// Start a best-effort asynchronous job that fills missing metadata after app startup.
pub fn spawn_startup_metadata_job(pool: SqlitePool, client: AlbumMetadataClient) {
    tokio::spawn(async move {
        if !client.enabled {
            tracing::info!("Album metadata startup check skipped because lookups are disabled");
            return;
        }

        tracing::info!("Starting album metadata completeness check");
        match Vinyl::list_requiring_metadata(&pool, STARTUP_BATCH_SIZE).await {
            Ok(ids) => {
                let total = ids.len();
                for (index, id) in ids.into_iter().enumerate() {
                    if index > 0 {
                        tokio::time::sleep(Duration::from_millis(1100)).await;
                    }

                    if let Err(err) = client.enrich_vinyl(&pool, &id).await {
                        tracing::warn!(vinyl_id = %id, error = %err, "startup metadata enrichment failed");
                    }
                }
                tracing::info!(checked = total, "Album metadata completeness check finished");
            }
            Err(err) => tracing::warn!(error = %err, "failed to list vinyls requiring metadata"),
        }
    });
}

#[derive(Debug)]
enum LookupOutcome {
    Selected(AlbumCandidate),
    NeedsChoice(Vec<AlbumCandidate>),
    NotFound,
}

/// Candidate album shown to admins when MusicBrainz returns several plausible matches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumCandidate {
    pub source: String,
    pub id: String,
    pub title: String,
    pub artist: String,
    pub release_year: Option<i32>,
    pub cover_image_url: Option<String>,
    pub disambiguation: Option<String>,
    pub source_url: String,
    pub score: Option<i32>,
}

impl AlbumCandidate {
    fn notes(&self) -> Option<String> {
        Some(format!("Metadata: {}", self.source_url))
    }
}

impl From<MusicBrainzReleaseGroup> for AlbumCandidate {
    fn from(group: MusicBrainzReleaseGroup) -> Self {
        let artist = group
            .artist_credit
            .into_iter()
            .map(|credit| credit.name)
            .collect::<Vec<_>>()
            .join("")
            .trim()
            .to_string();

        let source_url = format!("https://musicbrainz.org/release-group/{}", group.id);

        Self {
            source: SOURCE_NAME.to_string(),
            id: group.id,
            title: group.title,
            artist,
            release_year: group
                .first_release_date
                .as_deref()
                .and_then(|date| date.get(0..4))
                .and_then(|year| year.parse().ok()),
            cover_image_url: None,
            disambiguation: group.disambiguation.filter(|value| !value.trim().is_empty()),
            source_url,
            score: group.score,
        }
    }
}

#[derive(Debug, Deserialize)]
struct MusicBrainzReleaseGroupResponse {
    #[serde(default, rename = "release-groups")]
    release_groups: Vec<MusicBrainzReleaseGroup>,
}

#[derive(Debug, Deserialize)]
struct MusicBrainzReleaseGroup {
    id: String,
    title: String,
    score: Option<i32>,
    #[serde(default, rename = "first-release-date")]
    first_release_date: Option<String>,
    #[serde(default, rename = "artist-credit")]
    artist_credit: Vec<MusicBrainzArtistCredit>,
    #[serde(default)]
    disambiguation: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MusicBrainzArtistCredit {
    name: String,
}

#[derive(Debug, Deserialize)]
struct CoverArtArchiveResponse {
    #[serde(default)]
    images: Vec<CoverArtImage>,
}

#[derive(Debug, Deserialize)]
struct CoverArtImage {
    image: Option<String>,
    front: Option<bool>,
    thumbnails: Option<CoverArtThumbnails>,
}

#[derive(Debug, Deserialize)]
struct CoverArtThumbnails {
    small: Option<String>,
    large: Option<String>,
}

fn normalize(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_for_matching() {
        assert_eq!(normalize("The Dark Side of the Moon"), "thedarksideofthemoon");
        assert_eq!(normalize("Björk - Debut!"), "bjrkdebut");
    }

    #[test]
    fn parses_release_group_candidate() {
        let group = MusicBrainzReleaseGroup {
            id: "abc".to_string(),
            title: "Abbey Road".to_string(),
            score: Some(100),
            first_release_date: Some("1969-09-26".to_string()),
            artist_credit: vec![MusicBrainzArtistCredit {
                name: "The Beatles".to_string(),
            }],
            disambiguation: Some("".to_string()),
        };

        let candidate = AlbumCandidate::from(group);
        assert_eq!(candidate.artist, "The Beatles");
        assert_eq!(candidate.release_year, Some(1969));
        assert_eq!(candidate.disambiguation, None);
    }
}
