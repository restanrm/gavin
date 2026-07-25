import { useCallback, useEffect, useState, type FormEvent } from 'react';
import type { Vinyl } from '../types';
import { updateVinyl } from '../utils/api';
import { ImageUpload } from './ImageUpload';

interface VinylCardProps {
  vinyl: Vinyl;
  isAdmin?: boolean;
  onDelete?: (id: string) => void;
  onUpdate?: () => void;
}

interface AlbumCandidate {
  id: string;
  title: string;
  artist: string;
  release_year?: number;
  disambiguation?: string;
  source_url: string;
}

function parseCandidates(value?: string | null): AlbumCandidate[] {
  if (!value) {
    return [];
  }

  try {
    return JSON.parse(value) as AlbumCandidate[];
  } catch {
    return [];
  }
}

function isGeneratedMetadataNote(notes?: string | null): boolean {
  return notes?.trim().toLowerCase().startsWith('metadata:') ?? false;
}

function metadataLabel(status: Vinyl['metadata_status']): string {
  switch (status) {
    case 'pending':
      return 'Metadata pending';
    case 'needs_choice':
      return 'Metadata choice required';
    case 'not_found':
      return 'Metadata not found';
    case 'error':
      return 'Metadata error';
    case 'disabled':
      return 'Metadata disabled';
    case 'complete':
    default:
      return 'Metadata complete';
  }
}

function MetadataDetails({ vinyl }: { vinyl: Vinyl }) {
  const candidates = parseCandidates(vinyl.metadata_candidates);

  return (
    <div className="metadata-panel">
      <h4>Album metadata</h4>
      <div className={`metadata-badge metadata-${vinyl.metadata_status}`}>
        {metadataLabel(vinyl.metadata_status)}
      </div>
      {vinyl.metadata_source_url && (
        <p className="metadata-source">
          <a href={vinyl.metadata_source_url} target="_blank" rel="noreferrer">
            View metadata source
          </a>
        </p>
      )}
      {vinyl.metadata_checked_at && (
        <p className="metadata-checked">
          Last checked: {new Date(vinyl.metadata_checked_at).toLocaleString()}
        </p>
      )}
      {vinyl.metadata_status === 'needs_choice' && candidates.length > 0 && (
        <details className="metadata-candidates">
          <summary>Review possible album matches</summary>
          <ul>
            {candidates.map((candidate) => (
              <li key={candidate.id}>
                <a href={candidate.source_url} target="_blank" rel="noreferrer">
                  {candidate.artist} — {candidate.title}
                  {candidate.release_year ? ` (${candidate.release_year})` : ''}
                </a>
                {candidate.disambiguation ? ` — ${candidate.disambiguation}` : ''}
              </li>
            ))}
          </ul>
        </details>
      )}
      {vinyl.metadata_error && vinyl.metadata_status !== 'needs_choice' && (
        <p className="metadata-error">{vinyl.metadata_error}</p>
      )}
    </div>
  );
}

