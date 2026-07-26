//! Database models and operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::error::{AppError, Result};

/// Vinyl record model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Vinyl {
    pub id: String,
    pub artist: String,
    pub title: String,
    pub release_year: Option<i32>,
    pub genre: Option<String>,
    pub notes: Option<String>,
    pub cover_image_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata_status: String,
    pub metadata_source: Option<String>,
    pub metadata_source_id: Option<String>,
    pub metadata_source_url: Option<String>,
    pub metadata_candidates: Option<String>,
    pub metadata_error: Option<String>,
    pub metadata_checked_at: Option<DateTime<Utc>>,
}

/// Input for creating a new vinyl
#[derive(Debug, Deserialize)]
pub struct CreateVinyl {
    pub artist: String,
    pub title: String,
    #[serde(alias = "year")]
    pub release_year: Option<i32>,
    pub genre: Option<String>,
    pub notes: Option<String>,
    #[serde(alias = "cover_url")]
    pub cover_image_url: Option<String>,
}

/// A JSON patch field that can distinguish an omitted field from an explicit null.
#[derive(Debug, Clone, PartialEq)]
pub enum PatchField<T> {
    Missing,
    Null,
    Value(T),
}

impl<T> Default for PatchField<T> {
    fn default() -> Self {
        Self::Missing
    }
}

impl<'de, T> Deserialize<'de> for PatchField<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Option::<T>::deserialize(deserializer)?.map_or(Self::Null, Self::Value))
    }
}

/// Input for updating a vinyl
#[derive(Debug, Deserialize)]
pub struct UpdateVinyl {
    #[serde(default)]
    pub artist: PatchField<String>,
    #[serde(default)]
    pub title: PatchField<String>,
    #[serde(default)]
    pub release_year: PatchField<i32>,
    #[serde(default)]
    pub genre: PatchField<String>,
    #[serde(default)]
    pub notes: PatchField<String>,
    #[serde(default)]
    pub cover_image_url: PatchField<String>,
}

/// Metadata fields updated by the album metadata enrichment job.
#[derive(Debug)]
pub struct MetadataUpdate {
    pub release_year: Option<i32>,
    pub genre: Option<String>,
    pub notes: Option<String>,
    pub cover_image_url: Option<String>,
    pub metadata_status: String,
    pub metadata_source: Option<String>,
    pub metadata_source_id: Option<String>,
    pub metadata_source_url: Option<String>,
    pub metadata_candidates: Option<String>,
    pub metadata_error: Option<String>,
    pub metadata_checked_at: Option<DateTime<Utc>>,
}

/// Existing cover URL that should be cached locally by the metadata job.
#[derive(Debug, FromRow)]
pub struct VinylCoverImage {
    pub id: String,
    pub cover_image_url: String,
    pub metadata_source_id: Option<String>,
}

impl MetadataUpdate {
    pub fn error(status: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            release_year: None,
            genre: None,
            notes: None,
            cover_image_url: None,
            metadata_status: status.into(),
            metadata_source: Some("musicbrainz".to_string()),
            metadata_source_id: None,
            metadata_source_url: None,
            metadata_candidates: None,
            metadata_error: Some(error.into()),
            metadata_checked_at: Some(Utc::now()),
        }
    }
}

/// Bulk create request
#[derive(Debug, Deserialize)]
pub struct BulkCreateRequest {
    pub items: Vec<CreateVinyl>,
}

