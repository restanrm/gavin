-- Store internet metadata lookup status and choices for vinyl albums.
ALTER TABLE vinyls ADD COLUMN metadata_status TEXT NOT NULL DEFAULT 'pending';
ALTER TABLE vinyls ADD COLUMN metadata_source TEXT;
ALTER TABLE vinyls ADD COLUMN metadata_source_id TEXT;
ALTER TABLE vinyls ADD COLUMN metadata_source_url TEXT;
ALTER TABLE vinyls ADD COLUMN metadata_candidates TEXT;
ALTER TABLE vinyls ADD COLUMN metadata_error TEXT;
ALTER TABLE vinyls ADD COLUMN metadata_checked_at TEXT;

CREATE INDEX IF NOT EXISTS idx_vinyls_metadata_status ON vinyls(metadata_status);
CREATE INDEX IF NOT EXISTS idx_vinyls_metadata_checked_at ON vinyls(metadata_checked_at);