export function VinylCard({ vinyl, isAdmin = false, onDelete, onUpdate }: VinylCardProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [artist, setArtist] = useState(vinyl.artist);
  const [title, setTitle] = useState(vinyl.title);
  const [releaseYear, setReleaseYear] = useState(vinyl.release_year?.toString() ?? '');
  const [notes, setNotes] = useState(vinyl.notes ?? '');
  const [coverImageUrl, setCoverImageUrl] = useState(vinyl.cover_image_url ?? '');
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const resetForm = useCallback(() => {
    setArtist(vinyl.artist);
    setTitle(vinyl.title);
    setReleaseYear(vinyl.release_year?.toString() ?? '');
    setNotes(vinyl.notes ?? '');
    setCoverImageUrl(vinyl.cover_image_url ?? '');
    setError(null);
  }, [vinyl.artist, vinyl.title, vinyl.release_year, vinyl.notes, vinyl.cover_image_url]);

  const handleDelete = () => {
    if (window.confirm(`Delete "${vinyl.title}" by ${vinyl.artist}?`)) {
      onDelete?.(vinyl.id);
    }
  };

  const handleEdit = () => {
    resetForm();
    setIsEditing(true);
  };

  const handleCancel = useCallback(() => {
    resetForm();
    setIsEditing(false);
  }, [resetForm]);

  useEffect(() => {
    if (!isEditing) {
      return undefined;
    }

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        handleCancel();
      }
    };

    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    window.addEventListener('keydown', handleKeyDown);

    return () => {
      document.body.style.overflow = previousOverflow;
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [isEditing, handleCancel]);

  const handleSubmit = async (event: FormEvent) => {
    event.preventDefault();

    const trimmedArtist = artist.trim();
    const trimmedTitle = title.trim();
    const trimmedReleaseYear = releaseYear.trim();
    const trimmedNotes = notes.trim();
    const trimmedCoverImageUrl = coverImageUrl.trim();

    if (!trimmedArtist || !trimmedTitle) {
      setError('Artist and title are required');
      return;
    }

    const parsedReleaseYear = trimmedReleaseYear ? parseInt(trimmedReleaseYear, 10) : null;
    if (trimmedReleaseYear && Number.isNaN(parsedReleaseYear)) {
      setError('Release year must be a valid number');
      return;
    }

    setSubmitting(true);
    setError(null);

    try {
      await updateVinyl(vinyl.id, {
        artist: trimmedArtist,
        title: trimmedTitle,
        release_year: parsedReleaseYear,
        notes: trimmedNotes || null,
        cover_image_url: trimmedCoverImageUrl || null,
      });
      setIsEditing(false);
      onUpdate?.();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to update vinyl');
    } finally {
      setSubmitting(false);
    }
  };

  const coverPreviewUrl = coverImageUrl.trim() || vinyl.cover_image_url;

  return (
    <>
      <article className="vinyl-card">
        <div className="vinyl-cover">
          {vinyl.cover_image_url ? (
            <img
              src={vinyl.cover_image_url}
              alt={`${vinyl.title} album cover`}
              loading="lazy"
            />
          ) : (
            <div className="vinyl-placeholder" aria-hidden="true">
              <svg width="80" height="80" viewBox="0 0 80 80" fill="none">
                <circle cx="40" cy="40" r="35" stroke="currentColor" strokeWidth="2" />
                <circle cx="40" cy="40" r="8" fill="currentColor" />
                <circle cx="40" cy="40" r="20" stroke="currentColor" strokeWidth="1" opacity="0.3" />
              </svg>
            </div>
          )}
        </div>

        <div className="vinyl-info">
          <h3 className="vinyl-title">{vinyl.title}</h3>
          <p className="vinyl-artist">{vinyl.artist}</p>
          {vinyl.release_year && (
            <p className="vinyl-year" aria-label={`Released in ${vinyl.release_year}`}>
              {vinyl.release_year}
            </p>
          )}
          {vinyl.notes && !isGeneratedMetadataNote(vinyl.notes) && (
            <p className="vinyl-notes">{vinyl.notes}</p>
          )}
        </div>

        {isAdmin && (onUpdate || onDelete) && (
          <div className="card-actions">
            {onUpdate && (
              <button
                type="button"
                onClick={handleEdit}
                className="btn btn-secondary btn-sm"
                aria-label={`Edit ${vinyl.title}`}
              >
                Edit
              </button>
            )}
            {onDelete && (
              <button
                type="button"
                onClick={handleDelete}
                className="btn btn-danger btn-sm"
                aria-label={`Delete ${vinyl.title}`}
              >
                Delete
              </button>
            )}
          </div>
        )}
      </article>

      {isEditing && (
        <div className="edit-modal-backdrop" onMouseDown={handleCancel}>
          <div
            className="edit-modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby={`edit-title-heading-${vinyl.id}`}
            onMouseDown={(event) => event.stopPropagation()}
          >
            <div className="edit-modal-header">
              <div>
                <p className="edit-modal-eyebrow">Editing album</p>
                <h3 id={`edit-title-heading-${vinyl.id}`}>{vinyl.title}</h3>
              </div>
              <button
                type="button"
                onClick={handleCancel}
                disabled={submitting}
                className="edit-modal-close"
                aria-label="Close edit dialog"
              >
                ×
              </button>
            </div>

            <form className="vinyl-edit-form" onSubmit={handleSubmit}>
              {error && (
                <div className="error-message" role="alert">
                  {error}
                </div>
              )}

              <div className="edit-modal-body">
                <aside className="edit-cover-preview">
                  {coverPreviewUrl ? (
                    <img src={coverPreviewUrl} alt={`${title || vinyl.title} album cover preview`} />
                  ) : (
                    <div className="edit-cover-placeholder" aria-hidden="true">
                      No cover selected
                    </div>
                  )}
                  <ImageUpload
                    id={`edit-cover-image-${vinyl.id}`}
                    onUploadComplete={setCoverImageUrl}
                  />
                </aside>

                <div className="edit-form-fields">
                  <div className="form-row">
                    <div className="form-group">
                      <label htmlFor={`edit-artist-${vinyl.id}`} className="form-label">
                        Artist <span className="required">*</span>
                      </label>
                      <input
                        id={`edit-artist-${vinyl.id}`}
                        type="text"
                        value={artist}
                        onChange={(event) => setArtist(event.target.value)}
                        required
                        disabled={submitting}
                        className="form-input"
                      />
                    </div>

                    <div className="form-group">
                      <label htmlFor={`edit-year-${vinyl.id}`} className="form-label">
                        Release Year
                      </label>
                      <input
                        id={`edit-year-${vinyl.id}`}
                        type="number"
                        value={releaseYear}
                        onChange={(event) => setReleaseYear(event.target.value)}
                        min="1900"
                        max={new Date().getFullYear()}
                        disabled={submitting}
                        className="form-input"
                      />
                    </div>
                  </div>

                  <div className="form-group">
                    <label htmlFor={`edit-title-${vinyl.id}`} className="form-label">
                      Title <span className="required">*</span>
                    </label>
                    <input
                      id={`edit-title-${vinyl.id}`}
                      type="text"
                      value={title}
                      onChange={(event) => setTitle(event.target.value)}
                      required
                      disabled={submitting}
                      className="form-input"
                    />
                  </div>

                  <div className="form-group">
                    <label htmlFor={`edit-cover-url-${vinyl.id}`} className="form-label">
                      Cover Image URL
                    </label>
                    <input
                      id={`edit-cover-url-${vinyl.id}`}
                      type="url"
                      value={coverImageUrl}
                      onChange={(event) => setCoverImageUrl(event.target.value)}
                      disabled={submitting}
                      className="form-input"
                      placeholder="https://example.com/cover.jpg"
                    />
                  </div>

                  <div className="form-group">
                    <label htmlFor={`edit-notes-${vinyl.id}`} className="form-label">
                      Notes
                    </label>
                    <textarea
                      id={`edit-notes-${vinyl.id}`}
                      value={notes}
                      onChange={(event) => setNotes(event.target.value)}
                      disabled={submitting}
                      className="form-textarea"
                      rows={6}
                    />
                  </div>

                  <MetadataDetails vinyl={vinyl} />
                </div>
              </div>

              <div className="edit-modal-actions">
                <button type="submit" disabled={submitting} className="btn btn-primary">
                  {submitting ? 'Saving...' : 'Save changes'}
                </button>
                <button
                  type="button"
                  onClick={handleCancel}
                  disabled={submitting}
                  className="btn btn-secondary"
                >
                  Cancel
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </>
  );
}
