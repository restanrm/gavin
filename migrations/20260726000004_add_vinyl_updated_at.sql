-- Track the last time a vinyl row was edited or enriched.
ALTER TABLE vinyls ADD COLUMN updated_at TEXT;

UPDATE vinyls
SET updated_at = created_at
WHERE updated_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_vinyls_updated_at ON vinyls(updated_at);
