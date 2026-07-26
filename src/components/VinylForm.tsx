import { useState } from 'react';
import { createVinyl } from '../utils/api';
import { parseGenreInput } from '../utils/genres';

interface VinylFormProps {
  onSuccess: () => void;
}

export function VinylForm({ onSuccess }: VinylFormProps) {
  const [artist, setArtist] = useState('');
  const [title, setTitle] = useState('');
  const [releaseYear, setReleaseYear] = useState('');
  const [genre, setGenre] = useState('');
  const [notes, setNotes] = useState('');
  const [coverImageUrl, setCoverImageUrl] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!artist.trim() || !title.trim()) {
      setError('Artist and title are required');
      return;
    }

    setSubmitting(true);
    setError(null);

    try {
      const vinyl = {
        artist: artist.trim(),
        title: title.trim(),
        ...(releaseYear && { release_year: parseInt(releaseYear, 10) }),
        ...(genre.trim() && { genre: parseGenreInput(genre) }),
        ...(notes.trim() && { notes: notes.trim() }),
        ...(coverImageUrl.trim() && { cover_image_url: coverImageUrl.trim() }),
      };

      await createVinyl(vinyl);

      // Reset form
      setArtist('');
      setTitle('');
      setReleaseYear('');
      setGenre('');
      setNotes('');
      setCoverImageUrl('');

      onSuccess();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to add vinyl');
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="vinyl-form">
      <h3>Add Manually</h3>
      <p className="help-text">
        Use this form when you already know the artist and album title. To detect those automatically from an image, use <strong>Import from Album Cover</strong>.
      </p>

      {error && (
        <div className="error-message" role="alert">
          {error}
        </div>
      )}

      <div className="form-group">
        <label htmlFor="artist" className="form-label">
          Artist <span className="required">*</span>
        </label>
        <input
          id="artist"
          type="text"
          value={artist}
          onChange={(e) => setArtist(e.target.value)}
          required
          disabled={submitting}
          className="form-input"
          aria-required="true"
        />
      </div>

      <div className="form-group">
        <label htmlFor="title" className="form-label">
          Title <span className="required">*</span>
        </label>
        <input
          id="title"
          type="text"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          required
          disabled={submitting}
          className="form-input"
          aria-required="true"
        />
      </div>

      <div className="form-group">
        <label htmlFor="year" className="form-label">
          Release Year
        </label>
        <input
          id="year"
          type="number"
          value={releaseYear}
          onChange={(e) => setReleaseYear(e.target.value)}
          min="1900"
          max={new Date().getFullYear()}
          disabled={submitting}
          className="form-input"
        />
      </div>

      <div className="form-group">
        <label htmlFor="genre" className="form-label">
          Genre
        </label>
        <input
          id="genre"
          type="text"
          value={genre}
          onChange={(e) => setGenre(e.target.value)}
          disabled={submitting}
          className="form-input"
          placeholder="Rap, R&B, Rock…"
        />
      </div>

      <div className="form-group">
        <label htmlFor="notes" className="form-label">
          Notes
        </label>
        <textarea
          id="notes"
          value={notes}
          onChange={(e) => setNotes(e.target.value)}
          disabled={submitting}
          className="form-textarea"
          rows={3}
        />
      </div>

      <details className="manual-cover-details">
        <summary>Optional manual cover URL</summary>
        <div className="form-group">
          <label htmlFor="cover-url" className="form-label">
            Cover Image URL
          </label>
          <input
            id="cover-url"
            type="url"
            value={coverImageUrl}
            onChange={(e) => setCoverImageUrl(e.target.value)}
            disabled={submitting}
            className="form-input"
            placeholder="https://example.com/cover.jpg"
          />
        </div>
      </details>

      <button
        type="submit"
        disabled={submitting}
        className="btn btn-primary"
      >
        {submitting ? 'Adding...' : 'Add Vinyl'}
      </button>
    </form>
  );
}
