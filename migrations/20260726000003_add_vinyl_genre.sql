-- Store a human-readable album music genre, either entered manually or enriched from metadata.
ALTER TABLE vinyls ADD COLUMN genre TEXT;

CREATE INDEX IF NOT EXISTS idx_vinyls_genre ON vinyls(genre);