impl Vinyl {
    /// List all vinyls with optional search and metadata status filtering.
    pub async fn list(
        pool: &SqlitePool,
        search: Option<String>,
        missing_metadata_only: bool,
        genre_filter: Option<String>,
        sort: Option<String>,
    ) -> Result<Vec<Self>> {
        let mut vinyls = if let Some(search_term) = search {
            let trimmed = search_term.trim().to_lowercase();
            if trimmed.is_empty() {
                // Empty search, fetch all
                sqlx::query_as::<_, Vinyl>(
                    r#"
                    SELECT id, artist, title, release_year, genre, notes, cover_image_url, created_at, updated_at,
                           metadata_status, metadata_source, metadata_source_id, metadata_source_url,
                           metadata_candidates, metadata_error, metadata_checked_at
                    FROM vinyls
                    ORDER BY LOWER(artist), LOWER(title)
                    "#,
                )
                .fetch_all(pool)
                .await?
            } else {
                // Normalize search: lowercase, trim, and also compare without whitespace.
                let normalized = format!("%{}%", trimmed);
                let compact_search: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
                let compact_normalized = format!("%{}%", compact_search);

                sqlx::query_as::<_, Vinyl>(
                    r#"
                    SELECT id, artist, title, release_year, genre, notes, cover_image_url, created_at, updated_at,
                           metadata_status, metadata_source, metadata_source_id, metadata_source_url,
                           metadata_candidates, metadata_error, metadata_checked_at
                    FROM vinyls
                    WHERE LOWER(artist) LIKE ?1
                       OR LOWER(title) LIKE ?1
                       OR LOWER(genre) LIKE ?1
                       OR REPLACE(LOWER(artist), ' ', '') LIKE ?2
                       OR REPLACE(LOWER(title), ' ', '') LIKE ?2
                       OR REPLACE(LOWER(genre), ' ', '') LIKE ?2
                    ORDER BY LOWER(artist), LOWER(title)
                    "#,
                )
                .bind(&normalized)
                .bind(&compact_normalized)
                .fetch_all(pool)
                .await?
            }
        } else {
            sqlx::query_as::<_, Vinyl>(
                r#"
                SELECT id, artist, title, release_year, genre, notes, cover_image_url, created_at, updated_at,
                       metadata_status, metadata_source, metadata_source_id, metadata_source_url,
                       metadata_candidates, metadata_error, metadata_checked_at
                FROM vinyls
                ORDER BY LOWER(artist), LOWER(title)
                "#,
            )
            .fetch_all(pool)
            .await?
        };

        if missing_metadata_only {
            vinyls.retain(|vinyl| {
                (vinyl.metadata_status != "complete" && vinyl.metadata_status != "disabled")
                    || vinyl
                        .genre
                        .as_deref()
                        .map(str::trim)
                        .unwrap_or_default()
                        .is_empty()
            });
        }

        if let Some(genre_filter) = genre_filter.as_deref().map(str::trim).filter(|value| !value.is_empty()) {
            vinyls.retain(|vinyl| {
                vinyl
                    .genre
                    .as_deref()
                    .is_some_and(|genre| genre.eq_ignore_ascii_case(genre_filter))
            });
        }

        sort_vinyls(&mut vinyls, sort.as_deref());

        Ok(vinyls)
    }

    /// Get a vinyl by ID
    pub async fn get(pool: &SqlitePool, id: &str) -> Result<Self> {
        let vinyl = sqlx::query_as::<_, Vinyl>(
            r#"
            SELECT id, artist, title, release_year, genre, notes, cover_image_url, created_at, updated_at,
                   metadata_status, metadata_source, metadata_source_id, metadata_source_url,
                   metadata_candidates, metadata_error, metadata_checked_at
            FROM vinyls
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)?;

        Ok(vinyl)
    }

