import { useCallback, useEffect, useState, type FormEvent } from 'react';
import type { AlbumCandidate, AlbumDetails, Vinyl } from '../types';
import {
  getVinylDetails,
  refreshVinylMetadata,
  selectVinylMetadataCandidate,
  updateVinyl,
} from '../utils/api';
import { ImageUpload } from './ImageUpload';

interface VinylCardProps {
  vinyl: Vinyl;
  isAdmin?: boolean;
  onDelete?: (id: string) => void;
  onUpdate?: () => void;
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

function metadataNeedsAdminReview(status: Vinyl['metadata_status']): boolean {
  return status !== 'complete' && status !== 'disabled';
}

function metadataAlertLabel(status: Vinyl['metadata_status']): string {
  switch (status) {
    case 'needs_choice':
      return 'Pick match';
    case 'not_found':
      return 'No metadata';
    case 'error':
      return 'Metadata error';
    case 'pending':
      return 'Pending metadata';
    case 'disabled':
      return 'Metadata disabled';
    case 'complete':
    default:
      return 'Metadata complete';
  }
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

function formatTrackLength(lengthMs?: number | null): string | null {
  if (!lengthMs) {
    return null;
  }

  const totalSeconds = Math.round(lengthMs / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${seconds.toString().padStart(2, '0')}`;
}

function DetailField({ label, value }: { label: string; value?: string | number | null }) {
  if (!value) {
    return null;
  }

  return (
    <div className="album-detail-field">
      <dt>{label}</dt>
      <dd>{value}</dd>
    </div>
  );
}

function AlbumDetailsModal({
  vinyl,
  details,
  loading,
  error,
  onClose,
}: {
  vinyl: Vinyl;
  details: AlbumDetails | null;
  loading: boolean;
  error: string | null;
  onClose: () => void;
}) {
  const displayVinyl = details?.vinyl ?? vinyl;
  const notes = displayVinyl.notes && !isGeneratedMetadataNote(displayVinyl.notes)
    ? displayVinyl.notes
    : null;

  return (
    <div className="edit-modal-backdrop" onMouseDown={onClose}>
      <div
        className="edit-modal album-detail-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby={`album-detail-heading-${vinyl.id}`}
        onMouseDown={(event) => event.stopPropagation()}
      >
        <div className="edit-modal-header">
          <div>
            <p className="edit-modal-eyebrow">Album details</p>
            <h3 id={`album-detail-heading-${vinyl.id}`}>{displayVinyl.title}</h3>
            <p className="album-detail-artist">{displayVinyl.artist}</p>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="edit-modal-close"
            aria-label="Close album details"
          >
            ×
          </button>
        </div>

        <div className="album-detail-body">
          <aside className="album-detail-cover">
            {displayVinyl.cover_image_url ? (
              <img src={displayVinyl.cover_image_url} alt={`${displayVinyl.title} album cover`} />
            ) : (
              <div className="edit-cover-placeholder" aria-hidden="true">
                No cover available
              </div>
            )}
          </aside>

          <section className="album-detail-content" aria-label="Album information">
            <dl className="album-detail-fields">
              <DetailField label="Artist" value={displayVinyl.artist} />
              <DetailField label="Released" value={details?.release_date ?? displayVinyl.release_year} />
              <DetailField label="Format" value={details?.release_format} />
              <DetailField label="Country" value={details?.release_country} />
            </dl>

            {notes && (
              <div className="album-detail-notes">
                <h4>Notes</h4>
                <p>{notes}</p>
              </div>
            )}

            {details?.source_url && (
              <p className="album-detail-source">
                <a href={details.source_url} target="_blank" rel="noreferrer">
                  View source on MusicBrainz
                </a>
              </p>
            )}

            <div className="album-tracklist">
              <h4>Songs</h4>
              {loading && <p className="album-detail-muted">Loading songs…</p>}
              {error && <p className="metadata-error">{error}</p>}
              {!loading && !error && details?.tracks.length ? (
                <ol>
                  {details.tracks.map((track, index) => {
                    const length = formatTrackLength(track.length_ms);
                    return (
                      <li key={`${track.disc_number ?? 1}-${track.number ?? index}-${track.title}`}>
                        <span className="track-number">
                          {track.disc_number && track.disc_number > 1 ? `${track.disc_number}.` : ''}
                          {track.number ?? index + 1}
                        </span>
                        <span className="track-title">
                          {track.title}
                          {track.artist && track.artist !== displayVinyl.artist && (
                            <small>{track.artist}</small>
                          )}
                        </span>
                        {length && <span className="track-length">{length}</span>}
                      </li>
                    );
                  })}
                </ol>
              ) : null}
              {!loading && !error && details && details.tracks.length === 0 && (
                <p className="album-detail-muted">
                  No song list is available yet for this album.
                  {details.tracklist_error ? ` ${details.tracklist_error}.` : ''}
                </p>
              )}
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}

function MetadataDetails({
  vinyl,
  onSelectCandidate,
}: {
  vinyl: Vinyl;
  onSelectCandidate?: (candidate: AlbumCandidate) => Promise<void>;
}) {
  const candidates = parseCandidates(vinyl.metadata_candidates);
  const [selectingId, setSelectingId] = useState<string | null>(null);
  const [selectionError, setSelectionError] = useState<string | null>(null);

  const handleSelectCandidate = async (candidate: AlbumCandidate) => {
    if (!onSelectCandidate) {
      return;
    }

    setSelectingId(candidate.id);
    setSelectionError(null);

    try {
      await onSelectCandidate(candidate);
    } catch (err) {
      setSelectionError(err instanceof Error ? err.message : 'Failed to select metadata match');
    } finally {
      setSelectingId(null);
    }
  };

  return (
    <div className={`metadata-panel ${metadataNeedsAdminReview(vinyl.metadata_status) ? 'metadata-panel-review' : ''}`}>
      <h4>Album metadata</h4>
      <div className={`metadata-badge metadata-${vinyl.metadata_status}`}>
        {metadataLabel(vinyl.metadata_status)}
      </div>
      {metadataNeedsAdminReview(vinyl.metadata_status) && (
        <p className="metadata-review-help">
          This album needs admin review before its metadata is complete.
        </p>
      )}
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
      {selectionError && <p className="metadata-error">{selectionError}</p>}
      {vinyl.metadata_status === 'needs_choice' && candidates.length > 0 && (
        <details className="metadata-candidates" open>
          <summary>Review possible album matches</summary>
          <ul>
            {candidates.map((candidate) => (
              <li key={candidate.id} className="metadata-candidate">
                {candidate.cover_image_url ? (
                  <img
                    src={candidate.cover_image_url}
                    alt={`${candidate.artist} — ${candidate.title} cover`}
                    loading="lazy"
                  />
                ) : (
                  <div className="metadata-candidate-placeholder" aria-hidden="true" />
                )}
                <div className="metadata-candidate-info">
                  <strong>{candidate.artist} — {candidate.title}</strong>
                  {candidate.release_year && <span>{candidate.release_year}</span>}
                  {candidate.disambiguation && <small>{candidate.disambiguation}</small>}
                  <a href={candidate.source_url} target="_blank" rel="noreferrer">
                    View source details
                  </a>
                </div>
                <button
                  type="button"
                  className="btn btn-primary btn-sm"
                  onClick={() => handleSelectCandidate(candidate)}
                  disabled={selectingId !== null || !onSelectCandidate}
                >
                  {selectingId === candidate.id ? 'Selecting…' : 'Select this match'}
                </button>
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
  const [isDetailsOpen, setIsDetailsOpen] = useState(false);
  const [albumDetails, setAlbumDetails] = useState<AlbumDetails | null>(null);
  const [detailsLoading, setDetailsLoading] = useState(false);
  const [detailsError, setDetailsError] = useState<string | null>(null);
  const [artist, setArtist] = useState(vinyl.artist);
  const [title, setTitle] = useState(vinyl.title);
  const [releaseYear, setReleaseYear] = useState(vinyl.release_year?.toString() ?? '');
  const [notes, setNotes] = useState(vinyl.notes ?? '');
  const [coverImageUrl, setCoverImageUrl] = useState(vinyl.cover_image_url ?? '');
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [metadataPreviewVinyl, setMetadataPreviewVinyl] = useState<Vinyl | null>(null);
  const [needsParentRefreshOnClose, setNeedsParentRefreshOnClose] = useState(false);

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
    setMetadataPreviewVinyl(null);
    setNeedsParentRefreshOnClose(false);
    setIsEditing(true);
  };

  const handleCancel = useCallback(() => {
    resetForm();
    setMetadataPreviewVinyl(null);
    setIsEditing(false);
    if (needsParentRefreshOnClose) {
      setNeedsParentRefreshOnClose(false);
      onUpdate?.();
    }
  }, [needsParentRefreshOnClose, onUpdate, resetForm]);

  const handleOpenDetails = () => {
    setAlbumDetails(null);
    setDetailsError(null);
    setIsDetailsOpen(true);
  };

  const handleCloseDetails = useCallback(() => {
    setIsDetailsOpen(false);
  }, []);

  useEffect(() => {
    if (!isDetailsOpen) {
      return undefined;
    }

    let ignore = false;
    setDetailsLoading(true);
    setDetailsError(null);

    getVinylDetails(vinyl.id)
      .then((details) => {
        if (!ignore) {
          setAlbumDetails(details);
        }
      })
      .catch((err) => {
        if (!ignore) {
          setDetailsError(err instanceof Error ? err.message : 'Failed to load album details');
        }
      })
      .finally(() => {
        if (!ignore) {
          setDetailsLoading(false);
        }
      });

    return () => {
      ignore = true;
    };
  }, [isDetailsOpen, vinyl.id]);

  useEffect(() => {
    const modalOpen = isEditing || isDetailsOpen;
    if (!modalOpen) {
      return undefined;
    }

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        if (isEditing) {
          handleCancel();
        } else {
          handleCloseDetails();
        }
      }
    };

    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    window.addEventListener('keydown', handleKeyDown);

    return () => {
      document.body.style.overflow = previousOverflow;
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [isEditing, isDetailsOpen, handleCancel, handleCloseDetails]);

  const handleSelectMetadataCandidate = useCallback(async (candidate: AlbumCandidate) => {
    await selectVinylMetadataCandidate(vinyl.id, candidate);
    setMetadataPreviewVinyl(null);
    setNeedsParentRefreshOnClose(false);
    setIsEditing(false);
    onUpdate?.();
  }, [vinyl.id, onUpdate]);

  const currentFormVinylInput = () => {
    const trimmedArtist = artist.trim();
    const trimmedTitle = title.trim();
    const trimmedReleaseYear = releaseYear.trim();
    const trimmedNotes = notes.trim();
    const trimmedCoverImageUrl = coverImageUrl.trim();

    if (!trimmedArtist || !trimmedTitle) {
      setError('Artist and title are required');
      return null;
    }

    const parsedReleaseYear = trimmedReleaseYear ? parseInt(trimmedReleaseYear, 10) : null;
    if (trimmedReleaseYear && Number.isNaN(parsedReleaseYear)) {
      setError('Release year must be a valid number');
      return null;
    }

    return {
      artist: trimmedArtist,
      title: trimmedTitle,
      release_year: parsedReleaseYear,
      notes: trimmedNotes || null,
      cover_image_url: trimmedCoverImageUrl || null,
    };
  };

  const handleSubmit = async (event: FormEvent) => {
    event.preventDefault();

    const input = currentFormVinylInput();
    if (!input) {
      return;
    }

    setSubmitting(true);
    setError(null);

    try {
      await updateVinyl(vinyl.id, input);
      setMetadataPreviewVinyl(null);
      setNeedsParentRefreshOnClose(false);
      setIsEditing(false);
      onUpdate?.();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to update vinyl');
    } finally {
      setSubmitting(false);
    }
  };

  const handleRefreshMetadata = async () => {
    const input = currentFormVinylInput();
    if (!input) {
      return;
    }

    setSubmitting(true);
    setError(null);

    try {
      await updateVinyl(vinyl.id, input);
      const refreshedVinyl = await refreshVinylMetadata(vinyl.id);
      setMetadataPreviewVinyl(refreshedVinyl);
      setNeedsParentRefreshOnClose(true);
      setReleaseYear(refreshedVinyl.release_year?.toString() ?? '');
      setNotes(refreshedVinyl.notes ?? '');
      setCoverImageUrl(refreshedVinyl.cover_image_url ?? '');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to refresh album metadata');
    } finally {
      setSubmitting(false);
    }
  };

  const metadataDetailsVinyl = metadataPreviewVinyl ?? vinyl;
  const coverPreviewUrl = coverImageUrl.trim() || metadataDetailsVinyl.cover_image_url;
  const showMetadataReview = isAdmin && metadataNeedsAdminReview(vinyl.metadata_status);

  return (
    <>
      <article className={`vinyl-card ${showMetadataReview ? 'vinyl-card-metadata-review' : ''}`}>
        {showMetadataReview && (
          <div className={`vinyl-card-alert metadata-${vinyl.metadata_status}`}>
            {metadataAlertLabel(vinyl.metadata_status)}
          </div>
        )}
        <button
          type="button"
          className="vinyl-card-details-button"
          onClick={handleOpenDetails}
          aria-label={`View details for ${vinyl.title} by ${vinyl.artist}`}
        >
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
        </button>

        {isAdmin && (onUpdate || onDelete) && (
          <div className="card-actions">
            {onUpdate && (
              <button
                type="button"
                onClick={handleEdit}
                className={`btn btn-secondary btn-sm ${showMetadataReview ? 'btn-metadata-review' : ''}`}
                aria-label={`Edit ${vinyl.title}${showMetadataReview ? ' and review metadata' : ''}`}
              >
                {showMetadataReview ? 'Review' : 'Edit'}
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

      {isDetailsOpen && (
        <AlbumDetailsModal
          vinyl={vinyl}
          details={albumDetails}
          loading={detailsLoading}
          error={detailsError}
          onClose={handleCloseDetails}
        />
      )}

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

                  <MetadataDetails vinyl={metadataDetailsVinyl} onSelectCandidate={handleSelectMetadataCandidate} />
                </div>
              </div>

              <div className="edit-modal-actions">
                <button type="submit" disabled={submitting} className="btn btn-primary">
                  {submitting ? 'Saving...' : 'Save changes'}
                </button>
                <button
                  type="button"
                  onClick={handleRefreshMetadata}
                  disabled={submitting}
                  className="btn btn-secondary"
                >
                  {submitting ? 'Refreshing...' : 'Save & refresh metadata'}
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
