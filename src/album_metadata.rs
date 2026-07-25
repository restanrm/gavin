//! Album metadata enrichment using public internet sources.
//!
//! MusicBrainz is used to identify albums, and Cover Art Archive is used for
//! cover artwork. Enrichment is best-effort: network/API failures are recorded
//! on the vinyl row instead of failing the user-facing create operation.

use std::time::Duration;

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use unicode_normalization::UnicodeNormalization;

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
    album_cover_recognition_provider: String,
    openai_api_key: Option<String>,
    openai_base_url: String,
    openai_model: String,
    gemini_api_key: Option<String>,
    gemini_base_url: String,
    gemini_model: String,
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
            .timeout(Duration::from_secs(20))
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
            album_cover_recognition_provider: config.album_cover_recognition_provider.clone(),
            openai_api_key: config.openai_api_key.clone(),
            openai_base_url: config.openai_base_url.trim_end_matches('/').to_string(),
            openai_model: config.openai_model.clone(),
            gemini_api_key: config.gemini_api_key.clone(),
            gemini_base_url: config.gemini_base_url.trim_end_matches('/').to_string(),
            gemini_model: config.gemini_model.clone(),
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

    /// Analyze an uploaded album-cover photo and return MusicBrainz candidates.
    ///
    /// Public non-AI music metadata sources do not expose reverse-image search, so
    /// the uploaded photo is sent to the configured vision provider for visual
    /// identification. The detected album terms are then resolved through
    /// MusicBrainz and Cover Art Archive, so imports store official artwork
    /// instead of the user's photographed image.
    pub async fn analyze_cover_image(
        &self,
        image_data: &[u8],
    ) -> anyhow::Result<CoverImageAnalysis> {
        if !self.enabled {
            anyhow::bail!("Album metadata lookup is disabled");
        }

        let reverse_terms = self.reverse_image_search(image_data).await?;
        let candidates = self.lookup_from_reverse_image_terms(&reverse_terms).await?;

        let status = if candidates.is_empty() {
            "not_found"
        } else if cover_candidates_have_clear_winner_for_terms(&candidates, &reverse_terms) {
            "complete"
        } else {
            "needs_choice"
        };

        Ok(CoverImageAnalysis {
            status: status.to_string(),
            detected_terms: reverse_terms,
            candidates,
        })
    }

    async fn reverse_image_search(&self, image_data: &[u8]) -> anyhow::Result<Vec<String>> {
        match self.album_cover_recognition_provider.as_str() {
            "gemini" => self.recognize_cover_with_gemini(image_data).await,
            "openai" | "chatgpt" => self.recognize_cover_with_openai(image_data).await,
            "disabled" | "none" => {
                anyhow::bail!("Album-cover recognition is disabled")
            }
            provider => anyhow::bail!(
                "Unsupported ALBUM_COVER_RECOGNITION_PROVIDER '{}'. Use 'gemini', 'openai', or 'disabled'.",
                provider
            ),
        }
    }

    async fn recognize_cover_with_openai(
        &self,
        image_data: &[u8],
    ) -> anyhow::Result<Vec<String>> {
        let api_key = self.openai_api_key.as_deref().ok_or_else(|| {
            anyhow::anyhow!("Album-cover recognition provider 'openai' requires OPENAI_API_KEY")
        })?;

        let image_url = format!(
            "data:{};base64,{}",
            image_mime_type(image_data),
            BASE64_STANDARD.encode(image_data)
        );

        let url = format!("{}/v1/chat/completions", self.openai_base_url);
        let request = serde_json::json!({
            "model": self.openai_model,
            "response_format": { "type": "json_object" },
            "max_tokens": 220,
            "messages": [
                {
                    "role": "system",
                    "content": "You identify music album covers. Return JSON only. If uncertain, include several likely search terms."
                },
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "text",
                            "text": album_cover_recognition_prompt()
                        },
                        {
                            "type": "image_url",
                            "image_url": { "url": image_url, "detail": "low" }
                        }
                    ]
                }
            ]
        });

        let response = self
            .send_openai_chat_request(&url, api_key, &request)
            .await?;

        let content = response
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .ok_or_else(|| anyhow::anyhow!("ChatGPT returned no album-cover response"))?;

        let recognition = parse_album_recognition("ChatGPT", &content)?;

        recognition_terms("ChatGPT", recognition.search_terms)
    }

    async fn recognize_cover_with_gemini(
        &self,
        image_data: &[u8],
    ) -> anyhow::Result<Vec<String>> {
        let api_key = self.gemini_api_key.as_deref().ok_or_else(|| {
            anyhow::anyhow!("Album-cover recognition provider 'gemini' requires GEMINI_API_KEY")
        })?;

        let url = format!(
            "{}/v1beta/{}:generateContent",
            self.gemini_base_url,
            gemini_model_path(&self.gemini_model)
        );
        let request = serde_json::json!({
            "systemInstruction": {
                "parts": [
                    {
                        "text": "You identify music album covers. Output only valid JSON matching the requested schema. Do not explain your reasoning."
                    }
                ]
            },
            "contents": [
                {
                    "role": "user",
                    "parts": [
                        { "text": album_cover_recognition_prompt() },
                        {
                            "inlineData": {
                                "mimeType": image_mime_type(image_data),
                                "data": BASE64_STANDARD.encode(image_data)
                            }
                        }
                    ]
                }
            ],
            "generationConfig": {
                "responseMimeType": "application/json",
                "responseSchema": {
                    "type": "OBJECT",
                    "properties": {
                        "search_terms": {
                            "type": "ARRAY",
                            "items": { "type": "STRING" }
                        }
                    },
                    "required": ["search_terms"]
                },
                "maxOutputTokens": 220,
                "temperature": 0.0
            }
        });

        let response = self
            .send_gemini_request(&url, api_key, &request)
            .await?;

        let content = response
            .candidates
            .into_iter()
            .next()
            .map(|candidate| {
                candidate
                    .content
                    .parts
                    .into_iter()
                    .map(|part| part.text)
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .ok_or_else(|| anyhow::anyhow!("Gemini returned no album-cover response"))?;

        let recognition = parse_album_recognition("Gemini", &content)?;

        recognition_terms("Gemini", recognition.search_terms)
    }

    async fn send_openai_chat_request(
        &self,
        url: &str,
        api_key: &str,
        request: &serde_json::Value,
    ) -> anyhow::Result<OpenAiChatResponse> {
        let mut retry_delay = Duration::from_secs(2);

        for attempt in 0..3 {
            let response = self
                .http
                .post(url)
                .bearer_auth(api_key)
                .json(request)
                .send()
                .await?;

            if response.status().is_success() {
                return Ok(response.json().await?);
            }

            let status = response.status();
            let retry_after = retry_after_duration(response.headers());
            let body = response.text().await.unwrap_or_default();
            let message = provider_error_message("OpenAI", status, &body);

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS && attempt < 2 {
                let delay = retry_after.unwrap_or(retry_delay);
                tracing::warn!(
                    attempt = attempt + 1,
                    delay_seconds = delay.as_secs(),
                    "OpenAI rate limit hit during album-cover recognition; retrying"
                );
                tokio::time::sleep(delay).await;
                retry_delay *= 2;
                continue;
            }

            anyhow::bail!(message);
        }

        unreachable!("OpenAI request retry loop should return or bail")
    }

    async fn send_gemini_request(
        &self,
        url: &str,
        api_key: &str,
        request: &serde_json::Value,
    ) -> anyhow::Result<GeminiResponse> {
        let mut retry_delay = Duration::from_secs(2);

        for attempt in 0..3 {
            let response = self
                .http
                .post(url)
                .query(&[("key", api_key)])
                .json(request)
                .send()
                .await?;

            if response.status().is_success() {
                return Ok(response.json().await?);
            }

            let status = response.status();
            let retry_after = retry_after_duration(response.headers());
            let body = response.text().await.unwrap_or_default();
            let message = provider_error_message("Gemini", status, &body);

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS && attempt < 2 {
                let delay = retry_after.unwrap_or(retry_delay);
                tracing::warn!(
                    attempt = attempt + 1,
                    delay_seconds = delay.as_secs(),
                    "Gemini rate limit hit during album-cover recognition; retrying"
                );
                tokio::time::sleep(delay).await;
                retry_delay *= 2;
                continue;
            }

            anyhow::bail!(message);
        }

        unreachable!("Gemini request retry loop should return or bail")
    }

    /// Search MusicBrainz for albums by artist so admins can add one or more
    /// albums without typing each title manually.
    pub async fn search_artist_albums(&self, artist: &str) -> anyhow::Result<Vec<AlbumCandidate>> {
        if !self.enabled {
            anyhow::bail!("Album metadata lookup is disabled");
        }

        let artist = artist.trim();
        if artist.is_empty() {
            anyhow::bail!("Artist name is required");
        }

        let query = format!("artist:\"{}\"", escape_lucene_phrase(artist));
        let mut candidates = self.search_musicbrainz_query(&query, 50).await?;
        candidates.retain(|candidate| {
            candidate.score.unwrap_or(0) >= 70
                && normalize(&candidate.artist).contains(&normalize(artist))
        });

        candidates.sort_by(|left, right| {
            right
                .score
                .unwrap_or(0)
                .cmp(&left.score.unwrap_or(0))
                .then_with(|| left.release_year.cmp(&right.release_year))
                .then_with(|| left.title.cmp(&right.title))
        });
        candidates.truncate(30);

        for candidate in &mut candidates {
            candidate.cover_image_url = self
                .fetch_cover_url(&candidate.id)
                .await
                .ok()
                .flatten();
        }

        Ok(candidates)
    }

    async fn lookup_from_reverse_image_terms(
        &self,
        terms: &[String],
    ) -> anyhow::Result<Vec<AlbumCandidate>> {
        let mut candidates = Vec::<AlbumCandidate>::new();
        let mut seen_ids = std::collections::HashSet::<String>::new();

        for (index, query) in reverse_image_musicbrainz_queries(terms).into_iter().enumerate() {
            if index > 0 {
                tokio::time::sleep(Duration::from_millis(1100)).await;
            }

            let mut matches = self.search_musicbrainz_query(&query, 8).await?;
            for candidate in &mut matches {
                candidate.cover_image_url = self
                    .fetch_cover_url(&candidate.id)
                    .await
                    .ok()
                    .flatten();
            }

            for candidate in matches {
                if candidate.score.unwrap_or(0) >= 70 && seen_ids.insert(candidate.id.clone()) {
                    candidates.push(candidate);
                }
            }

            if cover_candidates_have_clear_winner_for_terms(&candidates, terms) {
                break;
            }
        }

        candidates.retain(|candidate| cover_candidate_relevance(candidate, terms) >= 40);
        candidates.sort_by(|left, right| {
            let left_relevance = cover_candidate_relevance(left, terms);
            let right_relevance = cover_candidate_relevance(right, terms);
            right_relevance
                .cmp(&left_relevance)
                .then_with(|| {
                    right
                        .cover_image_url
                        .is_some()
                        .cmp(&left.cover_image_url.is_some())
                })
                .then_with(|| right.score.unwrap_or(0).cmp(&left.score.unwrap_or(0)))
                .then_with(|| left.title.cmp(&right.title))
        });
        candidates.truncate(5);

        Ok(candidates)
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
        let query = format!(
            "releasegroup:\"{}\" AND artist:\"{}\"",
            escape_lucene_phrase(title),
            escape_lucene_phrase(artist)
        );

        self.search_musicbrainz_query(&query, 5).await
    }

    async fn search_musicbrainz_query(
        &self,
        query: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<AlbumCandidate>> {
        let url = format!("{}/ws/2/release-group", self.musicbrainz_base_url);
        let limit = limit.to_string();

        let response: MusicBrainzReleaseGroupResponse = self
            .http
            .get(url)
            .query(&[
                ("query", query),
                ("type", "album"),
                ("fmt", "json"),
                ("limit", limit.as_str()),
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

/// Result of analyzing a user-uploaded album-cover photo.
#[derive(Debug, Clone, Serialize)]
pub struct CoverImageAnalysis {
    pub status: String,
    pub detected_terms: Vec<String>,
    pub candidates: Vec<AlbumCandidate>,
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

#[derive(Debug, Deserialize)]
struct OpenAiChatResponse {
    #[serde(default)]
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    #[serde(default)]
    candidates: Vec<GeminiCandidate>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
}

#[derive(Debug, Deserialize)]
struct GeminiContent {
    #[serde(default)]
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Deserialize)]
struct GeminiPart {
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct AlbumRecognition {
    #[serde(default)]
    search_terms: Vec<String>,
}

fn album_cover_recognition_prompt() -> &'static str {
    "Identify this music album cover. Return only a JSON object, with no markdown and no explanatory text. The JSON schema is {\"search_terms\":[\"Artist - Album title\"]}. Include 1 to 6 strings. Prefer terms like 'Artist - Album title' when the artist is known. If only the album title is visible, return the album title alone rather than pairing it with unrelated label, subtitle, catalog, or marketing text. Do not invent details; include alternate guesses only if plausible."
}

fn parse_album_recognition(provider: &str, content: &str) -> anyhow::Result<AlbumRecognition> {
    if let Some(payload) = json_payload(content) {
        return serde_json::from_str(payload).map_err(|err| {
            anyhow::anyhow!(
                "failed to parse {} album-cover response as JSON: {}. Response preview: {}",
                provider,
                err,
                response_preview(content)
            )
        });
    }

    let fallback_terms = quoted_terms_from_text(content);
    if !fallback_terms.is_empty() {
        tracing::warn!(
            provider,
            "album-cover provider returned non-JSON text; using quoted terms fallback"
        );
        return Ok(AlbumRecognition {
            search_terms: fallback_terms,
        });
    }

    anyhow::bail!(
        "{} returned an empty or non-JSON album-cover response: {}",
        provider,
        response_preview(content)
    )
}

fn recognition_terms(provider: &str, raw_terms: Vec<String>) -> anyhow::Result<Vec<String>> {
    let terms = reverse_image_search_terms(raw_terms);
    if terms.is_empty() {
        anyhow::bail!("{} did not identify album-like search terms", provider);
    }

    Ok(terms)
}

fn normalize(value: &str) -> String {
    value
        .nfkd()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect()
}

fn image_mime_type(image_data: &[u8]) -> &'static str {
    if image_data.starts_with(&[0x89, b'P', b'N', b'G']) {
        "image/png"
    } else if image_data.starts_with(b"GIF87a") || image_data.starts_with(b"GIF89a") {
        "image/gif"
    } else if image_data.starts_with(b"RIFF") && image_data.get(8..12) == Some(b"WEBP") {
        "image/webp"
    } else {
        "image/jpeg"
    }
}

fn gemini_model_path(model: &str) -> String {
    if model.starts_with("models/") {
        model.to_string()
    } else {
        format!("models/{model}")
    }
}

fn strip_json_code_fence(value: &str) -> &str {
    let trimmed = value.trim();
    let Some(without_opening) = trimmed.strip_prefix("```") else {
        return trimmed;
    };

    let without_language = without_opening
        .strip_prefix("json")
        .unwrap_or(without_opening)
        .trim_start();

    without_language
        .strip_suffix("```")
        .unwrap_or(without_language)
        .trim()
}

fn json_payload(value: &str) -> Option<&str> {
    let stripped = strip_json_code_fence(value).trim();
    if stripped.is_empty() {
        return None;
    }

    if stripped.starts_with('{') {
        return Some(stripped);
    }

    let start = stripped.find('{')?;
    let end = stripped.rfind('}')?;
    (start < end).then(|| &stripped[start..=end])
}

fn response_preview(value: &str) -> String {
    let preview = value.trim().chars().take(300).collect::<String>();
    if preview.is_empty() {
        "<empty>".to_string()
    } else {
        preview
    }
}

fn quoted_terms_from_text(value: &str) -> Vec<String> {
    let mut quoted = Vec::<String>::new();
    let mut current = String::new();
    let mut in_quote = false;

    for character in value.chars() {
        match character {
            '"' | '“' | '”' => {
                if in_quote {
                    push_search_term(&mut quoted, current.clone());
                    current.clear();
                    in_quote = false;
                } else {
                    in_quote = true;
                }
            }
            _ if in_quote => current.push(character),
            _ => {}
        }
    }

    let mut terms = Vec::<String>::new();
    if quoted.len() >= 2 {
        push_search_term(
            &mut terms,
            quoted.iter().take(4).cloned().collect::<Vec<_>>().join(" "),
        );
    }

    for term in quoted {
        push_search_term(&mut terms, term);
    }

    terms
}

fn retry_after_duration(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs)
}

fn provider_error_message(provider: &str, status: reqwest::StatusCode, body: &str) -> String {
    let provider_message = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("error")
                .and_then(|error| error.get("message"))
                .and_then(|message| message.as_str())
                .map(str::to_string)
        })
        .or_else(|| {
            let trimmed = body.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        });

    let provider_message = provider_message.unwrap_or_else(|| "no provider details".to_string());

    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return format!(
            "{provider} rejected album-cover recognition with 429 Too Many Requests. This usually means the API key/project has no available quota or is rate-limited. Check provider billing/free-tier limits, wait for the limit to reset, or configure another API key/provider. Provider message: {provider_message}"
        );
    }

    format!(
        "{provider} album-cover recognition failed with HTTP {status}. Provider message: {provider_message}"
    )
}

fn reverse_image_search_terms(raw_terms: Vec<String>) -> Vec<String> {
    let mut terms = Vec::<String>::new();

    for term in raw_terms {
        push_search_term(&mut terms, cleanup_reverse_image_term(&term));
    }

    terms.truncate(12);
    terms
}

fn reverse_image_musicbrainz_queries(terms: &[String]) -> Vec<String> {
    let mut queries = Vec::<String>::new();

    for term in terms.iter().take(8) {
        push_search_term(&mut queries, term.clone());
    }

    for term in terms.iter().take(6) {
        if let Some((artist, title)) = split_artist_title(term) {
            queries.push(format!(
                "releasegroup:\"{}\" AND artist:\"{}\"",
                escape_lucene_phrase(&title),
                escape_lucene_phrase(&artist)
            ));
            queries.push(format!(
                "releasegroup:\"{}\" AND artist:\"{}\"",
                escape_lucene_phrase(&artist),
                escape_lucene_phrase(&title)
            ));
        }
    }

    let mut deduped = Vec::new();
    for query in queries {
        push_search_term(&mut deduped, query);
        if deduped.len() == 12 {
            break;
        }
    }

    deduped
}

fn push_search_term(terms: &mut Vec<String>, term: String) {
    let term = term.trim();
    if term.len() < 3 || term.chars().all(|character| character.is_ascii_digit()) {
        return;
    }

    let normalized = normalize(term);
    let has_artist_title_separator = split_artist_title(term).is_some();
    let is_duplicate = terms.iter().any(|existing| {
        normalize(existing) == normalized
            && !(has_artist_title_separator && split_artist_title(existing).is_none())
    });

    if normalized.len() < 3 || is_duplicate {
        return;
    }

    terms.push(term.to_string());
}

fn cleanup_reverse_image_term(term: &str) -> String {
    let without_html = strip_html_tags(term);
    let mut cleaned = without_html
        .replace(" - Wikipedia", "")
        .replace(" | Discogs", "")
        .replace(" | MusicBrainz", "")
        .replace(" album cover", "")
        .replace(" Album Cover", "")
        .replace(" cover art", "")
        .replace(" Cover Art", "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    cleaned = cleaned
        .trim_matches(|character: char| !character.is_alphanumeric())
        .to_string();

    cleaned
}

fn strip_html_tags(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut in_tag = false;

    for character in value.chars() {
        match character {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(character),
            _ => {}
        }
    }

    output
}

fn split_artist_title(value: &str) -> Option<(String, String)> {
    for separator in [" - ", " – ", " — "] {
        if let Some((left, right)) = value.split_once(separator) {
            if left.trim().len() >= 2 && right.trim().len() >= 2 {
                return Some((left.trim().to_string(), right.trim().to_string()));
            }
        }
    }

    if let Some((title, artist)) = value.split_once(" by ") {
        if title.trim().len() >= 2 && artist.trim().len() >= 2 {
            return Some((artist.trim().to_string(), title.trim().to_string()));
        }
    }

    None
}

fn cover_candidates_have_clear_winner_for_terms(
    candidates: &[AlbumCandidate],
    terms: &[String],
) -> bool {
    if candidates.is_empty() {
        return false;
    }

    let mut scores = candidates
        .iter()
        .map(|candidate| {
            (
                cover_candidate_relevance(candidate, terms),
                candidate.score.unwrap_or(0),
            )
        })
        .collect::<Vec<_>>();
    scores.sort_unstable_by(|left, right| right.cmp(left));

    let (top_relevance, top_musicbrainz_score) = scores[0];
    let (runner_up_relevance, runner_up_musicbrainz_score) = scores.get(1).copied().unwrap_or((0, 0));

    top_relevance >= 180
        && (top_relevance.saturating_sub(runner_up_relevance) >= 80
            || (top_relevance > runner_up_relevance
                && top_musicbrainz_score.saturating_sub(runner_up_musicbrainz_score) >= 15))
}

fn cover_candidate_relevance(candidate: &AlbumCandidate, terms: &[String]) -> i32 {
    let mut relevance = 0;
    let title = normalize(&candidate.title);
    let artist = normalize(&candidate.artist);
    let fragments = cover_term_fragments(terms);

    for fragment in &fragments {
        let fragment = normalize(fragment);
        if fragment.is_empty() {
            continue;
        }

        if !title.is_empty() && fragment == title {
            relevance += 220;
        } else if title.len() >= 6 && fragment.contains(&title) {
            relevance += 160;
        } else if fragment.len() >= 6 && title.contains(&fragment) {
            relevance += 90;
        }

        if !artist.is_empty() && fragment == artist {
            relevance += 180;
        } else if artist.len() >= 6 && fragment.contains(&artist) {
            relevance += 120;
        } else if fragment.len() >= 6 && artist.contains(&fragment) {
            relevance += 70;
        }
    }

    for term in terms {
        let term_tokens = meaningful_tokens(term);
        let title_overlap = meaningful_tokens(&candidate.title)
            .into_iter()
            .filter(|token| term_tokens.contains(token))
            .count() as i32;
        let artist_overlap = meaningful_tokens(&candidate.artist)
            .into_iter()
            .filter(|token| term_tokens.contains(token))
            .count() as i32;

        if title_overlap >= 2 {
            relevance += (title_overlap * 25).min(80);
        }
        if artist_overlap >= 2 {
            relevance += (artist_overlap * 20).min(60);
        }
    }

    relevance
}

fn cover_term_fragments(terms: &[String]) -> Vec<String> {
    let mut fragments = Vec::<String>::new();
    for term in terms {
        push_search_term(&mut fragments, term.clone());
        if let Some((left, right)) = split_artist_title(term) {
            push_search_term(&mut fragments, left);
            push_search_term(&mut fragments, right);
        }
    }
    fragments
}

fn meaningful_tokens(value: &str) -> Vec<String> {
    value
        .split(|character: char| !character.is_alphanumeric())
        .map(normalize)
        .filter(|token| token.len() >= 4)
        .collect()
}

fn escape_lucene_phrase(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_for_matching() {
        assert_eq!(normalize("The Dark Side of the Moon"), "thedarksideofthemoon");
        assert_eq!(normalize("Björk - Debut!"), "bjorkdebut");
        assert_eq!(normalize("Gaël Faye"), normalize("Gael Faye"));
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

    #[test]
    fn prepares_reverse_image_terms_for_musicbrainz_search() {
        let terms = reverse_image_search_terms(vec![
            "The Beatles Abbey Road album cover".to_string(),
            "The Beatles - Abbey Road".to_string(),
            "<b>Abbey Road</b> by The Beatles | Discogs".to_string(),
        ]);

        assert_eq!(terms[0], "The Beatles Abbey Road");
        assert!(terms.iter().any(|term| term == "The Beatles - Abbey Road"));

        let queries = reverse_image_musicbrainz_queries(&terms);
        assert!(queries.iter().any(|query| query.contains("releasegroup")));
        assert!(queries.iter().any(|query| query == "The Beatles Abbey Road"));
    }

    #[test]
    fn builds_gemini_model_paths() {
        assert_eq!(gemini_model_path("gemini-2.0-flash"), "models/gemini-2.0-flash");
        assert_eq!(gemini_model_path("models/gemini-2.0-flash"), "models/gemini-2.0-flash");
    }

    #[test]
    fn extracts_json_payloads_from_model_responses() {
        assert_eq!(
            json_payload("```json\n{\"search_terms\":[\"A - B\"]}\n```"),
            Some("{\"search_terms\":[\"A - B\"]}")
        );
        assert_eq!(json_payload(" {\"search_terms\":[]} "), Some("{\"search_terms\":[]}"));
        assert_eq!(
            json_payload("Here is the result: {\"search_terms\":[\"A - B\"]}"),
            Some("{\"search_terms\":[\"A - B\"]}")
        );
        assert_eq!(json_payload(""), None);
    }

    #[test]
    fn parses_album_recognition_payloads() {
        let recognition = parse_album_recognition(
            "Gemini",
            "Here is the result: {\"search_terms\":[\"The Beatles - Abbey Road\"]}",
        )
        .unwrap();

        assert_eq!(recognition.search_terms, vec!["The Beatles - Abbey Road"]);
    }

    #[test]
    fn falls_back_to_quoted_terms_for_non_json_responses() {
        let recognition = parse_album_recognition(
            "Gemini",
            "The text says \"amour supreme\" and \"99 revolution\" plus \"le monde de karmany\".",
        )
        .unwrap();

        assert!(recognition
            .search_terms
            .iter()
            .any(|term| term.contains("amour supreme")));
        assert!(recognition
            .search_terms
            .iter()
            .any(|term| term == "99 revolution"));
    }

    #[test]
    fn ranks_cover_candidates_against_detected_terms() {
        let terms = vec!["Amour Supreme - Le Monde de Karmany".to_string()];
        let youssoupha = AlbumCandidate {
            source: SOURCE_NAME.to_string(),
            id: "youssoupha".to_string(),
            title: "AMOUR SUPREME".to_string(),
            artist: "Youssoupha".to_string(),
            release_year: Some(2025),
            cover_image_url: Some("https://example.com/youssoupha.jpg".to_string()),
            disambiguation: None,
            source_url: "https://musicbrainz.org/release-group/youssoupha".to_string(),
            score: Some(91),
        };
        let ntm = AlbumCandidate {
            source: SOURCE_NAME.to_string(),
            id: "ntm".to_string(),
            title: "Le Monde de demain".to_string(),
            artist: "Suprême NTM".to_string(),
            release_year: Some(1990),
            cover_image_url: Some("https://example.com/ntm.jpg".to_string()),
            disambiguation: None,
            source_url: "https://musicbrainz.org/release-group/ntm".to_string(),
            score: Some(100),
        };

        assert!(
            cover_candidate_relevance(&youssoupha, &terms)
                > cover_candidate_relevance(&ntm, &terms)
        );
        assert!(cover_candidate_relevance(&ntm, &terms) < 40);
    }

    #[test]
    fn detects_clear_term_based_cover_candidate_winner() {
        let terms = vec!["Artist - Album".to_string()];
        let winner = AlbumCandidate {
            source: SOURCE_NAME.to_string(),
            id: "winner".to_string(),
            title: "Album".to_string(),
            artist: "Artist".to_string(),
            release_year: Some(2020),
            cover_image_url: Some("https://example.com/cover.jpg".to_string()),
            disambiguation: None,
            source_url: "https://musicbrainz.org/release-group/winner".to_string(),
            score: Some(100),
        };
        let mut runner_up = winner.clone();
        runner_up.id = "runner-up".to_string();
        runner_up.title = "Other Album".to_string();
        runner_up.score = Some(90);

        assert!(cover_candidates_have_clear_winner_for_terms(
            &[winner.clone(), runner_up.clone()],
            &terms
        ));

        runner_up.title = "Album".to_string();
        assert!(!cover_candidates_have_clear_winner_for_terms(
            &[winner, runner_up],
            &terms
        ));
    }
}