    /// Create a new vinyl
    pub async fn create(pool: &SqlitePool, input: CreateVinyl) -> Result<Self> {
        let artist = input.artist.trim().to_string();
        let title = input.title.trim().to_string();

        if artist.is_empty() || title.is_empty() {
            return Err(AppError::InvalidInput(
                "artist and title are required".to_string(),
            ));
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let metadata_status = "pending".to_string();

        sqlx::query(
            r#"
            INSERT INTO vinyls (
                id, artist, title, release_year, genre, notes, cover_image_url, created_at, updated_at, metadata_status
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
        )
        .bind(&id)
        .bind(&artist)
        .bind(&title)
        .bind(input.release_year)
        .bind(&input.genre)
        .bind(&input.notes)
        .bind(&input.cover_image_url)
        .bind(now)
        .bind(now)
        .bind(&metadata_status)
        .execute(pool)
        .await?;

        Ok(Self {
            id,
            artist,
            title,
            release_year: input.release_year,
            genre: input.genre,
            notes: input.notes,
            cover_image_url: input.cover_image_url,
            created_at: now,
            updated_at: now,
            metadata_status,
            metadata_source: None,
            metadata_source_id: None,
            metadata_source_url: None,
            metadata_candidates: None,
            metadata_error: None,
            metadata_checked_at: None,
        })
    }

    /// Update a vinyl
    pub async fn update(pool: &SqlitePool, id: &str, input: UpdateVinyl) -> Result<Self> {
        let existing = Self::get(pool, id).await?;

        let artist = match input.artist {
            PatchField::Missing => existing.artist.clone(),
            PatchField::Null => {
                return Err(AppError::InvalidInput(
                    "artist and title are required".to_string(),
                ))
            }
            PatchField::Value(value) => {
                let trimmed = value.trim().to_string();
                if trimmed.is_empty() {
                    return Err(AppError::InvalidInput(
                        "artist and title are required".to_string(),
                    ));
                }
                trimmed
            }
        };

        let title = match input.title {
            PatchField::Missing => existing.title.clone(),
            PatchField::Null => {
                return Err(AppError::InvalidInput(
                    "artist and title are required".to_string(),
                ))
            }
            PatchField::Value(value) => {
                let trimmed = value.trim().to_string();
                if trimmed.is_empty() {
                    return Err(AppError::InvalidInput(
                        "artist and title are required".to_string(),
                    ));
                }
                trimmed
            }
        };

        let release_year = match input.release_year {
            PatchField::Missing => existing.release_year,
            PatchField::Null => None,
            PatchField::Value(value) => Some(value),
        };
        let genre = match input.genre {
            PatchField::Missing => existing.genre.clone(),
            PatchField::Null => None,
            PatchField::Value(value) => {
                let trimmed = value.trim().to_string();
                (!trimmed.is_empty()).then_some(trimmed)
            }
        };
        let notes = match input.notes {
            PatchField::Missing => existing.notes.clone(),
            PatchField::Null => None,
            PatchField::Value(value) => {
                let trimmed = value.trim().to_string();
                (!trimmed.is_empty()).then_some(trimmed)
            }
        };
        let cover_image_url = match input.cover_image_url {
            PatchField::Missing => existing.cover_image_url.clone(),
            PatchField::Null => None,
            PatchField::Value(value) => {
                let trimmed = value.trim().to_string();
                (!trimmed.is_empty()).then_some(trimmed)
            }
        };

        let album_identity_changed = artist != existing.artist || title != existing.title;
        let (
            metadata_status,
            metadata_source,
            metadata_source_id,
            metadata_source_url,
            metadata_candidates,
            metadata_error,
            metadata_checked_at,
        ) = if album_identity_changed {
            ("pending".to_string(), None, None, None, None, None, None)
        } else {
            (
                existing.metadata_status.clone(),
                existing.metadata_source.clone(),
                existing.metadata_source_id.clone(),
                existing.metadata_source_url.clone(),
                existing.metadata_candidates.clone(),
                existing.metadata_error.clone(),
                existing.metadata_checked_at,
            )
        };

        let updated_at = Utc::now();

        sqlx::query(
            r#"
            UPDATE vinyls
            SET artist = ?1,
                title = ?2,
                release_year = ?3,
                genre = ?4,
                notes = ?5,
                cover_image_url = ?6,
                metadata_status = ?7,
                metadata_source = ?8,
                metadata_source_id = ?9,
                metadata_source_url = ?10,
                metadata_candidates = ?11,
                metadata_error = ?12,
                metadata_checked_at = ?13,
                updated_at = ?14
            WHERE id = ?15
            "#,
        )
        .bind(&artist)
        .bind(&title)
        .bind(release_year)
        .bind(&genre)
        .bind(&notes)
        .bind(&cover_image_url)
        .bind(&metadata_status)
        .bind(&metadata_source)
        .bind(&metadata_source_id)
        .bind(&metadata_source_url)
        .bind(&metadata_candidates)
        .bind(&metadata_error)
        .bind(metadata_checked_at)
        .bind(updated_at)
        .bind(id)
        .execute(pool)
        .await?;

        Self::get(pool, id).await
    }

    /// Update fields populated by album metadata enrichment.
    pub async fn update_metadata(pool: &SqlitePool, id: &str, input: MetadataUpdate) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE vinyls
            SET release_year = COALESCE(?1, release_year),
                genre = COALESCE(?2, genre),
                notes = COALESCE(?3, notes),
                cover_image_url = COALESCE(?4, cover_image_url),
                metadata_status = ?5,
                metadata_source = ?6,
                metadata_source_id = ?7,
                metadata_source_url = ?8,
                metadata_candidates = ?9,
                metadata_error = ?10,
                metadata_checked_at = ?11,
                updated_at = ?12
            WHERE id = ?13
            "#,
        )
        .bind(input.release_year)
        .bind(&input.genre)
        .bind(&input.notes)
        .bind(&input.cover_image_url)
        .bind(&input.metadata_status)
        .bind(&input.metadata_source)
        .bind(&input.metadata_source_id)
        .bind(&input.metadata_source_url)
        .bind(&input.metadata_candidates)
        .bind(&input.metadata_error)
        .bind(input.metadata_checked_at)
        .bind(Utc::now())
        .bind(id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }

        Ok(())
    }

