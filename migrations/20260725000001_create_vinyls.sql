-- Create vinyls table
CREATE TABLE IF NOT EXISTS vinyls (
    id TEXT PRIMARY KEY,
    artist TEXT NOT NULL,
    title TEXT NOT NULL,
    release_year INTEGER,
    notes TEXT,
    cover_image_url TEXT,
    created_at TEXT NOT NULL
);

-- Create indexes for search performance
CREATE INDEX IF NOT EXISTS idx_vinyls_artist ON vinyls(artist);
CREATE INDEX IF NOT EXISTS idx_vinyls_title ON vinyls(title);
CREATE INDEX IF NOT EXISTS idx_vinyls_created_at ON vinyls(created_at);
