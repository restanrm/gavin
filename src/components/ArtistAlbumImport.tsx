import { useState } from 'react';
import { createVinylFromCandidate, searchArtistAlbums } from '../utils/api';
import type { AlbumCandidate } from '../types';

interface ArtistAlbumImportProps {
  onSuccess: () => void;
}

type AddedAlbum = {
  artist: string;
  title: string;
};

export function ArtistAlbumImport({ onSuccess }: ArtistAlbumImportProps) {
  const [artist, setArtist] = useState('');
  const [results, setResults] = useState<AlbumCandidate[]>([]);
  const [searching, setSearching] = useState(false);
  const [addingId, setAddingId] = useState<string | null>(null);
  const [addedIds, setAddedIds] = useState<Set<string>>(new Set());
  const [lastAdded, setLastAdded] = useState<AddedAlbum | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleSearch = async (event: React.FormEvent) => {
    event.preventDefault();

    if (!artist.trim()) {
      setError('Artist name is required');
      return;
    }

    setSearching(true);
    setError(null);
    setLastAdded(null);

    try {
      const albums = await searchArtistAlbums(artist.trim());
      setResults(albums);
      setAddedIds(new Set());
      if (albums.length === 0) {
        setError('No albums found for this artist');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Album search failed');
    } finally {
      setSearching(false);
    }
  };

  const handleAddAlbum = async (candidate: AlbumCandidate) => {
    setAddingId(candidate.id);
    setError(null);

    try {
      await createVinylFromCandidate(candidate);
      setAddedIds((current) => new Set(current).add(candidate.id));
      setLastAdded({ artist: candidate.artist, title: candidate.title });
      onSuccess();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to add album');
    } finally {
      setAddingId(null);
    }
  };

  return (
    <div className="artist-album-import">
      <h3>Add Albums by Artist</h3>
      <p className="help-text">
        Enter an artist name, then add one or more albums from the results. The list stays open after each add.
      </p>

      <form onSubmit={handleSearch} className="artist-search-form">
        <div className="form-group">
          <label htmlFor="artist-album-search" className="form-label">
            Artist name
          </label>
          <input
            id="artist-album-search"
            type="text"
            value={artist}
            onChange={(event) => setArtist(event.target.value)}
            disabled={searching}
            className="form-input"
            placeholder="The Beatles"
          />
        </div>

        <button
          type="submit"
          disabled={searching || !artist.trim()}
          className="btn btn-primary"
        >
          {searching ? 'Searching...' : 'Find Albums'}
        </button>
      </form>

      {lastAdded && (
        <div className="success-message" role="status">
          Added {lastAdded.artist} — {lastAdded.title}
        </div>
      )}

      {error && (
        <div className="error-message" role="alert">
          {error}
        </div>
      )}

      {results.length > 0 && (
        <div className="artist-album-results">
          <p className="results-count">{results.length} albums found</p>
          <ul className="album-result-list">
            {results.map((candidate) => {
              const added = addedIds.has(candidate.id);
              return (
                <li
                  key={candidate.id}
                  className={`album-result ${added ? 'album-result-added' : ''}`}
                >
                  {candidate.cover_image_url ? (
                    <img src={candidate.cover_image_url} alt="" loading="lazy" />
                  ) : (
                    <div className="album-result-placeholder" aria-hidden="true" />
                  )}
                  <div className="album-result-info">
                    <strong>{candidate.title}</strong>
                    <span>{candidate.artist}</span>
                    {candidate.release_year && <span>{candidate.release_year}</span>}
                    {candidate.genre && <span>{candidate.genre}</span>}
                    {candidate.disambiguation && <small>{candidate.disambiguation}</small>}
                  </div>
                  <button
                    type="button"
                    className="btn btn-secondary"
                    onClick={() => handleAddAlbum(candidate)}
                    disabled={addingId !== null}
                    aria-label={`Add ${candidate.title} by ${candidate.artist}`}
                  >
                    {addingId === candidate.id ? 'Adding...' : added ? 'Added ✓' : 'Add'}
                  </button>
                </li>
              );
            })}
          </ul>
        </div>
      )}
    </div>
  );
}