    /// Update only the stored cover image URL.
    pub async fn update_cover_image_url(
        pool: &SqlitePool,
        id: &str,
        cover_image_url: &str,
    ) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE vinyls
            SET cover_image_url = ?1,
                updated_at = ?2
            WHERE id = ?3
            "#,
        )
        .bind(cover_image_url)
        .bind(Utc::now())
        .bind(id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }

        Ok(())
    }

    /// Existing external cover URLs that can be downloaded to the local upload cache.
    pub async fn list_external_cover_images(
        pool: &SqlitePool,
        limit: i64,
    ) -> Result<Vec<VinylCoverImage>> {
        let covers = sqlx::query_as::<_, VinylCoverImage>(
            r#"
            SELECT id, cover_image_url, metadata_source_id
            FROM vinyls
            WHERE cover_image_url IS NOT NULL
              AND (cover_image_url LIKE 'https://%' OR cover_image_url LIKE 'http://%')
            ORDER BY created_at
            LIMIT ?1
            "#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(covers)
    }

    /// IDs for vinyls that need a best-effort metadata lookup.
    pub async fn list_requiring_metadata(pool: &SqlitePool, limit: i64) -> Result<Vec<String>> {
        let ids = sqlx::query_scalar::<_, String>(
            r#"
            SELECT id
            FROM vinyls
            WHERE metadata_checked_at IS NULL
               OR metadata_status IN ('pending', 'error', 'not_found')
               OR genre IS NULL
               OR TRIM(genre) = ''
            ORDER BY created_at
            LIMIT ?1
            "#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(ids)
    }

    /// Delete a vinyl
    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM vinyls WHERE id = ?1")
            .bind(id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }

        Ok(())
    }
}

