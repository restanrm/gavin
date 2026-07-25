//! Database models and operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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
    pub notes: Option<String>,
    pub cover_image_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Input for creating a new vinyl
#[derive(Debug, Deserialize)]
pub struct CreateVinyl {
    pub artist: String,
    pub title: String,
    pub release_year: Option<i32>,
    pub notes: Option<String>,
    pub cover_image_url: Option<String>,
}

/// Input for updating a vinyl
#[derive(Debug, Deserialize)]
pub struct UpdateVinyl {
    pub artist: Option<String>,
    pub title: Option<String>,
    pub release_year: Option<i32>,
    pub notes: Option<String>,
    pub cover_image_url: Option<String>,
}

/// Bulk create request
#[derive(Debug, Deserialize)]
pub struct BulkCreateRequest {
    pub items: Vec<CreateVinyl>,
}

impl Vinyl {
    /// List all vinyls with optional search
    pub async fn list(pool: &SqlitePool, search: Option<String>) -> Result<Vec<Self>> {
        let vinyls = if let Some(search_term) = search {
            let trimmed = search_term.trim().to_lowercase();
            if trimmed.is_empty() {
                // Empty search, fetch all
                sqlx::query_as::<_, Vinyl>(
                    r#"
                    SELECT id, artist, title, release_year, notes, cover_image_url, created_at
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
                    SELECT id, artist, title, release_year, notes, cover_image_url, created_at
                    FROM vinyls
                    WHERE LOWER(artist) LIKE ?1
                       OR LOWER(title) LIKE ?1
                       OR REPLACE(LOWER(artist), ' ', '') LIKE ?2
                       OR REPLACE(LOWER(title), ' ', '') LIKE ?2
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
                SELECT id, artist, title, release_year, notes, cover_image_url, created_at
                FROM vinyls
                ORDER BY LOWER(artist), LOWER(title)
                "#,
            )
            .fetch_all(pool)
            .await?
        };

        Ok(vinyls)
    }

    /// Get a vinyl by ID
    pub async fn get(pool: &SqlitePool, id: &str) -> Result<Self> {
        let vinyl = sqlx::query_as::<_, Vinyl>(
            r#"
            SELECT id, artist, title, release_year, notes, cover_image_url, created_at
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

        sqlx::query(
            r#"
            INSERT INTO vinyls (id, artist, title, release_year, notes, cover_image_url, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
        )
        .bind(&id)
        .bind(&artist)
        .bind(&title)
        .bind(input.release_year)
        .bind(&input.notes)
        .bind(&input.cover_image_url)
        .bind(now)
        .execute(pool)
        .await?;

        Ok(Self {
            id,
            artist,
            title,
            release_year: input.release_year,
            notes: input.notes,
            cover_image_url: input.cover_image_url,
            created_at: now,
        })
    }

    /// Update a vinyl
    pub async fn update(pool: &SqlitePool, id: &str, input: UpdateVinyl) -> Result<Self> {
        // First check if exists
        let _existing = Self::get(pool, id).await?;

        sqlx::query(
            r#"
            UPDATE vinyls
            SET artist = COALESCE(?1, artist),
                title = COALESCE(?2, title),
                release_year = COALESCE(?3, release_year),
                notes = COALESCE(?4, notes),
                cover_image_url = COALESCE(?5, cover_image_url)
            WHERE id = ?6
            "#,
        )
        .bind(&input.artist)
        .bind(&input.title)
        .bind(input.release_year)
        .bind(&input.notes)
        .bind(&input.cover_image_url)
        .bind(id)
        .execute(pool)
        .await?;

        // Fetch updated record
        Self::get(pool, id).await
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
                notes TEXT,
                cover_image_url TEXT,
                created_at TEXT NOT NULL
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
            notes: None,
            cover_image_url: None,
        };

        let vinyl = Vinyl::create(&pool, input).await.unwrap();
        assert_eq!(vinyl.artist, "Pink Floyd");
        assert_eq!(vinyl.title, "The Dark Side of the Moon");

        let fetched = Vinyl::get(&pool, &vinyl.id).await.unwrap();
        assert_eq!(fetched.id, vinyl.id);
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
                notes: None,
                cover_image_url: None,
            },
        )
        .await
        .unwrap();

        // Search lowercase
        let results = Vinyl::list(&pool, Some("beatles".to_string())).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].artist, "The Beatles");

        // Search uppercase
        let results = Vinyl::list(&pool, Some("PINK".to_string())).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].artist, "Pink Floyd");

        // Search title
        let results = Vinyl::list(&pool, Some("wall".to_string())).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "The Wall");

        // Search with whitespace
        let results = Vinyl::list(&pool, Some("  floyd  ".to_string())).await.unwrap();
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
                notes: None,
                cover_image_url: None,
            },
        )
        .await
        .unwrap();

        let results = Vinyl::list(&pool, None).await.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].artist, "Beatles");
        assert_eq!(results[1].artist, "Zeppelin");
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
                artist: Some("Updated Artist".to_string()),
                title: None,
                release_year: Some(2020),
                notes: None,
                cover_image_url: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.artist, "Updated Artist");
        assert_eq!(updated.title, "Album"); // Unchanged
        assert_eq!(updated.release_year, Some(2020));
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