fn sort_vinyls(vinyls: &mut [Vinyl], sort: Option<&str>) {
    match sort.unwrap_or_default().trim().to_ascii_lowercase().as_str() {
        "date" | "release_date" | "release_year" => vinyls.sort_by(|left, right| {
            right
                .release_year
                .unwrap_or(i32::MIN)
                .cmp(&left.release_year.unwrap_or(i32::MIN))
                .then_with(|| compare_artist_title(left, right))
        }),
        "last_edit" | "updated_at" => vinyls.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| compare_artist_title(left, right))
        }),
        "genre" => vinyls.sort_by(|left, right| {
            let left_genre = left.genre.as_deref().map(str::to_lowercase);
            let right_genre = right.genre.as_deref().map(str::to_lowercase);

            left_genre
                .is_none()
                .cmp(&right_genre.is_none())
                .then_with(|| left_genre.cmp(&right_genre))
                .then_with(|| compare_artist_title(left, right))
        }),
        _ => vinyls.sort_by(compare_artist_title),
    }
}

fn compare_artist_title(left: &Vinyl, right: &Vinyl) -> std::cmp::Ordering {
    left.artist
        .to_lowercase()
        .cmp(&right.artist.to_lowercase())
        .then_with(|| left.title.to_lowercase().cmp(&right.title.to_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        sqlx::query(
            r#"
            CREATE TABLE vinyls (
                id TEXT PRIMARY KEY,
                artist TEXT NOT NULL,
                title TEXT NOT NULL,
                release_year INTEGER,
                genre TEXT,
                notes TEXT,
                cover_image_url TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                metadata_status TEXT NOT NULL DEFAULT 'pending',
                metadata_source TEXT,
                metadata_source_id TEXT,
                metadata_source_url TEXT,
                metadata_candidates TEXT,
                metadata_error TEXT,
                metadata_checked_at TEXT
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_create_and_get_vinyl() {
        let pool = setup_test_db().await;

        let input = CreateVinyl {
            artist: "Pink Floyd".to_string(),
            title: "The Dark Side of the Moon".to_string(),
            release_year: Some(1973),
            genre: None,
            notes: None,
            cover_image_url: None,
        };

        let vinyl = Vinyl::create(&pool, input).await.unwrap();
        assert_eq!(vinyl.artist, "Pink Floyd");
        assert_eq!(vinyl.title, "The Dark Side of the Moon");
        assert_eq!(vinyl.metadata_status, "pending");

        let fetched = Vinyl::get(&pool, &vinyl.id).await.unwrap();
        assert_eq!(fetched.id, vinyl.id);
    }

    #[tokio::test]
    async fn test_create_vinyl_accepts_bulk_aliases() {
        let input: CreateVinyl = serde_json::from_value(serde_json::json!({
            "artist": "The Beatles",
            "title": "Abbey Road",
            "year": 1969,
            "cover_url": "https://example.com/cover.jpg"
        }))
        .unwrap();

        assert_eq!(input.release_year, Some(1969));
        assert_eq!(
            input.cover_image_url.as_deref(),
            Some("https://example.com/cover.jpg")
        );
    }

    #[tokio::test]
    async fn test_search_case_insensitive() {
        let pool = setup_test_db().await;

        Vinyl::create(
            &pool,
            CreateVinyl {
                artist: "The Beatles".to_string(),
                title: "Abbey Road".to_string(),
                release_year: Some(1969),
                genre: None,
                notes: None,
                cover_image_url: None,
            },
        )
        .await
        .unwrap();

        Vinyl::create(
            &pool,
            CreateVinyl {
                artist: "Pink Floyd".to_string(),
                title: "The Wall".to_string(),
                release_year: Some(1979),
                genre: None,
                notes: None,
                cover_image_url: None,
            },
        )
        .await
        .unwrap();

        // Search lowercase
        let results = Vinyl::list(&pool, Some("beatles".to_string()), false, None, None).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].artist, "The Beatles");

        // Search uppercase
        let results = Vinyl::list(&pool, Some("PINK".to_string()), false, None, None).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].artist, "Pink Floyd");

        // Search title
        let results = Vinyl::list(&pool, Some("wall".to_string()), false, None, None).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "The Wall");

        // Search with whitespace
        let results = Vinyl::list(&pool, Some("  floyd  ".to_string()), false, None, None).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_list_sorted() {
        let pool = setup_test_db().await;

        Vinyl::create(
            &pool,
            CreateVinyl {
                artist: "Zeppelin".to_string(),
                title: "IV".to_string(),
                release_year: None,
                genre: None,
                notes: None,
                cover_image_url: None,
            },
        )
        .await
        .unwrap();

        Vinyl::create(
            &pool,
            CreateVinyl {
                artist: "Beatles".to_string(),
                title: "White Album".to_string(),
                release_year: None,
                genre: None,
                notes: None,
                cover_image_url: None,
            },
        )
        .await
        .unwrap();

        let results = Vinyl::list(&pool, None, false, None, None).await.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].artist, "Beatles");
        assert_eq!(results[1].artist, "Zeppelin");
    }

    #[tokio::test]
    async fn test_list_filters_and_sorts() {
        let pool = setup_test_db().await;

        let jazz = Vinyl::create(
            &pool,
            CreateVinyl {
                artist: "Miles Davis".to_string(),
                title: "Kind of Blue".to_string(),
                release_year: Some(1959),
                genre: Some("Jazz".to_string()),
                notes: None,
                cover_image_url: None,
            },
        )
        .await
        .unwrap();

        let rock = Vinyl::create(
            &pool,
            CreateVinyl {
                artist: "Pink Floyd".to_string(),
                title: "The Wall".to_string(),
                release_year: Some(1979),
                genre: Some("Rock".to_string()),
                notes: None,
                cover_image_url: None,
            },
        )
        .await
        .unwrap();

        let rap = Vinyl::create(
            &pool,
            CreateVinyl {
                artist: "IAM".to_string(),
                title: "L'école du micro d'argent".to_string(),
                release_year: Some(1997),
                genre: Some("Rap".to_string()),
                notes: None,
                cover_image_url: None,
            },
        )
        .await
        .unwrap();

        let by_date = Vinyl::list(&pool, None, false, None, Some("date".to_string()))
            .await
            .unwrap();
        assert_eq!(
            by_date.iter().map(|vinyl| vinyl.id.as_str()).collect::<Vec<_>>(),
            vec![rap.id.as_str(), rock.id.as_str(), jazz.id.as_str()]
        );

        let by_genre = Vinyl::list(&pool, None, false, None, Some("genre".to_string()))
            .await
            .unwrap();
        assert_eq!(
            by_genre
                .iter()
                .map(|vinyl| vinyl.genre.as_deref().unwrap_or_default())
                .collect::<Vec<_>>(),
            vec!["Jazz", "Rap", "Rock"]
        );

        let only_rock = Vinyl::list(&pool, None, false, Some("rock".to_string()), None)
            .await
            .unwrap();
        assert_eq!(only_rock.len(), 1);
        assert_eq!(only_rock[0].id, rock.id);

        Vinyl::update(
            &pool,
            &jazz.id,
            UpdateVinyl {
                artist: PatchField::Missing,
                title: PatchField::Missing,
                release_year: PatchField::Missing,
                genre: PatchField::Missing,
                notes: PatchField::Value("Recently edited".to_string()),
                cover_image_url: PatchField::Missing,
            },
        )
        .await
        .unwrap();
        let by_last_edit = Vinyl::list(&pool, None, false, None, Some("last_edit".to_string()))
            .await
            .unwrap();
        assert_eq!(by_last_edit[0].id, jazz.id);
    }

    #[tokio::test]
    async fn test_list_missing_metadata_only() {
        let pool = setup_test_db().await;

        let pending = Vinyl::create(
            &pool,
            CreateVinyl {
                artist: "Pending Artist".to_string(),
                title: "Pending Album".to_string(),
                release_year: None,
                genre: None,
                notes: None,
                cover_image_url: None,
            },
        )
        .await
        .unwrap();

        let complete = Vinyl::create(
            &pool,
            CreateVinyl {
                artist: "Complete Artist".to_string(),
                title: "Complete Album".to_string(),
                release_year: None,
                genre: None,
                notes: None,
                cover_image_url: None,
            },
        )
        .await
        .unwrap();

        Vinyl::update_metadata(
            &pool,
            &complete.id,
            MetadataUpdate {
                release_year: None,
                genre: Some("Rock".to_string()),
                notes: None,
                cover_image_url: None,
                metadata_status: "complete".to_string(),
                metadata_source: Some("musicbrainz".to_string()),
                metadata_source_id: Some("mbid".to_string()),
                metadata_source_url: Some("https://musicbrainz.org/release-group/mbid".to_string()),
                metadata_candidates: None,
                metadata_error: None,
                metadata_checked_at: Some(Utc::now()),
            },
        )
        .await
        .unwrap();

        let results = Vinyl::list(&pool, None, true, None, None).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, pending.id);
    }

    #[tokio::test]
    async fn test_update_vinyl() {
        let pool = setup_test_db().await;

        let vinyl = Vinyl::create(
            &pool,
            CreateVinyl {
                artist: "Test".to_string(),
                title: "Album".to_string(),
                release_year: None,
                genre: None,
                notes: None,
                cover_image_url: None,
            },
        )
        .await
        .unwrap();

        let updated = Vinyl::update(
            &pool,
            &vinyl.id,
            UpdateVinyl {
                artist: PatchField::Value("Updated Artist".to_string()),
                title: PatchField::Missing,
                release_year: PatchField::Value(2020),
                genre: PatchField::Missing,
                notes: PatchField::Missing,
                cover_image_url: PatchField::Missing,
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.artist, "Updated Artist");
        assert_eq!(updated.title, "Album"); // Unchanged
        assert_eq!(updated.release_year, Some(2020));
        assert_eq!(updated.metadata_status, "pending");
    }

    #[tokio::test]
    async fn test_update_vinyl_clears_nullable_fields() {
        let pool = setup_test_db().await;

        let vinyl = Vinyl::create(
            &pool,
            CreateVinyl {
                artist: "Test".to_string(),
                title: "Album".to_string(),
                release_year: Some(1999),
                genre: Some("Rock".to_string()),
                notes: Some("Original notes".to_string()),
                cover_image_url: Some("https://example.com/cover.jpg".to_string()),
            },
        )
        .await
        .unwrap();

        let updated = Vinyl::update(
            &pool,
            &vinyl.id,
            UpdateVinyl {
                artist: PatchField::Missing,
                title: PatchField::Missing,
                release_year: PatchField::Null,
                genre: PatchField::Null,
                notes: PatchField::Null,
                cover_image_url: PatchField::Value("".to_string()),
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.release_year, None);
        assert_eq!(updated.genre, None);
        assert_eq!(updated.notes, None);
        assert_eq!(updated.cover_image_url, None);
    }

    #[tokio::test]
    async fn test_update_vinyl_accepts_json_nulls() {
        let input: UpdateVinyl = serde_json::from_value(serde_json::json!({
            "release_year": null,
            "genre": null,
            "notes": null,
            "cover_image_url": null
        }))
        .unwrap();

        assert_eq!(input.release_year, PatchField::Null);
        assert_eq!(input.genre, PatchField::Null);
        assert_eq!(input.notes, PatchField::Null);
        assert_eq!(input.cover_image_url, PatchField::Null);
        assert_eq!(input.artist, PatchField::Missing);
    }

    #[tokio::test]
    async fn test_update_metadata() {
        let pool = setup_test_db().await;
        let vinyl = Vinyl::create(
            &pool,
            CreateVinyl {
                artist: "Test".to_string(),
                title: "Album".to_string(),
                release_year: None,
                genre: None,
                notes: None,
                cover_image_url: None,
            },
        )
        .await
        .unwrap();

        Vinyl::update_metadata(
            &pool,
            &vinyl.id,
            MetadataUpdate {
                release_year: Some(1970),
                genre: Some("Rock".to_string()),
                notes: Some("Metadata: https://example.com".to_string()),
                cover_image_url: Some("https://example.com/cover.jpg".to_string()),
                metadata_status: "complete".to_string(),
                metadata_source: Some("musicbrainz".to_string()),
                metadata_source_id: Some("mbid".to_string()),
                metadata_source_url: Some("https://musicbrainz.org/release-group/mbid".to_string()),
                metadata_candidates: None,
                metadata_error: None,
                metadata_checked_at: Some(Utc::now()),
            },
        )
        .await
        .unwrap();

        let updated = Vinyl::get(&pool, &vinyl.id).await.unwrap();
        assert_eq!(updated.release_year, Some(1970));
        assert_eq!(updated.genre.as_deref(), Some("Rock"));
        assert_eq!(updated.metadata_status, "complete");
        assert_eq!(updated.metadata_source_id.as_deref(), Some("mbid"));
    }

    #[tokio::test]
    async fn test_list_requiring_metadata() {
        let pool = setup_test_db().await;
        let pending = Vinyl::create(
            &pool,
            CreateVinyl {
                artist: "Test".to_string(),
                title: "Pending".to_string(),
                release_year: None,
                genre: None,
                notes: None,
                cover_image_url: None,
            },
        )
        .await
        .unwrap();

        let complete = Vinyl::create(
            &pool,
            CreateVinyl {
                artist: "Test".to_string(),
                title: "Complete".to_string(),
                release_year: None,
                genre: None,
                notes: None,
                cover_image_url: None,
            },
        )
        .await
        .unwrap();
        Vinyl::update_metadata(
            &pool,
            &complete.id,
            MetadataUpdate {
                release_year: None,
                genre: Some("Rock".to_string()),
                notes: None,
                cover_image_url: None,
                metadata_status: "complete".to_string(),
                metadata_source: Some("musicbrainz".to_string()),
                metadata_source_id: Some("mbid".to_string()),
                metadata_source_url: Some("https://musicbrainz.org/release-group/mbid".to_string()),
                metadata_candidates: None,
                metadata_error: None,
                metadata_checked_at: Some(Utc::now()),
            },
        )
        .await
        .unwrap();

        let complete_missing_genre = Vinyl::create(
            &pool,
            CreateVinyl {
                artist: "Test".to_string(),
                title: "Complete Missing Genre".to_string(),
                release_year: None,
                genre: None,
                notes: None,
                cover_image_url: None,
            },
        )
        .await
        .unwrap();
        Vinyl::update_metadata(
            &pool,
            &complete_missing_genre.id,
            MetadataUpdate {
                release_year: None,
                genre: None,
                notes: None,
                cover_image_url: None,
                metadata_status: "complete".to_string(),
                metadata_source: Some("musicbrainz".to_string()),
                metadata_source_id: Some("missing-genre".to_string()),
                metadata_source_url: Some("https://musicbrainz.org/release-group/missing-genre".to_string()),
                metadata_candidates: None,
                metadata_error: None,
                metadata_checked_at: Some(Utc::now()),
            },
        )
        .await
        .unwrap();

        let ids = Vinyl::list_requiring_metadata(&pool, 10).await.unwrap();
        assert_eq!(ids, vec![pending.id, complete_missing_genre.id]);
    }

    #[tokio::test]
    async fn test_delete_vinyl() {
        let pool = setup_test_db().await;

        let vinyl = Vinyl::create(
            &pool,
            CreateVinyl {
                artist: "Test".to_string(),
                title: "Album".to_string(),
                release_year: None,
                genre: None,
                notes: None,
                cover_image_url: None,
            },
        )
        .await
        .unwrap();

        Vinyl::delete(&pool, &vinyl.id).await.unwrap();

        let result = Vinyl::get(&pool, &vinyl.id).await;
        assert!(result.is_err());
    }
}
